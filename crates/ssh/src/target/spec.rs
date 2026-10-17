use crate::SshError;

const URI_PREFIX: &str = "ssh://";
const WILDCARDS: [char; 2] = ['*', '?'];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Spec {
    user: Option<String>,
    host: String,
    port: Option<u16>,
}

impl Spec {
    pub(super) fn parse(text: &str) -> Result<Self, SshError> {
        let trimmed = text.trim();
        let address = trimmed.strip_prefix(URI_PREFIX).unwrap_or(trimmed);

        let (user, endpoint) = match address.rsplit_once('@') {
            Some((user, endpoint)) => (Some(user.to_owned()), endpoint),
            None => (None, address),
        };
        let (host, port) = Self::split_port(endpoint).map_err(|reason| Self::invalid(text, reason))?;

        if host.is_empty() {
            return Err(Self::invalid(text, "the host is empty"));
        }

        if user.as_deref() == Some("") {
            return Err(Self::invalid(text, "the user is empty"));
        }

        if address.contains(WILDCARDS) {
            return Err(Self::invalid(
                text,
                "a target names one host, so it cannot hold * or ?",
            ));
        }

        Ok(Self {
            user,
            host: host.to_owned(),
            port,
        })
    }

    pub(super) fn user(&self) -> Option<&str> {
        self.user.as_deref()
    }

    pub(super) fn host(&self) -> &str {
        &self.host
    }

    pub(super) fn port(&self) -> Option<u16> {
        self.port
    }

    fn split_port(endpoint: &str) -> Result<(&str, Option<u16>), &'static str> {
        if let Some(rest) = endpoint.strip_prefix('[') {
            return Self::split_bracketed(rest);
        }

        if endpoint.matches(':').count() != 1 {
            return Ok((endpoint, None));
        }

        let (host, port) = endpoint.split_once(':').ok_or("missing ':'")?;

        Ok((host, Some(Self::parse_port(port)?)))
    }

    fn split_bracketed(rest: &str) -> Result<(&str, Option<u16>), &'static str> {
        let (host, after) = rest.split_once(']').ok_or("missing ']'")?;

        let port = match after.strip_prefix(':') {
            Some(port) => Some(Self::parse_port(port)?),
            None if after.is_empty() => None,
            None => return Err("unexpected text after ']'"),
        };

        Ok((host, port))
    }

    fn parse_port(text: &str) -> Result<u16, &'static str> {
        match text.parse::<u16>() {
            Ok(0) | Err(_) => Err("the port must be a number from 1 to 65535"),
            Ok(port) => Ok(port),
        }
    }

    fn invalid(text: &str, reason: &'static str) -> SshError {
        SshError::Resolve {
            target: text.to_owned(),
            reason,
        }
    }
}
