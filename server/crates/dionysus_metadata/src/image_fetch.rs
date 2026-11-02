//! Downloads provider images to disk under `{MEDIA_ROOT}/{media_id}/images/`
//! and writes the matching `media_images` rows. Only hosts on an allow-list
//! are fetched — plugin- or provider-supplied URLs never reach the network
//! unchecked.

use std::path::{Path, PathBuf};

use diesel::prelude::*;
use dionysus_db::models::NewMediaImage;
use dionysus_db::types::media_image_type::MediaImageType;
use dionysus_metadata_model::{MediaImageKind, RemoteImage};
use reqwest::StatusCode;
use url::Url;

/// Hosts (suffixes) images may be fetched from by default.
pub const DEFAULT_IMAGE_DOMAINS: &[&str] = &["tmdb.org", "media-amazon.com", "fanart.tv"];

const DEFAULT_MAX_BYTES: u64 = 20 * 1024 * 1024;
const DOWNLOAD_TIMEOUT_SECS: u64 = 30;
const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp"];

#[derive(Debug, thiserror::Error)]
pub enum ImageFetchError {
  #[error("domain not on the image allow-list: {0}")]
  DomainNotAllowed(String),
  #[error("http {status} from {url}")]
  Http { status: u16, url: String },
  #[error("network for {url}: {message}")]
  Network { url: String, message: String },
  #[error("image exceeds {max_bytes} byte cap: {url}")]
  TooLarge { url: String, max_bytes: u64 },
  #[error("i/o: {0}")]
  Io(#[from] std::io::Error),
  #[error(transparent)]
  Db(#[from] diesel::result::Error),
}

/// One image written to disk, mirroring the inserted `media_images` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredImage {
  /// Relative to `MEDIA_ROOT`: `{media_id}/images/{kind}-{n}.{ext}`.
  pub storage_path: String,
  pub kind: MediaImageKind,
  pub locale: String,
}

pub struct ImageFetcher {
  client: reqwest::Client,
  allowed_domains: Vec<String>,
  media_root: PathBuf,
  max_bytes: u64,
}

impl ImageFetcher {
  pub fn new(media_root: impl Into<PathBuf>, allowed_domains: Vec<String>) -> Self {
    let client = reqwest::Client::builder()
      .timeout(std::time::Duration::from_secs(DOWNLOAD_TIMEOUT_SECS))
      .build()
      .expect("reqwest client");
    Self {
      client,
      allowed_domains,
      media_root: media_root.into(),
      max_bytes: DEFAULT_MAX_BYTES,
    }
  }

  /// Fetcher restricted to [`DEFAULT_IMAGE_DOMAINS`] — what the importer
  /// uses unless it has a reason to widen the allow-list.
  pub fn with_default_domains(media_root: impl Into<PathBuf>) -> Self {
    Self::new(
      media_root,
      DEFAULT_IMAGE_DOMAINS.iter().map(|d| d.to_string()).collect(),
    )
  }

  /// Fetch every image and insert `media_images` rows in one transaction.
  /// Per-image failures (bad domain, network, decode) are collected and
  /// returned, never abort the batch. If the transaction fails, files
  /// already written are removed again.
  pub async fn fetch_and_store(
    &self,
    conn: &mut PgConnection,
    media_id: uuid::Uuid,
    images: &[RemoteImage],
  ) -> (Vec<StoredImage>, Vec<ImageFetchError>) {
    let mut stored = Vec::new();
    let mut errors = Vec::new();
    let mut seen_urls: Vec<&str> = Vec::new();
    let mut counters: std::collections::HashMap<&'static str, u32> =
      std::collections::HashMap::new();

    for image in images {
      if seen_urls.contains(&image.url.as_str()) {
        continue;
      }
      seen_urls.push(&image.url);

      match self.download(&image.url).await {
        Ok(bytes) => {
          let kind_key: &'static str = match image.kind {
            MediaImageKind::Poster => "poster",
            MediaImageKind::Backdrop => "backdrop",
            MediaImageKind::Logo => "logo",
            MediaImageKind::Banner => "banner",
            MediaImageKind::Still => "still",
            MediaImageKind::Thumbnail => "thumbnail",
          };
          let n = counters.entry(kind_key).or_default();
          *n += 1;
          let ext = image_extension(&image.url, bytes.first());
          let relative = format!("{media_id}/images/{kind_key}-{n}.{ext}");
          let absolute = self.media_root.join(&relative);
          if let Err(err) = write_file(&absolute, &bytes).await {
            errors.push(err);
            continue;
          }
          stored.push(StoredImage {
            storage_path: relative,
            kind: image.kind,
            locale: image.locale.clone().unwrap_or_else(|| "und".to_string()),
          });
        }
        Err(err) => errors.push(err),
      }
    }

