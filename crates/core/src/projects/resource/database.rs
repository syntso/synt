use std::fmt;
use std::num::NonZeroU16;
use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use synt_db::{Engine, Server, TlsMode};
use synt_platform::SecretName;
use zeroize::Zeroizing;

use super::{Address, Login, ResourceName};
use crate::CoreError;
use crate::projects::LocalPath;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DatabaseResource {
    pub(crate) engine: Engine,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) host: Option<Address>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) port: Option<NonZeroU16>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) database: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) path: Option<LocalPath>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) user: Option<Login>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) password: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) via: Option<ResourceName>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) tls: Option<TlsMode>,
}

impl DatabaseResource {
    pub(crate) fn password_name(&self) -> Result<Option<SecretName>, CoreError> {
        let Some(password) = self.password.as_deref() else {
            return Ok(None);
        };

        let name = SecretName::from_reference(password).ok_or(CoreError::PasswordNotSecret)?;

        name.map(Some)
            .map_err(|reason| CoreError::field("password", reason.into()))
    }

    pub(crate) fn tls(&self) -> TlsMode {
        match (self.tls, self.engine) {
            (Some(tls), _) => tls,
            (None, Engine::Postgres | Engine::Mysql) => TlsMode::Prefer,
            (None, Engine::Sqlite | Engine::Redis) => TlsMode::Disable,
        }
    }

    pub(crate) fn port(&self) -> u16 {
        self.port
            .map(NonZeroU16::get)
            .or_else(|| self.engine.default_port())
            .unwrap_or_default()
    }

    pub(crate) fn check(&self) -> Result<(), CoreError> {
        if self.engine == Engine::Sqlite {
            return self.check_file();
        }

        self.check_network()
    }

    pub(crate) fn server(&self, password: Option<Zeroizing<String>>) -> Result<Server, CoreError> {
        if self.engine == Engine::Sqlite {
            return Ok(Server::file(self.file()?));
        }

        let host = self.host.clone().ok_or(CoreError::MissingField("host"))?;

        Ok(
            Server::network(self.engine, String::from(host), self.port())
                .with_database(self.database.clone())
                .with_user(self.user.clone().map(String::from))
                .with_password(password)
                .with_tls(self.tls()),
        )
    }

    fn check_file(&self) -> Result<(), CoreError> {
        let network_fields = [
            ("host", self.host.is_some()),
            ("port", self.port.is_some()),
            ("database", self.database.is_some()),
            ("user", self.user.is_some()),
            ("password", self.password.is_some()),
            ("via", self.via.is_some()),
            ("tls", self.tls.is_some()),
        ];

        if let Some((field, _)) = network_fields.iter().find(|(_, set)| *set) {
            return Err(CoreError::SqliteTakesPath(field));
        }

        self.file().map(drop)
    }

    fn check_network(&self) -> Result<(), CoreError> {
        if self.path.is_some() {
            return Err(CoreError::NetworkTakesHost(self.engine));
        }

        if self.host.is_none() {
            return Err(CoreError::MissingField("host"));
        }

        self.check_engine_rules()?;
        self.password_name().map(drop)
    }

    fn check_engine_rules(&self) -> Result<(), CoreError> {
        if self.engine != Engine::Redis {
            return Ok(());
        }

        if self.tls == Some(TlsMode::Prefer) {
            return Err(CoreError::RedisPrefer);
        }

        match self.database.as_deref().map(str::parse::<u32>) {
            Some(Err(_)) => Err(CoreError::RedisDatabase),
            _ => Ok(()),
        }
    }

    fn file(&self) -> Result<PathBuf, CoreError> {
        let path = self.path.as_ref().ok_or(CoreError::MissingField("path"))?;

        path.path()
    }
}

impl fmt::Display for DatabaseResource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.engine == Engine::Sqlite {
            return write!(
                formatter,
                "sqlite {}",
                self.path
                    .as_ref()
                    .map(LocalPath::as_str)
                    .unwrap_or_default()
            );
        }

        let user = self
            .user
            .as_ref()
            .map(|user| format!("{}@", user.as_str()))
            .unwrap_or_default();
        let host = self.host.as_ref().map(Address::as_str).unwrap_or_default();
        let database = self
            .database
            .as_deref()
            .map(|database| format!("/{database}"))
            .unwrap_or_default();

        write!(
            formatter,
            "{} {user}{host}:{}{database} tls={}",
            self.engine,
            self.port(),
            self.tls()
        )
    }
}

#[cfg(test)]
mod tests {
    use synt_db::{Engine, TlsMode};

    use super::{Address, DatabaseResource, Login, ResourceName};
    use crate::projects::LocalPath;
    use crate::projects::local_path::rooted;

    fn postgres() -> DatabaseResource {
        DatabaseResource {
            engine: Engine::Postgres,
            host: Some(Address::parse("10.0.2.5").unwrap()),
            port: None,
            database: Some("orders".to_owned()),
            path: None,
            user: Some(Login::parse("app").unwrap()),
            password: Some("secret://orders-db".to_owned()),
            via: Some(ResourceName::parse("api-1").unwrap()),
            tls: Some(TlsMode::Require),
        }
    }

    fn sqlite(path: &str) -> DatabaseResource {
        DatabaseResource {
            engine: Engine::Sqlite,
            host: None,
            port: None,
            database: None,
            path: Some(LocalPath::parse(path).unwrap()),
            user: None,
            password: None,
            via: None,
            tls: None,
        }
    }

    #[test]
    fn a_full_network_database_passes_and_describes_itself() {
        let resource = postgres();

        assert!(resource.check().is_ok());
        assert_eq!(
            resource.to_string(),
            "postgres app@10.0.2.5:5432/orders tls=require"
        );
    }

    #[test]
    fn a_password_written_in_the_file_is_refused() {
        let resource = DatabaseResource {
            password: Some("hunter2".to_owned()),
            ..postgres()
        };

        let refused = resource.check().unwrap_err().to_string();

        assert!(refused.contains("secret://"), "{refused}");
        assert!(!refused.contains("hunter2"), "{refused}");
    }

    #[test]
    fn a_network_database_needs_a_host() {
        let resource = DatabaseResource {
            host: None,
            ..postgres()
        };

        assert!(resource.check().is_err());
    }

    #[test]
    fn sqlite_takes_an_absolute_path_and_nothing_of_the_network() {
        let path = rooted("/var/lib/app.sqlite");

        assert!(sqlite(&path).check().is_ok());
        assert!(LocalPath::parse("app.sqlite").is_err());

        let with_host = DatabaseResource {
            host: Some(Address::parse("x").unwrap()),
            ..sqlite(&path)
        };
        assert!(with_host.check().unwrap_err().to_string().contains("host"));
    }

    #[test]
    fn tls_defaults_follow_the_engine() {
        let unset = DatabaseResource {
            tls: None,
            ..postgres()
        };
        let redis = DatabaseResource {
            engine: Engine::Redis,
            database: None,
            ..unset.clone()
        };

        assert_eq!(unset.tls(), TlsMode::Prefer);
        assert_eq!(redis.tls(), TlsMode::Disable);
        assert_eq!(redis.port(), 6379);
    }

    #[test]
    fn redis_refuses_prefer_and_named_databases() {
        let prefer = DatabaseResource {
            engine: Engine::Redis,
            tls: Some(TlsMode::Prefer),
            database: None,
            ..postgres()
        };
        let named = DatabaseResource {
            engine: Engine::Redis,
            tls: None,
            ..postgres()
        };

        assert!(prefer.check().is_err());
        assert!(named.check().is_err());
    }
}
