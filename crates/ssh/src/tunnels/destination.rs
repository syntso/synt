use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::SshError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Destination {
    host: String,
    port: u16,
}

impl Destination {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
        }
    }

    pub fn parse(text: &str) -> Result<Self, SshError> {
        let invalid = || SshError::NotARemote(text.to_owned());

        let (host, port) = text.rsplit_once(':').ok_or_else(invalid)?;
        let host = host.trim_start_matches('[').trim_end_matches(']');
        let port: u16 = port.parse().map_err(|_| invalid())?;

        if host.is_empty() || port == 0 || host.contains(char::is_whitespace) {
            return Err(invalid());
        }

        Ok(Self::new(host, port))
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

impl TryFrom<String> for Destination {
    type Error = SshError;

    fn try_from(text: String) -> Result<Self, SshError> {
        Self::parse(&text)
    }
}

impl From<Destination> for String {
    fn from(destination: Destination) -> Self {
        destination.to_string()
    }
}

impl Display for Destination {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        if self.host.contains(':') {
            return write!(formatter, "[{}]:{}", self.host, self.port);
        }

        write!(formatter, "{}:{}", self.host, self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::Destination;

    #[test]
    fn a_remote_is_host_and_port() {
        let parsed = Destination::parse("orders-db:5432").unwrap();

        assert_eq!(parsed, Destination::new("orders-db", 5432));
        assert_eq!(
            Destination::parse("[::1]:6379").unwrap().to_string(),
            "[::1]:6379"
        );
    }

    #[test]
    fn anything_else_is_refused() {
        for text in ["orders-db", "orders-db:0", ":5432", "a b:1", "db:port"] {
            assert!(Destination::parse(text).is_err(), "{text:?}");
        }
    }
}