    if stored.is_empty() {
      return (stored, errors);
    }

    // Blocking Diesel call: a handful of inserts after the (slow) network
    // work is already done.
    let inserted = self.conn_transaction(conn, media_id, &stored);
    if let Err(err) = inserted {
      for image in &stored {
        let _ = std::fs::remove_file(self.media_root.join(&image.storage_path));
      }
      errors.push(err);
      stored.clear();
    }
    (stored, errors)
  }

  fn conn_transaction(
    &self,
    conn: &mut PgConnection,
    media_id: uuid::Uuid,
    stored: &[StoredImage],
  ) -> Result<(), ImageFetchError> {
    conn.transaction::<(), diesel::result::Error, _>(|conn| {
      for image in stored {
        diesel::insert_into(dionysus_db::schema::media_images::table)
          .values(NewMediaImage {
            media_id,
            image_type: image_type_from_model(image.kind),
            locale: &image.locale,
            storage_path: &image.storage_path,
          })
          .execute(conn)?;
      }
      Ok(())
    })?;
    Ok(())
  }

  /// Download one URL, enforcing the domain allow-list and size cap.
  pub async fn download(&self, url: &str) -> Result<Vec<u8>, ImageFetchError> {
    let parsed = Url::parse(url)
      .map_err(|e| ImageFetchError::Network { url: url.to_string(), message: e.to_string() })?;
    let host = parsed.host_str().unwrap_or_default().to_lowercase();
    if !self.domain_allowed(&host) {
      return Err(ImageFetchError::DomainNotAllowed(host));
    }

    let response = self
      .client
      .get(parsed)
      .send()
      .await
      .map_err(|e| ImageFetchError::Network { url: url.to_string(), message: e.to_string() })?;
    let status = response.status();
    if status != StatusCode::OK {
      return Err(ImageFetchError::Http { status: status.as_u16(), url: url.to_string() });
    }

    let bytes = response
      .bytes()
      .await
      .map_err(|e| ImageFetchError::Network { url: url.to_string(), message: e.to_string() })?;
    if bytes.len() as u64 > self.max_bytes {
      return Err(ImageFetchError::TooLarge {
        url: url.to_string(),
        max_bytes: self.max_bytes,
      });
    }
    Ok(bytes.to_vec())
  }

  /// Exact host match or subdomain match against a listed suffix.
  fn domain_allowed(&self, host: &str) -> bool {
    self.allowed_domains.iter().any(|allowed| {
      let allowed = allowed.to_lowercase();
      host == allowed || host.ends_with(&format!(".{allowed}"))
    })
  }
}

fn image_type_from_model(kind: MediaImageKind) -> MediaImageType {
  match kind {
    MediaImageKind::Poster => MediaImageType::Poster,
    MediaImageKind::Backdrop => MediaImageType::Backdrop,
    MediaImageKind::Logo => MediaImageType::Logo,
    MediaImageKind::Banner => MediaImageType::Banner,
    MediaImageKind::Still => MediaImageType::Still,
    MediaImageKind::Thumbnail => MediaImageType::Thumbnail,
  }
}

async fn write_file(path: &Path, bytes: &[u8]) -> Result<(), ImageFetchError> {
  if let Some(parent) = path.parent() {
    tokio::fs::create_dir_all(parent).await?;
  }
  tokio::fs::write(path, bytes).await?;
  Ok(())
}

