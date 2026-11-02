//! TVMaze wire types and the pure mapping onto the plugin model.
//!
//! Everything here is IO-free so it compiles and tests on native targets;
//! only the [`dionysus_metadata_sdk::Plugin`] implementation in `lib.rs`
//! touches the (wasm-only) HTTP/log capability.

use dionysus_metadata_sdk as sdk;
use sdk::serde_json::Value;
use serde::Deserialize;

/// One show, as returned by `singlesearch/shows` and `shows/{id}` — the two
/// responses share a shape (the search one carries `score` and, with
/// `embed=cast`, the same `_embedded`).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Show {
  pub id: i64,
  pub name: String,
  pub score: Option<f32>,
  pub premiered: Option<String>,
  pub runtime: Option<u32>,
  pub average_runtime: Option<u32>,
  pub summary: Option<String>,
  #[serde(default)]
  pub externals: Externals,
  #[serde(rename = "_embedded", default)]
  pub embedded: Option<Embedded>,
}

/// External ids are loosely typed on TVMaze's side (numbers, strings, a
/// `tv:`-prefixed TMDB id, or `null`), so they stay [`Value`]s until mapped.
#[derive(Deserialize, Default)]
pub struct Externals {
  pub imdb: Option<Value>,
  pub thetvdb: Option<Value>,
  pub tmdb: Option<Value>,
}

#[derive(Deserialize, Default)]
pub struct Embedded {
  #[serde(default)]
  pub cast: Vec<CastMember>,
}

#[derive(Deserialize)]
pub struct CastMember {
  pub person: Person,
  pub character: Option<Character>,
}

#[derive(Deserialize)]
pub struct Person {
  pub id: i64,
  pub name: String,
}

#[derive(Deserialize)]
pub struct Character {
  pub name: String,
}

pub fn descriptor() -> sdk::Descriptor {
  sdk::Descriptor::new("tvmaze", "TVMaze", env!("CARGO_PKG_VERSION"))
}

/// `singlesearch` returns at most one show; it becomes the single best hit.
pub fn to_results(show: &Show) -> Vec<sdk::SearchResult> {
  vec![sdk::SearchResult {
    id: show.id.to_string(),
    title: show.name.clone(),
    year: release_year(show),
    kind: sdk::MediaKind::Episode,
    score: show.score.unwrap_or(0.9),
  }]
}

/// TVMaze record → the shared model. The HTML summary fills *both* text
/// fields (TVMaze is the only synopsis source a TV query will get from this
/// plugin), and the show id plus every external id it reports are recorded
/// for cross-linking.
pub fn to_metadata(show: &Show) -> sdk::MediaMetadata {
  let summary = show.summary.as_deref().map(strip_html).unwrap_or_default();

  let mut external_ids = vec![sdk::ExternalId {
    provider: "tvmaze".to_string(),
    value: show.id.to_string(),
  }];
  let reported: [(&str, &Option<Value>); 3] = [
    ("imdb", &show.externals.imdb),
    ("tvdb", &show.externals.thetvdb),
    ("tmdb", &show.externals.tmdb),
  ];
  for (provider, value) in reported {
    if let Some(value) = text_value(value) {
      // TVMaze disambiguates TMDB movies from shows with a `tv:` prefix;
      // the model stores the bare id under the provider name.
      let value =
        if provider == "tmdb" { value.strip_prefix("tv:").unwrap_or(&value).to_string() } else { value };
      external_ids.push(sdk::ExternalId { provider: provider.to_string(), value });
    }
  }

  let credits = show
    .embedded
    .as_ref()
    .map(|embedded| {
      embedded
        .cast
        .iter()
        .enumerate()
        .map(|(index, member)| sdk::Credit {
          person_external_ids: vec![sdk::ExternalId {
            provider: "tvmaze".to_string(),
            value: member.person.id.to_string(),
          }],
          name: member.person.name.clone(),
          roles: vec![sdk::CreditRole::Actor],
          characters: member
            .character
            .as_ref()
            .map(|character| vec![character.name.clone()])
            .unwrap_or_default(),
          billing: index as i32,
        })
        .collect()
    })
    .unwrap_or_default();

  sdk::MediaMetadata {
    release_year: release_year(show),
    runtime_secs: show.average_runtime.or(show.runtime).map(|minutes| (minutes * 60) as i32),
    kind: sdk::MediaKind::Episode,
    localized_texts: vec![sdk::LocalizedText {
      locale: "en".to_string(),
      title: show.name.clone(),
      synopsis: summary.clone(),
      summary,
    }],
    credits,
    external_ids,
    ..Default::default()
  }
}

