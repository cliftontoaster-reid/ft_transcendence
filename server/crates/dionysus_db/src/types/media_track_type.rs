use diesel::deserialize::{self, FromSql};
use diesel::pg::{Pg, PgValue};
use diesel::serialize::{self, Output, ToSql};
use std::io::Write;

#[derive(
  Debug,
  Clone,
  Copy,
  PartialEq,
  Eq,
  diesel::expression::AsExpression,
  diesel::deserialize::FromSqlRow,
)]
#[diesel(sql_type = crate::schema::sql_types::MediaTrackType)]
pub enum MediaTrackType {
  Video,
  Audio,
  Subtitles,
}

impl ToSql<crate::schema::sql_types::MediaTrackType, Pg> for MediaTrackType {
  fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
    match *self {
      MediaTrackType::Video => out.write_all(b"video")?,
      MediaTrackType::Audio => out.write_all(b"audio")?,
      MediaTrackType::Subtitles => out.write_all(b"subtitles")?,
    }
    Ok(serialize::IsNull::No)
  }
}

impl FromSql<crate::schema::sql_types::MediaTrackType, Pg> for MediaTrackType {
  fn from_sql(bytes: PgValue) -> deserialize::Result<Self> {
    match bytes.as_bytes() {
      b"video" => Ok(MediaTrackType::Video),
      b"audio" => Ok(MediaTrackType::Audio),
      b"subtitles" => Ok(MediaTrackType::Subtitles),
      _ => Err("Unrecognized enum variant".into()),
    }
  }
}