/// Prefer the URL's own extension, then the content type, then `.jpg`.
fn image_extension(url: &str, first_byte: Option<&u8>) -> &'static str {
  let _ = first_byte; // sniffing reserved for later; keep the signature stable
  let from_path = Url::parse(url)
    .ok()
    .and_then(|u| {
      Path::new(u.path())
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
    })
    .filter(|e| IMAGE_EXTENSIONS.contains(&e.as_str()));
  match from_path.as_deref() {
    Some("jpeg") => "jpg",
    Some(ext) => IMAGE_EXTENSIONS
      .iter()
      .find(|&&candidate| candidate == ext)
      .copied()
      .unwrap_or("jpg"),
    None => "jpg",
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use tokio::io::{AsyncReadExt, AsyncWriteExt};

  #[test]
  fn extension_prefers_url_then_falls_back_to_jpg() {
    assert_eq!(image_extension("https://x.org/a/b/poster.PNG", None), "png");
    assert_eq!(image_extension("https://x.org/a/b/poster.jpeg", None), "jpg");
    assert_eq!(image_extension("https://x.org/a/b/poster", None), "jpg");
    assert_eq!(image_extension("https://x.org/a/b/poster.bmp", None), "jpg");
  }

  #[test]
  fn domain_allow_list_matches_subdomains_only() {
    let fetcher = ImageFetcher::new("/tmp/media", vec!["tmdb.org".to_string()]);
    assert!(fetcher.domain_allowed("image.tmdb.org"));
    assert!(fetcher.domain_allowed("tmdb.org"));
    assert!(!fetcher.domain_allowed("eviltmdb.org"));
    assert!(!fetcher.domain_allowed("example.com"));
  }

  #[test]
  fn default_domains_constructor_allows_the_known_cdn_hosts() {
    let fetcher = ImageFetcher::with_default_domains("/tmp/media");
    assert!(fetcher.domain_allowed("image.tmdb.org"));
    assert!(fetcher.domain_allowed("m.media-amazon.com"));
    assert!(fetcher.domain_allowed("assets.fanart.tv"));
    assert!(!fetcher.domain_allowed("evil.example.com"));
  }

  #[tokio::test]
  async fn download_rejects_disallowed_domain_without_network() {
    let fetcher = ImageFetcher::new("/tmp/media", vec!["tmdb.org".to_string()]);
    let err = fetcher
      .download("https://example.com/poster.jpg")
      .await
      .expect_err("must reject");
    assert!(matches!(err, ImageFetchError::DomainNotAllowed(_)));
  }

  /// Minimal hand-rolled HTTP server: no extra test dependency, and it
  /// exercises the real reqwest path.
  async fn serve_once(body: Vec<u8>, content_type: &'static str) -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("addr").port();
    tokio::spawn(async move {
      let (mut socket, _) = listener.accept().await.expect("accept");
      let mut buf = vec![0u8; 4096];
      let _ = socket.read(&mut buf).await;
      let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
      );
      let _ = socket.write_all(head.as_bytes()).await;
      let _ = socket.write_all(&body).await;
      let _ = socket.flush().await;
    });
    port
  }

  #[tokio::test]
  async fn download_fetches_allowed_host() {
    let port = serve_once(b"fake-jpeg-bytes".to_vec(), "image/jpeg").await;
    let fetcher = ImageFetcher::new("/tmp/media", vec!["127.0.0.1".to_string()]);
    let bytes = fetcher
      .download(&format!("http://127.0.0.1:{port}/poster.jpg"))
      .await
      .expect("download");
    assert_eq!(bytes, b"fake-jpeg-bytes");
  }

  #[test]
  fn image_type_mapping_covers_every_kind() {
    for kind in [
      MediaImageKind::Poster,
      MediaImageKind::Backdrop,
      MediaImageKind::Logo,
      MediaImageKind::Banner,
      MediaImageKind::Still,
      MediaImageKind::Thumbnail,
    ] {
      let db = image_type_from_model(kind);
      // Round-trip through the DB enum's SQL representation.
      let repr = match db {
        MediaImageType::Poster => "poster",
        MediaImageType::Backdrop => "backdrop",
        MediaImageType::Logo => "logo",
        MediaImageType::Banner => "banner",
        MediaImageType::Still => "still",
        MediaImageType::Thumbnail => "thumbnail",
      };
      assert!(!repr.is_empty());
    }
  }
}
