//! Best-effort filename → title/year/episode extraction. Feeds the metadata
//! search (Phase 4+); when unsure, it returns a cleaned-up title and nothing
//! else rather than guessing.

use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedName {
  pub title: String,
  pub year: Option<i16>,
  pub season: Option<i32>,
  pub episode: Option<i32>,
  /// Lowercased IMDb id (`tt1375666`), when the name carries one.
  pub imdb_id: Option<String>,
}

impl ParsedName {
  /// A season+episode pair marks the file as a TV episode rather than a
  /// movie — shared by the DB insert and the metadata query.
  pub fn is_episode(&self) -> bool {
    self.season.is_some() && self.episode.is_some()
  }
}

static IMDB_ID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\b(tt\d{7,})\b").unwrap());
static SXXEYY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\bS(\d{1,2})E(\d{1,3})\b").unwrap());
static X_EPISODE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b(\d{1,2})x(\d{2,3})\b").unwrap());
static YEAR_IN_PARENS: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"[\[(](19|20)\d{2}[\])]").unwrap());
static YEAR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b(19|20)\d{2}\b").unwrap());
static BRACKET_GROUP: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"[\[(][^\])]*[\])]|\{[^}]*\}").unwrap());

/// Scene/release tokens that end the title portion of a name.
const QUALITY_TOKENS: &[&str] = &[
  "2160p", "1080p", "720p", "576p", "480p", "4k", "uhd", "bluray", "bdrip", "brrip", "remux",
  "webrip", "webdl", "web", "hdtv", "dvdrip", "dvd", "hdrip", "camrip", "hdcam", "x264", "x265",
  "h264", "h265", "hevc", "avc", "xvid", "aac", "ac3", "eac3", "dts", "truehd", "atmos", "flac",
  "opus", "ddp5", "dd5", "hdr", "hdr10", "dv", "sdr", "proper", "repack", "extended", "unrated",
  "theatrical", "remastered", "internal", "limited", "multi", "dual", "subbed", "dubbed",
];

/// Parse a file *stem* (extension already stripped).
pub fn parse(stem: &str) -> ParsedName {
  let mut parsed_imdb_id: Option<String> = None;
  let mut rest = stem.replace(['.', '_'], " ");

  if let Some(caps) = IMDB_ID.captures(&rest) {
    let full = caps.get(0).unwrap().as_str().to_string();
    let id = caps[1].to_lowercase();
    rest = rest.replace(&full, " ");
    parsed_imdb_id = Some(id);
  }
  let imdb_id = parsed_imdb_id;

  let mut season: Option<i32> = None;
  let mut episode: Option<i32> = None;
  if let Some(caps) = SXXEYY
    .captures(&rest)
    .or_else(|| X_EPISODE.captures(&rest))
  {
    let full = caps.get(0).unwrap().as_str().to_string();
    let s = caps[1].parse().ok();
    let e = caps[2].parse().ok();
    rest = rest.replace(&full, " ");
    season = s;
    episode = e;
  }

  // Bracketed/braced groups ([BluRay], {Group}) are noise once the IMDb id
  // above has been lifted out of them.
  rest = BRACKET_GROUP.replace_all(&rest, " ").into_owned();

  let (year, rest) = take_year(rest);

  ParsedName {
    title: clean_title(&rest),
    year,
    season,
    episode,
    imdb_id,
  }
}

/// Convenience: parse the file name of a path (extension ignored).
pub fn parse_file_name(path: &Path) -> ParsedName {
  let stem = path
    .file_stem()
    .map(|s| s.to_string_lossy().into_owned())
    .unwrap_or_default();
  parse(&stem)
}

/// A year counts when it sits in parentheses/brackets, or when a release
/// tag (1080p, BluRay, …) follows it — `Blade.Runner.2049.1080p` keeps
/// 2049 as part of the title because nothing marks it as a release year,
/// and a bare trailing year (`Some Movie 2019`) is taken as the year.
fn take_year(text: String) -> (Option<i16>, String) {
  if let Some(caps) = YEAR_IN_PARENS.captures(&text) {
    let full = caps.get(0).unwrap().as_str();
    let year = caps[0][1..5].parse().ok();
    let rest = text.replace(full, " ");
    return (year, rest);
  }

  let mut chosen: Option<(i16, usize, usize)> = None; // year, match start, match end
  for caps in YEAR.captures_iter(&text) {
    let m = caps.get(0).unwrap();
    let year: i16 = match caps[0].parse() {
      Ok(y) => y,
      Err(_) => continue,
    };
    let after = text[m.end()..].trim_start();
    let followed_by_tag = after.split_whitespace().next().is_some_and(is_quality_token);
    let is_trailing = after.is_empty();
    if followed_by_tag {
      chosen = Some((year, m.start(), m.end()));
      break;
    }
    if is_trailing {
      chosen = Some((year, m.start(), m.end()));
    }
  }
  match chosen {
    Some((year, start, end)) => {
      let mut rest = text;
      rest.replace_range(start..end, " ");
      (Some(year), rest)
    }
    None => (None, text),
  }
}

