//! Priority-ordered provider registry and cross-provider merge logic.
//!
//! Merge rules (plan §Phase 4):
//! - localized texts: per-locale, per-field "first non-empty wins"
//! - external ids: union, deduped by `(provider, value)`
//! - images: deduped by URL, first occurrence wins
//! - scalar fields (year, runtime, kind): first non-empty wins
//!
//! Native and WASM providers are indistinguishable here because the WASM
//! host adapter implements [`MetadataProvider`] itself.

use std::sync::Arc;

use tracing::warn;

use crate::model::{
  Credit, Descriptor, LocalizedText, MediaKind, MediaMetadata, MediaQuery, SearchResult,
};
use crate::{MetadataProvider, ProviderError};

struct Entry {
  provider: Arc<dyn MetadataProvider>,
  /// Higher priority providers win field-level merges. Ties keep
  /// registration order.
  priority: i32,
}

/// An ordered set of providers. Errors from one provider never sink the
/// whole operation — they're logged and the remaining providers still
/// contribute.
#[derive(Default)]
pub struct ProviderRegistry {
  entries: Vec<Entry>,
}

impl ProviderRegistry {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn register(&mut self, provider: Arc<dyn MetadataProvider>, priority: i32) {
    self.entries.push(Entry { provider, priority });
    // Stable sort: same priority keeps registration order.
    self.entries.sort_by_key(|e| std::cmp::Reverse(e.priority));
  }

  pub fn ids(&self) -> Vec<&str> {
    self.entries.iter().map(|e| e.provider.id()).collect()
  }

  /// Search every provider; failures are logged and skipped. Results are
  /// sorted best-score-first across all providers.
  pub async fn search(&self, query: &MediaQuery) -> Vec<SearchResult> {
    let mut all = Vec::new();
    for entry in &self.entries {
      match entry.provider.search(query).await {
        Ok(results) => all.extend(results),
        Err(err) => warn!(provider = entry.provider.id(), %err, "search failed"),
      }
    }
    merge_search_results(all)
  }

  /// Fetch full metadata from a single provider by that provider's id.
  pub async fn fetch_one(
    &self,
    provider_id: &str,
    id: &str,
  ) -> Result<MediaMetadata, ProviderError> {
    let entry = self
      .entries
      .iter()
      .find(|e| e.provider.id() == provider_id)
      .ok_or_else(|| ProviderError::Other(format!("no provider registered as {provider_id}")))?;
    entry.provider.fetch(id).await
  }

  /// Fetch from every provider that has an id for this media, merged in
  /// priority order. A failing provider is logged and skipped; the call only
  /// errors when *every* requested provider failed (or nothing was asked).
  pub async fn fetch_all(
    &self,
    ids_by_provider: &[(&str, &str)],
  ) -> Result<MediaMetadata, ProviderError> {
    let mut merged_parts: Vec<MediaMetadata> = Vec::new();
    let mut last_error: Option<ProviderError> = None;

    for (provider_id, id) in ids_by_provider {
      match self.fetch_one(provider_id, id).await {
        Ok(metadata) => merged_parts.push(metadata),
        Err(err) => {
          warn!(provider = provider_id, %err, "fetch failed");
          last_error = Some(err);
        }
      }
    }

    if merged_parts.is_empty() {
      return Err(last_error.unwrap_or_else(|| {
        ProviderError::Other("no provider ids supplied for fetch".to_string())
      }));
    }
    Ok(merge_metadata(merged_parts))
  }

  /// Search a single provider by its registered id. Used by the fetcher to
  /// ask one plugin at a time so failures degrade to per-provider warnings.
  pub async fn search_one(
    &self,
    provider_id: &str,
    query: &MediaQuery,
  ) -> Result<Vec<SearchResult>, ProviderError> {
    let entry = self
      .entries
      .iter()
      .find(|e| e.provider.id() == provider_id)
      .ok_or_else(|| ProviderError::Other(format!("no provider registered as {provider_id}")))?;
    entry.provider.search(query).await
  }

