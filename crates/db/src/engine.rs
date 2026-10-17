use std::fmt::{self, Display, Formatter};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlparser::dialect::{Dialect, MySqlDialect, PostgreSqlDialect, SQLiteDialect};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Engine {
    Postgres,
    Mysql,
    Sqlite,
    Redis,
}

impl Engine {
    pub fn name(self) -> &'static str {
        match self {
            Self::Postgres => "postgres",
            Self::Mysql => "mysql",
            Self::Sqlite => "sqlite",
            Self::Redis => "redis",
        }
    }

    pub fn default_port(self) -> Option<u16> {
        match self {
            Self::Postgres => Some(5432),
            Self::Mysql => Some(3306),
            Self::Redis => Some(6379),
            Self::Sqlite => None,
        }
    }

    pub fn speaks_sql(self) -> bool {
        self != Self::Redis
    }

    pub(crate) fn dialect(self) -> Option<Box<dyn Dialect>> {
        match self {
            Self::Postgres => Some(Box::new(PostgreSqlDialect {})),
            Self::Mysql => Some(Box::new(MySqlDialect {})),
            Self::Sqlite => Some(Box::new(SQLiteDialect {})),
            Self::Redis => None,
        }
    }
}

impl Display for Engine {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}