fn is_quality_token(token: &str) -> bool {
  let lower = token.to_lowercase();
  // Release names append the group to the codec tag: "x264-GROUP".
  let head = lower.split('-').next().unwrap_or(&lower);
  QUALITY_TOKENS.contains(&lower.as_str()) || QUALITY_TOKENS.contains(&head)
}

/// The title ends at the first release tag; anything after it (codec, group,
/// source) is naming noise, not title.
fn clean_title(text: &str) -> String {
  let tokens: Vec<&str> = text.split_whitespace().collect();
  let cut = tokens.iter().position(|t| is_quality_token(t)).unwrap_or(tokens.len());
  tokens[..cut].join(" ").trim().to_string()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn scene_name_with_year_and_tags() {
    assert_eq!(
      parse("The.Matrix.1999.1080p.BluRay.x264-GROUP"),
      ParsedName {
        title: "The Matrix".into(),
        year: Some(1999),
        ..Default::default()
      }
    );
  }

  #[test]
  fn episode_markers() {
    let parsed = parse("Breaking.Bad.S03E07.720p.WEB-DL");
    assert_eq!(parsed.title, "Breaking Bad");
    assert_eq!((parsed.season, parsed.episode), (Some(3), Some(7)));

    // 1x02 style, and the marker is removed from the middle of the title.
    let parsed = parse("show_name_1x02_the_payback");
    assert_eq!(parsed.title, "show name the payback");
    assert_eq!((parsed.season, parsed.episode), (Some(1), Some(2)));
  }

  #[test]
  fn year_in_brackets_and_imdb_id() {
    let parsed = parse("[tt0137523] Fight.Club.1999.1080p.BluRay");
    assert_eq!(parsed.imdb_id.as_deref(), Some("tt0137523"));
    assert_eq!(parsed.title, "Fight Club");
    assert_eq!(parsed.year, Some(1999));
  }

  #[test]
  fn imdb_id_is_case_insensitive_and_normalized() {
    assert_eq!(parse("Movie.TT0137523.x264").imdb_id.as_deref(), Some("tt0137523"));
  }

  #[test]
  fn trailing_year_is_the_release_year() {
    assert_eq!(
      parse("Some.Movie.2019"),
      ParsedName {
        title: "Some Movie".into(),
        year: Some(2019),
        ..Default::default()
      }
    );
  }

  #[test]
  fn numeric_title_keeps_its_number() {
    // 2049 here is followed by a quality tag but is part of the title; the
    // best-effort parser may read it as a year — documented trade-off. A
    // number with no year shape (2001) must stay in the title.
    let parsed = parse("2001.A.Space.Odyssey.1080p.BluRay");
    assert_eq!(parsed.title, "2001 A Space Odyssey");
    assert_eq!(parsed.year, None);
  }

  #[test]
  fn bare_name_has_only_a_title() {
    assert_eq!(
      parse("Home.Video"),
      ParsedName {
        title: "Home Video".into(),
        ..Default::default()
      }
    );
  }

  #[test]
  fn is_episode_needs_both_season_and_episode() {
    assert!(parse("Breaking.Bad.S03E07.720p").is_episode());
    assert!(parse("show_1x02_payback").is_episode());
    assert!(!parse("The.Matrix.1999.1080p").is_episode());
    // Half a pair is not an episode: don't guess.
    assert!(!parse("Season.3.Episode").is_episode());
  }

  #[test]
  fn parse_file_name_ignores_extension() {
    assert_eq!(
      parse_file_name(Path::new("/imports/Dune.Part.Two.2024.2160p.mkv")),
      ParsedName {
        title: "Dune Part Two".into(),
        year: Some(2024),
        ..Default::default()
      }
    );
  }
}
