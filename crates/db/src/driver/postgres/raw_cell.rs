use std::error::Error;

use tokio_postgres::types::{FromSql, Type};

pub(super) struct RawCell<'a>(pub(super) Option<&'a [u8]>);

impl<'a> FromSql<'a> for RawCell<'a> {
    fn from_sql(_type: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
        Ok(Self(Some(raw)))
    }

    fn from_sql_null(_type: &Type) -> Result<Self, Box<dyn Error + Sync + Send>> {
        Ok(Self(None))
    }

    fn accepts(_type: &Type) -> bool {
        true
    }
}
