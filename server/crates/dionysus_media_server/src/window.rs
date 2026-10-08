/// Map a DASH segment `number` to a `[start, end)` media window.
///
/// Segment 0 covers `[0, duration)`, segment 1 covers `[duration, 2*duration)`, etc.
pub fn window(
  number: u32,
  duration_secs: u64,
) -> (
  Option<prost_types::Timestamp>,
  Option<prost_types::Timestamp>,
) {
  let start = number as u64 * duration_secs;
  let end = start + duration_secs;
  (
    Some(prost_types::Timestamp {
      seconds: start as i64,
      nanos: 0,
    }),
    Some(prost_types::Timestamp {
      seconds: end as i64,
      nanos: 0,
    }),
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn segment_numbers_map_to_windows() {
    let (s, e) = window(0, 4);
    assert_eq!(s.unwrap().seconds, 0);
    assert_eq!(e.unwrap().seconds, 4);
    let (s, e) = window(3, 4);
    assert_eq!(s.unwrap().seconds, 12);
    assert_eq!(e.unwrap().seconds, 16);
  }
}