/// `"2008-01-20" → Some(2008)`.
pub fn release_year(show: &Show) -> Option<i16> {
  show.premiered.as_deref()?.get(0..4)?.parse().ok()
}

fn text_value(value: &Option<Value>) -> Option<String> {
  let value = value.as_ref()?;
  value.as_str().map(str::to_string).or_else(|| value.as_i64().map(|n| n.to_string()))
}

pub fn search_url(title: &str) -> String {
  format!(
    "https://api.tvmaze.com/singlesearch/shows?q={}&embed=cast",
    encode_component(title)
  )
}

pub fn show_url(id: &str) -> String {
  format!("https://api.tvmaze.com/shows/{id}?embed=cast")
}

/// RFC 3986 query-component encoding; space becomes `+` as search forms do.
pub fn encode_component(input: &str) -> String {
  let mut out = String::with_capacity(input.len());
  for byte in input.bytes() {
    match byte {
      b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
        out.push(byte as char)
      }
      b' ' => out.push('+'),
      _ => out.push_str(&format!("%{byte:02X}")),
    }
  }
  out
}

/// TVMaze summaries are HTML fragments (`<p>…</p>` plus a few entities);
/// both DB fields want plain text. Closing tags also leave a space so
/// adjacent paragraphs don't glue together.
pub fn strip_html(input: &str) -> String {
  let mut out = String::with_capacity(input.len());
  let mut in_tag = false;
  for c in input.chars() {
    match c {
      '<' => in_tag = true,
      '>' => {
        in_tag = false;
        out.push(' ');
      }
      _ if !in_tag => out.push(c),
      _ => {}
    }
  }
  let text = out
    .replace("&lt;", "<")
    .replace("&gt;", ">")
    .replace("&quot;", "\"")
    .replace("&#39;", "'")
    .replace("&apos;", "'")
    .replace("&nbsp;", " ");
  // `&amp;` last — earlier it would double-decode `&amp;lt;` into `<`.
  let text = text.replace("&amp;", "&");
  text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
  use super::*;

  /// Real-shaped `singlesearch/shows?…&embed=cast` payload.
  const SHOW: &str = r#"{
    "id": 169,
    "name": "Breaking Bad",
    "type": "Scripted",
    "status": "Ended",
    "runtime": 60,
    "averageRuntime": 45,
    "premiered": "2008-01-20",
    "score": 0.8300888,
    "summary": "<p>A chemistry teacher turns to crime &amp; drugs.</p><p>He is very &lt;careful&gt;.</p>",
    "externals": {"tvrage": null, "thetvdb": 81189, "imdb": "tt0903747", "tmdb": "tv:1396"},
    "_embedded": {"cast": [
      {"person": {"id": 1, "name": "Bryan Cranston"}, "character": {"name": "Walter White"}},
      {"person": {"id": 2, "name": "Aaron Paul"}, "character": null}
    ]}
  }"#;

  fn parsed() -> Show {
    sdk::serde_json::from_str(SHOW).expect("show json")
  }

  #[test]
  fn parses_a_real_shaped_response() {
    let show = parsed();
    assert_eq!(show.id, 169);
    assert_eq!(show.name, "Breaking Bad");
    assert_eq!(show.average_runtime, Some(45));
    let embedded = show.embedded.as_ref().expect("embedded");
    assert_eq!(embedded.cast.len(), 2);
    assert_eq!(embedded.cast[0].person.name, "Bryan Cranston");
    assert!(embedded.cast[1].character.is_none());
    // A null external (`tvrage`) is absent, not an error.
    assert!(text_value(&show.externals.thetvdb).as_deref() == Some("81189"));
  }

  #[test]
  fn search_hit_is_episode_scoped() {
    let hits = to_results(&parsed());
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, "169");
    assert_eq!(hits[0].kind, sdk::MediaKind::Episode);
    assert_eq!(hits[0].year, Some(2008));
    assert_eq!(hits[0].score, 0.8300888);
  }

  #[test]
  fn metadata_maps_runtime_ids_credits_and_summary() {
    let metadata = to_metadata(&parsed());

    assert_eq!(metadata.release_year, Some(2008));
    // averageRuntime (45) wins over the per-episode runtime (60).
    assert_eq!(metadata.runtime_secs, Some(45 * 60));
    assert_eq!(metadata.kind, sdk::MediaKind::Episode);

    let en = &metadata.localized_texts[0];
    assert_eq!(en.title, "Breaking Bad");
    assert_eq!(en.synopsis, en.summary, "one summary feeds both fields");
    assert!(en.synopsis.contains("crime & drugs"), "{}", en.synopsis);
    // Markup is gone (escaped entities legitimately decode back to `<`…`>`).
    assert!(!en.synopsis.contains("<p>"), "{}", en.synopsis);
    assert!(!en.synopsis.contains("</"), "{}", en.synopsis);

    let providers: Vec<&str> =
      metadata.external_ids.iter().map(|id| id.provider.as_str()).collect();
    assert_eq!(providers, ["tvmaze", "imdb", "tvdb", "tmdb"]);
    let tmdb = metadata
      .external_ids
      .iter()
      .find(|id| id.provider == "tmdb")
      .expect("tmdb id");
    assert_eq!(tmdb.value, "1396", "`tv:` prefix stripped");

    assert_eq!(metadata.credits.len(), 2);
    assert_eq!(metadata.credits[0].billing, 0);
    assert_eq!(metadata.credits[0].characters, ["Walter White"]);
    assert!(metadata.credits[1].characters.is_empty());
    assert_eq!(metadata.credits[1].person_external_ids[0].value, "2");
  }

  #[test]
  fn html_is_stripped_without_double_decoding() {
    assert_eq!(strip_html("<p>hello</p>"), "hello");
    assert_eq!(strip_html("<p>a</p><p>b</p>"), "a b");
    assert_eq!(strip_html("a &amp; b"), "a & b");
    // "&amp;lt;" is literally the text "&lt;" — decode entities once.
    assert_eq!(strip_html("&amp;lt;"), "&lt;");
    assert_eq!(strip_html("&#39;quoted&#39;"), "'quoted'");
    assert_eq!(strip_html("no tags"), "no tags");
  }

  #[test]
  fn query_component_encoding_is_url_safe() {
    assert_eq!(encode_component("The Wire & Stuff"), "The+Wire+%26+Stuff");
    assert_eq!(encode_component("24"), "24");
    assert_eq!(encode_component("Y tu mamá también"), "Y+tu+mam%C3%A1+tambi%C3%A9n");
    assert_eq!(
      search_url("The Wire & Stuff"),
      "https://api.tvmaze.com/singlesearch/shows?q=The+Wire+%26+Stuff&embed=cast"
    );
    assert_eq!(show_url("169"), "https://api.tvmaze.com/shows/169?embed=cast");
  }

  #[test]
  fn missing_dates_and_runtimes_stay_absent() {
    let mut show = parsed();
    show.premiered = None;
    show.runtime = None;
    show.average_runtime = None;
    assert_eq!(release_year(&show), None);
    let metadata = to_metadata(&show);
    assert_eq!(metadata.release_year, None);
    assert_eq!(metadata.runtime_secs, None);
    // Even without summary/externals the identity survives.
    assert!(metadata.external_ids.iter().any(|id| id.provider == "tvmaze"));
  }

  #[test]
  fn descriptor_is_the_stable_identity() {
    let descriptor = descriptor();
    assert_eq!(descriptor.id, "tvmaze");
    assert_eq!(descriptor.model_version, sdk::MODEL_VERSION);
  }
}
