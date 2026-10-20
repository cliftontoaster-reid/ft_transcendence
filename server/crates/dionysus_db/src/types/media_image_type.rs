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
#[diesel(sql_type = crate::schema::sql_types::MediaImageType)]
pub enum MediaImageType {
  Poster,
  Backdrop,
  Logo,
  Banner,
  Still,
  Thumbnail,
}

impl ToSql<crate::schema::sql_types::MediaImageType, Pg> for MediaImageType {
  fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
    match *self {
      MediaImageType::Poster => out.write_all(b"poster")?,
      MediaImageType::Backdrop => out.write_all(b"backdrop")?,
      MediaImageType::Logo => out.write_all(b"logo")?,
      MediaImageType::Banner => out.write_all(b"banner")?,
      MediaImageType::Still => out.write_all(b"still")?,
      MediaImageType::Thumbnail => out.write_all(b"thumbnail")?,
    }
    Ok(serialize::IsNull::No)
  }
}

impl FromSql<crate::schema::sql_types::MediaImageType, Pg> for MediaImageType {
  fn from_sql(bytes: PgValue) -> deserialize::Result<Self> {
    match bytes.as_bytes() {
      b"poster" => Ok(MediaImageType::Poster),
      b"backdrop" => Ok(MediaImageType::Backdrop),
      b"logo" => Ok(MediaImageType::Logo),
      b"banner" => Ok(MediaImageType::Banner),
      b"still" => Ok(MediaImageType::Still),
      b"thumbnail" => Ok(MediaImageType::Thumbnail),
      _ => Err("Unrecognized enum variant".into()),
    }
  }
}
