//! End-to-end tests: a real `wasm32-wasip2` component loaded by the wasmtime
//! host, exercising the full surface — descriptor handshake, MessagePack
//! search/fetch round-trips, guest errors, both capabilities (HTTP allow,
//! deny, timeout; log bridge), trace dumps, directory loading, and registry
//! indistinguishability from native providers.
//!
//! Tests that need the fixture component are skipped (like `DATABASE_URL`-
//! gated DB tests) when the `wasm32-wasip2` target is not installed; CI
//! installs it. The component itself is built by the first such test into
//! `target/plugins` — a separate target dir so the nested cargo never
//! contends with the outer test run's build lock.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use dionysus_metadata::{MetadataProvider, ProviderRegistry};
use dionysus_metadata_model::{MediaKind, MediaQuery, MODEL_VERSION};
use dionysus_metadata_wasm::{PluginOptions, WasmError, WasmLoader, WasmProvider};

// -- helpers ---------------------------------------------------------------

/// True when this toolchain can compile `wasm32-wasip2` at all.
fn wasm_target_installed() -> bool {
  let out = match std::process::Command::new("rustc")
    .args(["--print", "target-libdir", "--target", "wasm32-wasip2"])
    .output()
  {
    Ok(out) if out.status.success() => out,
    _ => return false,
  };
  std::path::Path::new(String::from_utf8_lossy(&out.stdout).trim()).is_dir()
}

/// The component for `package`, built once per test process into the
/// shared `target/plugins` dir. `None` → skip (no wasm target).
fn plugin_wasm(package: &str) -> Option<PathBuf> {
  static BUILT: Mutex<Option<std::collections::HashMap<String, Option<PathBuf>>>> =
    Mutex::new(None);
  let mut guard = BUILT.lock().unwrap();
  let memo = guard.get_or_insert_with(Default::default);
  if let Some(cached) = memo.get(package) {
    return cached.clone();
  }

  let built = if !wasm_target_installed() {
    eprintln!("wasm32-wasip2 not installed — skipping component tests");
    None
  } else {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let status = std::process::Command::new(env!("CARGO"))
      .current_dir(&workspace)
      .args(["build", "-p", package, "--target", "wasm32-wasip2", "--release"])
      .arg("--target-dir")
      .arg(workspace.join("target/plugins"))
      .status()
      .expect("spawn cargo plugin build");
    assert!(status.success(), "{package} failed to build");
    Some(workspace.join(format!("target/plugins/wasm32-wasip2/release/{package}.wasm")))
  };
  memo.insert(package.to_string(), built.clone());
  built
}

fn fixture_wasm() -> Option<PathBuf> {
  plugin_wasm("dionysus_plugin_fixture")
}

fn tvmaze_wasm() -> Option<PathBuf> {
  plugin_wasm("dionysus_plugin_tvmaze")
}

/// The fixture plugin's own version — `descriptor.version` must match the
/// crate it was built from, not the host's.
fn fixture_version() -> String {
  let manifest =
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../plugins/fixture/Cargo.toml");
  let text = std::fs::read_to_string(manifest).expect("fixture manifest");
  let line = text.lines().find(|l| l.starts_with("version = ")).expect("version line");
  line.trim_start_matches("version = ").trim().trim_matches('"').to_string()
}

/// Load the fixture component with the given options.
fn load_fixture(options: PluginOptions) -> Option<(WasmLoader, WasmProvider)> {
  let fixture = fixture_wasm()?;
  let loader = WasmLoader::new(options).expect("loader");
  let provider = loader.load_file(&fixture).expect("fixture loads");
  Some((loader, provider))
}

fn query(title: &str) -> MediaQuery {
  MediaQuery { title: title.to_string(), year: Some(2004), ..Default::default() }
}

/// One canned HTTP response, then the listener goes quiet.
fn serve_once(status: u16, body: &str) -> std::net::SocketAddr {
  use std::io::{Read, Write};
  let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
  let addr = listener.local_addr().unwrap();
  let body = body.to_string();
  std::thread::spawn(move || {
    if let Ok((mut stream, _)) = listener.accept() {
      let mut buf = [0u8; 4096];
      let _ = stream.read(&mut buf);
      let response = format!(
        "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
      );
      let _ = stream.write_all(response.as_bytes());
    }
  });
  addr
}

