use tokio_postgres::types::FromSql;
use tokio_postgres::{Error, Row};

pub(super) trait FromRow: Sized {
    fn from_row(row: &Row) -> Result<Self, Error>;
}

macro_rules! tuple_from_row {
    ($($field:ident $index:tt),+) => {
        impl<$($field),+> FromRow for ($($field,)+)
        where
            $($field: for<'a> FromSql<'a>),+
        {
            fn from_row(row: &Row) -> Result<Self, Error> {
                Ok(($(row.try_get::<_, $field>($index)?,)+))
            }
        }
    };
}

tuple_from_row!(A 0);
tuple_from_row!(A 0, B 1, C 2);
tuple_from_row!(A 0, B 1, C 2, D 3, E 4, F 5);
tuple_from_row!(A 0, B 1, C 2, D 3, E 4, F 5, G 6);
