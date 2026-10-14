use std::fmt;
use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use synt_db::{Engine, Server, TlsMode};
use zeroize::Zeroizing;

use super::{ResourceName, ResourceSpec};
use crate::CoreError;
use crate::projects::LocalPath;
use crate::secrets::SecretName;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DatabaseResource {
    pub engine: Engine,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub via: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls: Option<TlsMode>,
}

impl DatabaseResource {
    pub(crate) fn password_name(&self) -> Result<Option<SecretName>, CoreError> {
        let Some(password) = self.password.as_deref() else {
            return Ok(None);
        };

        let name = SecretName::from_reference(password).ok_or(CoreError::PasswordNotSecret)?;

        name.map(Some)
            .map_err(|reason| CoreError::field("password", reason))
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
            .or_else(|| self.engine.default_port())
            .unwrap_or_default()
    }

    pub(crate) fn server(&self, password: Option<Zeroizing<String>>) -> Result<Server, CoreError> {
        if self.engine == Engine::Sqlite {
            return Ok(Server::file(self.file()?));
        }

        let host = self.host.clone().ok_or(CoreError::MissingField("host"))?;

        Ok(Server::network(self.engine, host, self.port())
            .with_database(self.database.clone())
            .with_user(self.user.clone())
            .with_password(password)
            .with_tls(self.tls()))
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

        let host = self
            .host
            .as_deref()
            .ok_or(CoreError::MissingField("host"))?;

        if !Self::is_plain_word(host) {
            return Err(CoreError::NotAnAddress {
                field: "host",
                value: host.to_owned(),
            });
        }

        if self.port == Some(0) {
            return Err(CoreError::PortZero("port"));
        }

        if let Some(user) = &self.user
            && !Self::is_plain_word(user)
        {
            return Err(CoreError::NotALogin(user.clone()));
        }

        if let Some(via) = &self.via {
            ResourceName::check(via, "via")?;
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
        let path = self
            .path
            .as_deref()
            .ok_or(CoreError::MissingField("path"))?;

        let file = LocalPath::parse(path).map_err(|reason| CoreError::field("path", reason))?;

        Ok(file.into_path())
    }

    fn is_plain_word(text: &str) -> bool {
        !text.is_empty()
            && !text.contains(|character: char| {
                character.is_whitespace() || character.is_control() || character == '@'
            })
    }
}

impl ResourceSpec for DatabaseResource {
    fn kind(&self) -> &'static str {
        "database"
    }

    fn check(&self) -> Result<(), CoreError> {
        if self.engine == Engine::Sqlite {
            return self.check_file();
        }

        self.check_network()
    }
}

impl fmt::Display for DatabaseResource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.engine == Engine::Sqlite {
            return write!(
                formatter,
                "sqlite {}",
                self.path.as_deref().unwrap_or_default()
            );
        }

        let user = self
            .user
            .as_deref()
            .map(|user| format!("{user}@"))
            .unwrap_or_default();
        let host = self.host.as_deref().unwrap_or_default();
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

    use super::{DatabaseResource, ResourceSpec};

    fn postgres() -> DatabaseResource {
        DatabaseResource {
            engine: Engine::Postgres,
            host: Some("10.0.2.5".to_owned()),
            port: None,
            database: Some("orders".to_owned()),
            path: None,
            user: Some("app".to_owned()),
            password: Some("secret://orders-db".to_owned()),
            via: Some("api-1".to_owned()),
            tls: Some(TlsMode::Require),
        }
    }

    fn sqlite(path: &str) -> DatabaseResource {
        DatabaseResource {
            engine: Engine::Sqlite,
            host: None,
            port: None,
            database: None,
            path: Some(path.to_owned()),
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
        assert!(sqlite("/var/lib/app.sqlite").check().is_ok());
        assert!(sqlite("app.sqlite").check().is_err());

        let with_host = DatabaseResource {
            host: Some("x".to_owned()),
            ..sqlite("/var/lib/app.sqlite")
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
