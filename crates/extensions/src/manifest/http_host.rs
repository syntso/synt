use std::borrow::Borrow;
use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::ManifestError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct HttpHost(String);

impl HttpHost {
    const LONGEST: usize = 253;

    fn is_plain_host(host: &str) -> bool {
        let allowed = |character: char| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || ".-".contains(character)
        };

        !host.is_empty()
            && host.len() <= Self::LONGEST
            && !host.starts_with(['.', '-'])
            && !host.ends_with(['.', '-'])
            && !host.contains("..")
            && host.chars().all(allowed)
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for HttpHost {
    type Error = ManifestError;

    fn try_from(text: String) -> Result<Self, ManifestError> {
        let (host, port) = match text.rsplit_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (text.as_str(), None),
        };

        let plain_port = port.is_none_or(|port| port.parse::<u16>().is_ok_and(|port| port > 0));

        if !Self::is_plain_host(host) || !plain_port {
            return Err(ManifestError::HttpHost(text));
        }

        Ok(Self(text))
    }
}

impl From<HttpHost> for String {
    fn from(value: HttpHost) -> Self {
        value.0
    }
}

impl Borrow<str> for HttpHost {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl Display for HttpHost {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::HttpHost;

    #[test]
    fn hosts_are_plain_names_with_an_optional_port() {
        for host in [
            "api.example.com",
            "127.0.0.1:8080",
            "localhost",
            "grafana-1.lan:3000",
        ] {
            assert!(HttpHost::try_from(host.to_owned()).is_ok(), "{host}");
        }

        for host in [
            "",
            "*.example.com",
            "https://example.com",
            "Example.com",
            "example.com:0",
            "example.com:99999",
            "example..com",
            "example.com/path",
            ".example.com",
        ] {
            assert!(HttpHost::try_from(host.to_owned()).is_err(), "{host}");
        }
    }
}
