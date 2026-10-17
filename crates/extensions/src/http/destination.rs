use std::net::Ipv4Addr;

use ureq::http::Uri;

use crate::error::HttpError;
use crate::manifest::Capabilities;

const HTTPS_PORT: u16 = 443;
const HTTP_PORT: u16 = 80;
const LOCALHOST: &str = "localhost";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Destination {
    secure: bool,
    host: String,
    port: u16,
}

impl Destination {
    pub(crate) fn parse(url: &str) -> Result<Self, HttpError> {
        let uri: Uri = url.parse().map_err(|source| HttpError::NotUrl {
            url: url.to_owned(),
            source,
        })?;

        let secure = match uri.scheme_str() {
            Some("https") => true,
            Some("http") => false,
            _ => return Err(HttpError::Scheme(url.to_owned())),
        };

        let host = uri
            .host()
            .filter(|host| !host.is_empty() && !host.starts_with('['))
            .ok_or_else(|| HttpError::NoHost(url.to_owned()))?
            .to_ascii_lowercase();

        let has_user = uri
            .authority()
            .is_some_and(|authority| authority.as_str().contains('@'));
        if has_user {
            return Err(HttpError::Credentials(url.to_owned()));
        }

        let port = uri.port_u16().unwrap_or(Self::port_for(secure));

        Ok(Self { secure, host, port })
    }

    pub(crate) fn listed_in(url: &str, capabilities: &Capabilities) -> Result<Self, HttpError> {
        let destination = Self::parse(url)?;

        if !destination.is_listed_in(capabilities) {
            return Err(HttpError::NotListed(destination.listed()));
        }

        Ok(destination)
    }

    pub(crate) fn listed(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub(crate) fn is_listed_in(&self, capabilities: &Capabilities) -> bool {
        capabilities.reaches(&self.listed())
            || (self.default_port() && capabilities.reaches(&self.host))
    }

    pub(crate) fn may_carry(&self, secret: &str, capabilities: &Capabilities) -> bool {
        capabilities.may_attach(secret, &self.listed())
            || (self.default_port() && capabilities.may_attach(secret, &self.host))
    }

    pub(crate) fn keeps_secrets_private(&self) -> bool {
        self.secure || self.is_loopback()
    }

    fn default_port(&self) -> bool {
        self.port == Self::port_for(self.secure)
    }

    fn port_for(secure: bool) -> u16 {
        if secure { HTTPS_PORT } else { HTTP_PORT }
    }

    fn is_loopback(&self) -> bool {
        self.host == LOCALHOST
            || self
                .host
                .parse::<Ipv4Addr>()
                .is_ok_and(|address| address.is_loopback())
    }
}

#[cfg(test)]
mod tests {
    use super::Destination;
    use crate::manifest::Capabilities;

    fn capabilities() -> Capabilities {
        toml::from_str(
            r#"
            http = ["grafana.example.com", "127.0.0.1:3000"]
            secrets = { token = ["grafana.example.com"] }
            "#,
        )
        .unwrap()
    }

    #[test]
    fn a_plain_host_covers_its_default_port_only() {
        let listed = Destination::parse("https://Grafana.example.com/api/health").unwrap();
        let other_port = Destination::parse("https://grafana.example.com:8443/").unwrap();

        assert!(listed.is_listed_in(&capabilities()));
        assert!(!other_port.is_listed_in(&capabilities()));
    }

    #[test]
    fn a_host_with_a_port_covers_that_port() {
        let listed = Destination::parse("http://127.0.0.1:3000/metrics").unwrap();
        let other = Destination::parse("http://127.0.0.1:3001/metrics").unwrap();

        assert!(listed.is_listed_in(&capabilities()));
        assert!(!other.is_listed_in(&capabilities()));
        assert!(listed.keeps_secrets_private());
    }

    #[test]
    fn secrets_go_only_where_the_manifest_says() {
        let grafana = Destination::parse("https://grafana.example.com/").unwrap();
        let local = Destination::parse("http://127.0.0.1:3000/").unwrap();

        assert!(grafana.may_carry("token", &capabilities()));
        assert!(!local.may_carry("token", &capabilities()));
        assert!(!grafana.may_carry("other", &capabilities()));
    }

    #[test]
    fn only_this_computer_counts_as_private_over_plain_http() {
        for url in [
            "http://localhost:3000/",
            "http://127.0.0.1/",
            "http://127.8.9.10/",
        ] {
            assert!(
                Destination::parse(url).unwrap().keeps_secrets_private(),
                "{url}"
            );
        }

        for url in [
            "http://127.evil.com/",
            "http://127.0.0.1.evil.com/",
            "http://localhost.evil.com/",
        ] {
            assert!(
                !Destination::parse(url).unwrap().keeps_secrets_private(),
                "{url}"
            );
        }
    }

    #[test]
    fn refuses_other_schemes_credentials_and_odd_hosts() {
        for url in [
            "ftp://example.com/",
            "example.com/path",
            "https://user:pass@example.com/",
            "https://[::1]/",
            "file:///etc/passwd",
        ] {
            assert!(Destination::parse(url).is_err(), "{url}");
        }

        assert!(
            !Destination::parse("http://example.com/")
                .unwrap()
                .keeps_secrets_private()
        );
    }
}