/// Accepts one connection, answers… eventually (never within a test
/// timeout).
fn serve_hanging() -> std::net::SocketAddr {
  use std::io::Read;
  let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
  let addr = listener.local_addr().unwrap();
  std::thread::spawn(move || {
    if let Ok((mut stream, _)) = listener.accept() {
      let mut buf = [0u8; 4096];
      let _ = stream.read(&mut buf);
      std::thread::sleep(Duration::from_secs(120));
    }
  });
  addr
}

// -- handshake & round-trip -------------------------------------------------

#[tokio::test]
async fn descriptor_round_trip() {
  let (_loader, provider) = load_fixture(PluginOptions::default()).expect("fixture");
  let descriptor = provider.descriptor();
  assert_eq!(descriptor.id, "fixture");
  assert_eq!(descriptor.name, "Fixture Plugin");
  assert_eq!(descriptor.version, fixture_version());
  assert_eq!(descriptor.model_version, MODEL_VERSION);
  assert_eq!(provider.id(), "fixture");
}

#[tokio::test]
async fn search_and_fetch_round_trip_every_model_field() {
  let (_loader, provider) = load_fixture(PluginOptions::default()).expect("fixture");

  let hits = provider.search(&query("The Fixture")).await.expect("search");
  assert_eq!(hits.len(), 2);
  assert_eq!(hits[0].id, "fixture:1");
  assert_eq!(hits[0].title, "The Fixture");
  assert_eq!(hits[0].year, Some(2004));
  assert_eq!(hits[0].kind, MediaKind::Movie);
  assert_eq!(hits[1].title, "The Fixture (alternate)");

  let metadata = provider.fetch("fixture:1").await.expect("fetch");
  assert_eq!(metadata.release_year, Some(1999));
  assert_eq!(metadata.kind, MediaKind::Episode);
  assert_eq!(metadata.runtime_secs, Some(5_400));
  let en = metadata
    .localized_texts
    .iter()
    .find(|t| t.locale == "en")
    .expect("en");
  assert_eq!(en.title, "Fixture Show");
  assert_eq!(en.synopsis, "A fixture synopsis.");
  assert_eq!(en.summary, "A fixture summary.");
  assert_eq!(metadata.images[0].kind, dionysus_metadata_model::MediaImageKind::Poster);
  assert_eq!(metadata.credits[0].name, "Ada Example");
  assert_eq!(metadata.credits[0].characters, ["Self"]);
  assert!(metadata.external_ids.iter().any(|e| e.provider == "imdb"));
}

// -- guest errors ----------------------------------------------------------

#[tokio::test]
async fn guest_errors_surface_as_provider_errors() {
  let (_loader, provider) = load_fixture(PluginOptions::default()).expect("fixture");

  let err = provider.fetch("err").await.expect_err("guest error");
  assert!(err.to_string().contains("fixture fetch failure"), "{err}");

  let err = provider.search(&query("fail-search")).await.expect_err("guest error");
  assert!(err.to_string().contains("fixture search failure"), "{err}");

  let err = provider.fetch("unknown").await.expect_err("unknown id");
  assert!(err.to_string().contains("unknown fixture id"), "{err}");

  // Still usable after guest errors — the store was not poisoned.
  let hits = provider.search(&query("still alive")).await.expect("recovered");
  assert_eq!(hits.len(), 2);
}

// -- HTTP capability -------------------------------------------------------

#[tokio::test]
async fn http_capability_reaches_allow_listed_hosts() {
  let server = serve_once(200, "hello-plugin");
  let options = PluginOptions::default().with_only_allowed_domains(["127.0.0.1"]);
  let (_loader, provider) = load_fixture(options).expect("fixture");

  let url = format!("http://{server}/probe");
  let hits = provider.search(&query(&url)).await.expect("http passthrough");
  assert_eq!(hits[0].id, "http:200");
  assert_eq!(hits[0].title, "200 hello-plugin");
}

#[tokio::test]
async fn http_capability_denies_other_hosts() {
  // Even a local server is refused when its host is not on the allow-list.
  let server = serve_once(200, "should never arrive");
  let (_loader, provider) = load_fixture(PluginOptions::default()).expect("fixture");

  let url = format!("http://{server}/probe");
  let err = provider.search(&query(&url)).await.expect_err("denied");
  assert!(err.to_string().contains("domain not allowed"), "{err}");
}

