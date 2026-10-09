use diesel::deserialize::{self, FromSql};
use diesel::pg::{Pg, PgValue};

#[derive(
  Debug, Clone, PartialEq, Eq, diesel::expression::AsExpression, diesel::deserialize::FromSqlRow,
)]
#[diesel(sql_type = crate::schema::sql_types::Tsvector)]
pub struct TsVector(pub String);

impl FromSql<crate::schema::sql_types::Tsvector, Pg> for TsVector {
  fn from_sql(bytes: PgValue) -> deserialize::Result<Self> {
    let text = std::str::from_utf8(bytes.as_bytes())?;
    Ok(TsVector(text.to_string()))
  }
}
