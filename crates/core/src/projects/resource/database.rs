use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use synt_db::{Engine, Server, TlsMode};
use zeroize::Zeroizing;

use crate::passwords::PasswordName;
use crate::projects::names::check_name;

const HOME_PREFIX: &str = "~/";

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
    pub(crate) fn check(&self) -> Result<(), String> {
        if self.engine == Engine::Sqlite {
            return self.check_file();
        }

        self.check_network()
    }

    pub(crate) fn password_name(&self) -> Result<Option<PasswordName>, String> {
        let Some(password) = self.password.as_deref() else {
            return Ok(None);
        };

        let name = PasswordName::from_reference(password).ok_or_else(|| {
            "password must name a secret kept by the core, such as secret://orders-db; \
             store it with password.set"
                .to_owned()
        })?;

        name.map(Some)
            .map_err(|reason| format!("password: {reason}"))
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

    pub(crate) fn server(&self, password: Option<Zeroizing<String>>) -> Result<Server, String> {
        if self.engine == Engine::Sqlite {
            return Ok(Server::file(self.file()?));
        }

        let host = self.host.clone().ok_or("host is missing")?;

        Ok(Server::network(self.engine, host, self.port())
            .with_database(self.database.clone())
            .with_user(self.user.clone())
            .with_password(password)
            .with_tls(self.tls()))
    }

    pub(crate) fn describe(&self) -> String {
        if self.engine == Engine::Sqlite {
            return format!("sqlite {}", self.path.as_deref().unwrap_or_default());
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

        format!(
            "{} {user}{host}:{}{database} tls={}",
            self.engine,
            self.port(),
            self.tls()
        )
    }

    fn check_file(&self) -> Result<(), String> {
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
            return Err(format!(
                "a sqlite database is a file: give its path, not {field}"
            ));
        }

        self.file().map(|_| ())
    }

    fn check_network(&self) -> Result<(), String> {
        if self.path.is_some() {
            return Err(format!(
                "a {} database is reached over the network: give host, not path",
                self.engine
            ));
        }

        let host = self.host.as_deref().ok_or("host is missing")?;
        if !is_plain_word(host) {
            return Err(format!("host must be a host name or address, got {host:?}"));
        }

        if self.port == Some(0) {
            return Err("port must be between 1 and 65535".to_owned());
        }

        if let Some(user) = &self.user
            && !is_plain_word(user)
        {
            return Err(format!("user must be a login name, got {user:?}"));
        }

        if let Some(via) = &self.via {
            check_name(via).map_err(|reason| format!("via: {reason}"))?;
        }

        self.check_engine_rules()?;
        self.password_name().map(|_| ())
    }

    fn check_engine_rules(&self) -> Result<(), String> {
        if self.engine != Engine::Redis {
            return Ok(());
        }

        if self.tls == Some(TlsMode::Prefer) {
            return Err(
                "redis does not negotiate tls; choose disable, require or verify-full".to_owned(),
            );
        }

        match self.database.as_deref().map(str::parse::<u32>) {
            Some(Err(_)) => Err("a redis database is a number such as 0".to_owned()),
            _ => Ok(()),
        }
    }

    fn file(&self) -> Result<PathBuf, String> {
        let path = self.path.as_deref().ok_or("path is missing")?;

        if let Some(rest) = path.strip_prefix(HOME_PREFIX) {
            let home = synt_platform::home_directory()
                .map_err(|error| format!("there is no home folder to expand ~ in: {error}"))?;
            return Ok(home.join(rest));
        }

        let file = PathBuf::from(path);

        if !file.is_absolute() {
            return Err(format!("path {path:?} must be absolute or start with ~/"));
        }

        Ok(file)
    }
}

fn is_plain_word(text: &str) -> bool {
    !text.is_empty()
        && !text.contains(|character: char| {
            character.is_whitespace() || character.is_control() || character == '@'
        })
}

#[cfg(test)]
mod tests {
    use synt_db::{Engine, TlsMode};

    use super::DatabaseResource;

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

        assert_eq!(resource.check(), Ok(()));
        assert_eq!(
            resource.describe(),
            "postgres app@10.0.2.5:5432/orders tls=require"
        );
    }

    #[test]
    fn a_password_written_in_the_file_is_refused() {
        let resource = DatabaseResource {
            password: Some("hunter2".to_owned()),
            ..postgres()
        };

        let refused = resource.check().unwrap_err();

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
        assert_eq!(sqlite("/var/lib/app.sqlite").check(), Ok(()));
        assert!(sqlite("app.sqlite").check().is_err());

        let with_host = DatabaseResource {
            host: Some("x".to_owned()),
            ..sqlite("/var/lib/app.sqlite")
        };
        assert!(with_host.check().unwrap_err().contains("host"));
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