  /// Registered priority of a provider id (`None` if unregistered).
  pub fn priority(&self, provider_id: &str) -> Option<i32> {
    self
      .entries
      .iter()
      .find(|e| e.provider.id() == provider_id)
      .map(|e| e.priority)
  }

  /// A registered provider's self-reported descriptor (`None` if
  /// unregistered).
  pub fn descriptor(&self, provider_id: &str) -> Option<Descriptor> {
    self
      .entries
      .iter()
      .find(|e| e.provider.id() == provider_id)
      .map(|e| e.provider.descriptor())
  }

  /// Every registered provider id except the excluded ones, in priority
  /// order — i.e. "the plugins", when the natives are named explicitly.
  pub fn ids_except(&self, excluding: &[&str]) -> Vec<String> {
    self
      .entries
      .iter()
      .map(|e| e.provider.id().to_string())
      .filter(|id| !excluding.contains(&id.as_str()))
      .collect()
  }
}

/// Merge metadata from several providers. Input order is priority order:
/// earlier entries win every field-level contest.
pub fn merge_metadata(parts: Vec<MediaMetadata>) -> MediaMetadata {
  let mut merged = MediaMetadata::default();

  for part in parts {
    if merged.release_year.is_none() {
      merged.release_year = part.release_year;
    }
    if merged.runtime_secs.is_none() {
      merged.runtime_secs = part.runtime_secs;
    }
    // `kind` defaults to Movie in the model, so the first entry defines it
    // unless an explicit Episode appears first.
    if merged.kind != part.kind && part.kind == MediaKind::Episode {
      merged.kind = MediaKind::Episode;
    }

    for text in part.localized_texts {
      merge_localized_text(&mut merged, text);
    }
    for image in part.images {
      if !merged.images.iter().any(|i| i.url == image.url) {
        merged.images.push(image);
      }
    }
    for credit in part.credits {
      if !merged.credits.iter().any(|c| same_credit(c, &credit)) {
        merged.credits.push(credit);
      }
    }
    for id in part.external_ids {
      if !merged
        .external_ids
        .iter()
        .any(|e| e.provider == id.provider && e.value == id.value)
      {
        merged.external_ids.push(id);
      }
    }
  }
  merged
}

/// First-non-empty-wins per field, per locale (compared case-insensitively).
fn merge_localized_text(merged: &mut MediaMetadata, text: LocalizedText) {
  let locale_key = text.locale.to_lowercase();
  if let Some(existing) = merged
    .localized_texts
    .iter_mut()
    .find(|t| t.locale.to_lowercase() == locale_key)
  {
    if existing.title.is_empty() {
      existing.title = text.title;
    }
    if existing.synopsis.is_empty() {
      existing.synopsis = text.synopsis;
    }
    if existing.summary.is_empty() {
      existing.summary = text.summary;
    }
  } else {
    merged.localized_texts.push(text);
  }
}

fn same_credit(a: &Credit, b: &Credit) -> bool {
  a.name == b.name
    && a.person_external_ids == b.person_external_ids
    && a.roles == b.roles
    && a.characters == b.characters
}

