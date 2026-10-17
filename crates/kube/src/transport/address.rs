use std::io;

use http::Uri;

const HTTPS_PORT: u16 = 443;
const HTTP_PORT: u16 = 80;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Address {
    pub(super) host: String,
    pub(super) port: u16,
}

impl TryFrom<&Uri> for Address {
    type Error = io::Error;

    fn try_from(uri: &Uri) -> io::Result<Self> {
        let host = uri.host().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, format!("{uri} has no host"))
        })?;
        let host = host.trim_start_matches('[').trim_end_matches(']');

        let default_port = if uri.scheme_str() == Some("http") {
            HTTP_PORT
        } else {
            HTTPS_PORT
        };

        Ok(Self {
            host: host.to_owned(),
            port: uri.port_u16().unwrap_or(default_port),
        })
    }
}

#[cfg(test)]
mod tests {
    use http::Uri;

    use super::Address;

    fn address(text: &str) -> Address {
        Address::try_from(&text.parse::<Uri>().expect("a uri")).expect("an address")
    }

    fn at(host: &str, port: u16) -> Address {
        Address {
            host: host.to_owned(),
            port,
        }
    }

    #[test]
    fn the_port_comes_from_the_url_or_its_scheme() {
        assert_eq!(address("https://k3s:6443"), at("k3s", 6443));
        assert_eq!(
            address("https://api.example.com"),
            at("api.example.com", 443)
        );
        assert_eq!(address("http://localhost"), at("localhost", 80));
    }

    #[test]
    fn an_ipv6_host_loses_its_brackets() {
        assert_eq!(address("https://[fd00::1]:6443"), at("fd00::1", 6443));
    }
}
