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
#[diesel(sql_type = crate::schema::sql_types::MediaKind)]
pub enum MediaKind {
  Movie,
  Episode,
}

impl ToSql<crate::schema::sql_types::MediaKind, Pg> for MediaKind {
  fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
    match *self {
      MediaKind::Movie => out.write_all(b"movie")?,
      MediaKind::Episode => out.write_all(b"episode")?,
    }
    Ok(serialize::IsNull::No)
  }
}

impl FromSql<crate::schema::sql_types::MediaKind, Pg> for MediaKind {
  fn from_sql(bytes: PgValue) -> deserialize::Result<Self> {
    match bytes.as_bytes() {
      b"movie" => Ok(MediaKind::Movie),
      b"episode" => Ok(MediaKind::Episode),
      _ => Err("Unrecognized enum variant".into()),
    }
  }
}