/// Concatenate search results from all providers, best score first. The sort
/// is stable, so equal scores keep provider priority order.
pub fn merge_search_results(mut results: Vec<SearchResult>) -> Vec<SearchResult> {
  results.sort_by(|a, b| b.score.total_cmp(&a.score));
  results
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::model::{
    CreditRole, Descriptor, ExternalId, MediaImageKind, MediaKind, RemoteImage,
  };
  use async_trait::async_trait;
  use std::sync::Mutex;

  /// Canned provider for registry tests.
  struct MockProvider {
    id: String,
    results: Vec<SearchResult>,
    metadata: Option<MediaMetadata>,
    fail_search: bool,
    fail_fetch: bool,
    calls: Mutex<Vec<String>>,
  }

  impl MockProvider {
    fn new(id: &str) -> Self {
      Self {
        id: id.to_string(),
        results: Vec::new(),
        metadata: None,
        fail_search: false,
        fail_fetch: false,
        calls: Mutex::new(Vec::new()),
      }
    }

    fn with_results(mut self, results: Vec<SearchResult>) -> Self {
      self.results = results;
      self
    }

    fn with_metadata(mut self, metadata: MediaMetadata) -> Self {
      self.metadata = Some(metadata);
      self
    }
  }

  #[async_trait]
  impl MetadataProvider for MockProvider {
    fn id(&self) -> &str {
      &self.id
    }

    async fn search(&self, _query: &MediaQuery) -> Result<Vec<SearchResult>, ProviderError> {
      self.calls.lock().unwrap().push("search".to_string());
      if self.fail_search {
        return Err(ProviderError::Network("down".to_string()));
      }
      Ok(self.results.clone())
    }

    async fn fetch(&self, _id: &str) -> Result<MediaMetadata, ProviderError> {
      self.calls.lock().unwrap().push("fetch".to_string());
      if self.fail_fetch {
        return Err(ProviderError::NotFound("nope".to_string()));
      }
      Ok(self.metadata.clone().unwrap_or_default())
    }
  }

  fn result(id: &str, score: f32) -> SearchResult {
    SearchResult {
      id: id.to_string(),
      title: id.to_string(),
      year: None,
      kind: MediaKind::Movie,
      score,
    }
  }

  fn text(locale: &str, title: &str, synopsis: &str, summary: &str) -> LocalizedText {
    LocalizedText {
      locale: locale.to_string(),
      title: title.to_string(),
      synopsis: synopsis.to_string(),
      summary: summary.to_string(),
    }
  }

  #[tokio::test]
  async fn search_sorts_by_score_and_survives_failures() {
    let primary = MockProvider::new("primary")
      .with_results(vec![result("a", 0.6), result("b", 0.9)]);
    let mut broken = MockProvider::new("broken");
    broken.fail_search = true;

    let mut registry = ProviderRegistry::new();
    registry.register(Arc::new(primary), 10);
    registry.register(Arc::new(broken), 5);

    let hits = registry.search(&MediaQuery::default()).await;
    let ids: Vec<&str> = hits.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, ["b", "a"]);
  }

  #[tokio::test]
  async fn fetch_all_merges_in_priority_order() {
    let tmdb = MockProvider::new("tmdb").with_metadata(MediaMetadata {
      release_year: Some(1999),
      runtime_secs: Some(8_100),
      localized_texts: vec![text("en", "The Matrix", "A hacker learns the truth.", "")],
      external_ids: vec![ExternalId { provider: "tmdb".into(), value: "603".into() }],
      ..Default::default()
    });
    let omdb = MockProvider::new("omdb").with_metadata(MediaMetadata {
      release_year: Some(2000), // wrong — must lose the merge
      runtime_secs: None,
      localized_texts: vec![text("en", "", "", "Neo joins the resistance.")],
      images: vec![RemoteImage {
        url: "https://m.media-amazon.com/p.jpg".into(),
        kind: MediaImageKind::Poster,
        locale: None,
      }],
      credits: vec![Credit {
        person_external_ids: vec![ExternalId { provider: "tmdb".into(), value: "1".into() }],
        name: "Keanu Reeves".into(),
        roles: vec![CreditRole::Actor],
        characters: vec!["Neo".into()],
        billing: 1,
      }],
      external_ids: vec![
        ExternalId { provider: "imdb".into(), value: "tt0133093".into() },
        ExternalId { provider: "tmdb".into(), value: "603".into() }, // dup
      ],
      ..Default::default()
    });

    let mut registry = ProviderRegistry::new();
    registry.register(Arc::new(tmdb), 10);
    registry.register(Arc::new(omdb), 5);

    let merged = registry
      .fetch_all(&[("tmdb", "603"), ("omdb", "tt0133093")])
      .await
      .expect("merge");

    assert_eq!(merged.release_year, Some(1999)); // first non-empty wins
    assert_eq!(merged.runtime_secs, Some(8_100));
    let en = merged
      .localized_texts
      .iter()
      .find(|t| t.locale == "en")
      .expect("en");
    // Field-level merge: title from the higher-priority provider, summary
    // filled in by the second.
    assert_eq!(en.title, "The Matrix");
    assert_eq!(en.synopsis, "A hacker learns the truth.");
    assert_eq!(en.summary, "Neo joins the resistance.");
    // External-id union with dedup.
    assert_eq!(merged.external_ids.len(), 2);
    assert!(merged.external_ids.iter().any(|e| e.provider == "imdb"));
    assert_eq!(merged.credits.len(), 1);
    assert_eq!(merged.images.len(), 1);
  }

  #[tokio::test]
  async fn fetch_all_errors_only_when_everything_fails() {
    let mut broken = MockProvider::new("broken");
    broken.fail_fetch = true;
    let ok = MockProvider::new("ok").with_metadata(MediaMetadata {
      release_year: Some(2021),
      ..Default::default()
    });

    let mut registry = ProviderRegistry::new();
    registry.register(Arc::new(broken), 10);
    registry.register(Arc::new(ok), 5);

    let merged = registry.fetch_all(&[("broken", "x"), ("ok", "y")]).await.expect("merge");
    assert_eq!(merged.release_year, Some(2021));

    let err = registry.fetch_all(&[("broken", "x")]).await;
    assert!(err.is_err());
    let err = registry.fetch_all(&[]).await;
    assert!(err.is_err());
  }

  #[tokio::test]
  async fn duplicate_credits_and_images_collapse() {
    let credit = Credit {
      person_external_ids: vec![],
      name: "Denis Villeneuve".into(),
      roles: vec![CreditRole::Director],
      characters: vec![],
      billing: 0,
    };
    let image = RemoteImage {
      url: "https://image.tmdb.org/p.jpg".into(),
      kind: MediaImageKind::Poster,
      locale: None,
    };
    let a = MediaMetadata {
      credits: vec![credit.clone()],
      images: vec![image.clone()],
      ..Default::default()
    };
    let b = MediaMetadata {
      credits: vec![credit],
      images: vec![image],
      ..Default::default()
    };
    let merged = merge_metadata(vec![a, b]);
    assert_eq!(merged.credits.len(), 1);
    assert_eq!(merged.images.len(), 1);
  }

  #[test]
  fn descriptor_defaults_helper_is_stable() {
    let d = Descriptor::new("tmdb", "TMDB", "0.1.0");
    assert_eq!(d.id, "tmdb");
    assert_eq!(d.model_version, crate::model::MODEL_VERSION);
  }

  #[tokio::test]
  async fn single_provider_accessors_drive_the_plugin_loop() {
    let mut registry = ProviderRegistry::new();
    let plugin = MockProvider::new("fixture")
      .with_results(vec![result("fixture:1", 0.9)]);
    let native = MockProvider::new("tmdb").with_results(vec![result("603", 1.0)]);
    registry.register(Arc::new(native), 10);
    registry.register(Arc::new(plugin), 1);

    // Priority order, natives excluded: exactly the plugin set.
    assert_eq!(registry.ids_except(&["tmdb", "omdb"]), ["fixture"]);
    assert_eq!(registry.priority("tmdb"), Some(10));
    assert_eq!(registry.priority("fixture"), Some(1));
    assert_eq!(registry.priority("nope"), None);

    let hits = registry.search_one("fixture", &MediaQuery::default()).await.unwrap();
    assert_eq!(hits[0].id, "fixture:1");

    let unknown = registry.search_one("nope", &MediaQuery::default()).await;
    assert!(matches!(unknown, Err(ProviderError::Other(_))));

    // fetch_one through the same accessor pair the fetcher uses.
    let metadata = registry.fetch_one("fixture", "fixture:1").await.unwrap();
    assert_eq!(metadata.release_year, None);
  }
}
