use crate::driver::{Mysql, Postgres, Redis, Sqlite};

pub(crate) enum Connection {
    Postgres(Postgres),
    Mysql(Mysql),
    Sqlite(Sqlite),
    Redis(Redis),
}
