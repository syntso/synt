use std::fmt::{self, Display, Formatter};
use std::num::NonZeroU16;

use serde::{Deserialize, Serialize};

use crate::SshError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Destination {
    host: String,
    port: NonZeroU16,
}

impl Destination {
    pub fn new(host: impl Into<String>, port: u16) -> Result<Self, SshError> {
        let host = host.into();
        let named = !host.is_empty() && !host.contains(char::is_whitespace);

        let Some(port) = NonZeroU16::new(port).filter(|_| named) else {
            return Err(SshError::NotADestination(format!("{host}:{port}")));
        };

        Ok(Self { host, port })
    }

    pub fn parse(text: &str) -> Result<Self, SshError> {
        let invalid = || SshError::NotADestination(text.to_owned());

        let (host, port) = text.rsplit_once(':').ok_or_else(invalid)?;
        let host = host.trim_start_matches('[').trim_end_matches(']');
        let port = port.parse().map_err(|_| invalid())?;

        Self::new(host, port).map_err(|_| invalid())
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port.get()
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
    fn a_destination_names_a_host_and_a_port_above_zero() {
        assert!(Destination::new("orders-db", 5432).is_ok());

        assert!(Destination::new("", 5432).is_err());
        assert!(Destination::new("two words", 5432).is_err());
        assert!(Destination::new("orders-db", 0).is_err());
    }

    #[test]
    fn text_parses_into_host_and_port() {
        let plain = Destination::parse("orders-db:5432").unwrap();
        let bracketed = Destination::parse("[::1]:6379").unwrap();

        assert_eq!((plain.host(), plain.port()), ("orders-db", 5432));
        assert_eq!((bracketed.host(), bracketed.port()), ("::1", 6379));
        assert_eq!(bracketed.to_string(), "[::1]:6379");

        for text in [
            "orders-db",
            "orders-db:0",
            ":5432",
            "orders-db:http",
            "a b:1",
        ] {
            assert!(Destination::parse(text).is_err(), "{text}");
        }
    }
}