#[tokio::test]
async fn http_capability_enforces_the_timeout() {
  let server = serve_hanging();
  let mut options = PluginOptions::default().with_only_allowed_domains(["127.0.0.1"]);
  options.http_timeout = Duration::from_millis(200);
  let (_loader, provider) = load_fixture(options).expect("fixture");

  let url = format!("http://{server}/probe");
  let err = provider.search(&query(&url)).await.expect_err("timeout");
  assert!(err.to_string().contains("request failed"), "{err}");
}

// -- log capability --------------------------------------------------------

#[derive(Clone, Default)]
struct CapturedWriter(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for CapturedWriter {
  fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
    self.0.lock().unwrap().extend_from_slice(buf);
    Ok(buf.len())
  }
  fn flush(&mut self) -> std::io::Result<()> {
    Ok(())
  }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for CapturedWriter {
  type Writer = CapturedWriter;
  fn make_writer(&'a self) -> Self::Writer {
    self.clone()
  }
}

#[tokio::test]
async fn log_capability_reaches_tracing() {
  let fixture = match fixture_wasm() {
    Some(fixture) => fixture,
    None => return,
  };
  let captured = CapturedWriter::default();
  let subscriber = tracing_subscriber::fmt()
    .with_ansi(false)
    .with_writer(captured.clone())
    .finish();
  // Global default: tracing caches per-callsite interest process-wide, and
  // parallel tests hit the guest `log` callsite before this one — a
  // thread-local `set_default` would inherit that stale "disabled" cache.
  // There is no guard: a global subscriber lives for the whole test
  // binary, and only this test registers one.
  tracing::subscriber::set_global_default(subscriber)
    .expect("only this test sets a global subscriber");

  let loader = WasmLoader::new(PluginOptions::default()).expect("loader");
  let provider = loader.load_file(&fixture).expect("fixture");
  let hits = provider.search(&query("capture-me")).await.expect("search");
  assert_eq!(hits.len(), 2);

  let text = String::from_utf8_lossy(&captured.0.lock().unwrap()).to_string();
  assert!(text.contains("fixture search for \"capture-me\""), "captured:\n{text}");
}

// -- trace dumps -----------------------------------------------------------

#[tokio::test]
async fn trace_dir_receives_pretty_json_payloads() {
  let fixture = match fixture_wasm() {
    Some(fixture) => fixture,
    None => return,
  };
  let dir = std::env::temp_dir().join(format!("dionysus-wasm-trace-{}", std::process::id()));
  let _ = std::fs::remove_dir_all(&dir);

  let loader =
    WasmLoader::new(PluginOptions { trace_dir: Some(dir.clone()), ..Default::default() })
      .expect("loader");
  let provider = loader.load_file(&fixture).expect("fixture");
  provider.search(&query("trace-me")).await.expect("search");
  provider.fetch("fixture:1").await.expect("fetch");

  let mut files: Vec<String> = std::fs::read_dir(&dir)
    .expect("trace dir")
    .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
    .collect();
  files.sort();

  for expected in [
    "fixture.descriptor.out",
    "fixture.search.in",
    "fixture.search.out",
    "fixture.fetch.in",
    "fixture.fetch.out",
  ] {
    assert!(
      files.iter().any(|f| f.starts_with(expected) && f.ends_with(".json")),
      "missing {expected} in {files:?}"
    );
  }

  let search_in = files.iter().find(|f| f.starts_with("fixture.search.in")).unwrap();
  let json = std::fs::read_to_string(dir.join(search_in)).unwrap();
  assert!(json.contains("trace-me"), "{json}");
  assert!(json.contains('\n'), "trace dumps are pretty-printed");

  let fetch_out = files.iter().find(|f| f.starts_with("fixture.fetch.out")).unwrap();
  let json = std::fs::read_to_string(dir.join(fetch_out)).unwrap();
  assert!(json.contains("Fixture Show"), "{json}");

  let _ = std::fs::remove_dir_all(&dir);
}

// -- directory loading -----------------------------------------------------

#[test]
fn load_dir_keeps_good_plugins_and_reports_bad_ones() {
  let fixture = match fixture_wasm() {
    Some(fixture) => fixture,
    None => return,
  };
  let dir = std::env::temp_dir().join(format!("dionysus-wasm-dir-{}", std::process::id()));
  let _ = std::fs::remove_dir_all(&dir);
  std::fs::create_dir_all(&dir).unwrap();
  std::fs::copy(&fixture, dir.join("fixture.wasm")).unwrap();
  std::fs::write(dir.join("broken.wasm"), b"not a wasm component").unwrap();
  std::fs::write(dir.join("notes.txt"), b"ignored, not .wasm").unwrap();

  let loader = WasmLoader::new(PluginOptions::default()).expect("loader");
  let (providers, errors) = loader.load_dir(&dir);

  assert_eq!(providers.len(), 1, "{errors:?}");
  assert_eq!(providers[0].descriptor().id, "fixture");
  assert_eq!(errors.len(), 1, "{errors:?}");
  assert!(errors[0].to_string().contains("broken"), "{errors:?}");

  // A missing directory is an error entry, not a panic.
  let (none, errors) = loader.load_dir(&dir.join("does-not-exist"));
  assert!(none.is_empty());
  assert_eq!(errors.len(), 1);
  assert!(matches!(errors[0], WasmError::Io { .. }));

  let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn garbage_bytes_fail_to_compile_as_a_plugin() {
  let loader = WasmLoader::new(PluginOptions::default()).expect("loader");
  let err = loader.load_bytes("garbage", b"definitely not wasm").expect_err("must fail");
  assert!(err.to_string().contains("compile"), "{err}");
}

// -- registry indistinguishability ----------------------------------------

#[tokio::test]
async fn wasm_provider_behaves_like_a_native_in_the_registry() {
  let (_loader, provider) = load_fixture(PluginOptions::default()).expect("fixture");

  let mut registry = ProviderRegistry::new();
  registry.register(Arc::new(provider), 1);
  assert_eq!(registry.ids(), ["fixture"]);
  assert_eq!(registry.priority("fixture"), Some(1));

  let hits = registry.search(&query("The Fixture")).await;
  assert_eq!(hits[0].id, "fixture:1");

  let metadata = registry.fetch_one("fixture", "fixture:1").await.expect("fetch");
  assert_eq!(metadata.release_year, Some(1999));
}

// -- the TVMaze example plugin ---------------------------------------------

#[tokio::test]
async fn tvmaze_example_plugin_loads_with_its_own_identity() {
  let Some(path) = tvmaze_wasm() else { return };
  let loader = WasmLoader::new(PluginOptions::default()).expect("loader");
  let provider = loader.load_file(&path).expect("tvmaze loads");

  let descriptor = provider.descriptor();
  assert_eq!(descriptor.id, "tvmaze");
  assert_eq!(descriptor.name, "TVMaze");
  assert_eq!(descriptor.model_version, MODEL_VERSION);
  assert_eq!(provider.id(), "tvmaze");
}

#[tokio::test]
async fn tvmaze_search_for_movies_never_touches_the_network() {
  let Some(path) = tvmaze_wasm() else { return };
  // No domains allowed at all: if the guest tried HTTP, this would fail.
  let options = PluginOptions::default().with_only_allowed_domains(std::iter::empty::<&str>());
  let loader = WasmLoader::new(options).expect("loader");
  let provider = loader.load_file(&path).expect("tvmaze loads");

  // Movie kind short-circuits inside the guest before any HTTP.
  let movie = MediaQuery { title: "The Matrix".to_string(), ..Default::default() };
  let hits = provider.search(&movie).await.expect("tvless search");
  assert!(hits.is_empty(), "{hits:?}");
}

#[tokio::test]
async fn tvmaze_fetch_rejects_ids_it_never_handed_out() {
  let Some(path) = tvmaze_wasm() else { return };
  let options = PluginOptions::default().with_only_allowed_domains(std::iter::empty::<&str>());
  let loader = WasmLoader::new(options).expect("loader");
  let provider = loader.load_file(&path).expect("tvmaze loads");

  // Non-numeric ids are refused before any HTTP round-trip.
  let err = provider.fetch("fixture:1").await.expect_err("bad id");
  assert!(err.to_string().contains("not a show id"), "{err}");
}
