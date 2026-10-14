use ureq::http::Uri;

use crate::manifest::Capabilities;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Destination {
    pub(crate) secure: bool,
    pub(crate) host: String,
    pub(crate) port: u16,
}

impl Destination {
    pub(crate) fn parse(url: &str) -> Result<Self, String> {
        let uri: Uri = url
            .parse()
            .map_err(|error| format!("{url:?} is not a URL: {error}"))?;

        let secure = match uri.scheme_str() {
            Some("https") => true,
            Some("http") => false,
            _ => return Err(format!("{url:?} must start with https:// or http://")),
        };

        let host = uri
            .host()
            .filter(|host| !host.is_empty() && !host.starts_with('['))
            .ok_or_else(|| format!("{url:?} names no host this core may reach"))?
            .to_ascii_lowercase();

        let has_user = uri
            .authority()
            .is_some_and(|authority| authority.as_str().contains('@'));
        if has_user {
            return Err(format!(
                "{url:?} carries credentials; attach a secret instead"
            ));
        }

        let port = uri.port_u16().unwrap_or(if secure { 443 } else { 80 });

        Ok(Self { secure, host, port })
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
        self.port == if self.secure { 443 } else { 80 }
    }

    fn is_loopback(&self) -> bool {
        self.host == "localhost" || self.host.starts_with("127.")
    }
}

#[cfg(test)]
mod tests {
    use super::Destination;
    use crate::manifest::Capabilities;

    fn capabilities() -> Capabilities {
        Capabilities {
            http: vec![
                "grafana.example.com".to_owned(),
                "127.0.0.1:3000".to_owned(),
            ],
            secrets: [("token".to_owned(), vec!["grafana.example.com".to_owned()])].into(),
            ..Capabilities::default()
        }
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
