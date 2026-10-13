use crate::SshError;

const URI_PREFIX: &str = "ssh://";
const WILDCARDS: [char; 2] = ['*', '?'];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Spec {
    pub(crate) user: Option<String>,
    pub(crate) host: String,
    pub(crate) port: Option<u16>,
}

impl Spec {
    pub(crate) fn parse(text: &str) -> Result<Self, SshError> {
        let trimmed = text.trim();
        let address = trimmed.strip_prefix(URI_PREFIX).unwrap_or(trimmed);

        let (user, endpoint) = match address.rsplit_once('@') {
            Some((user, endpoint)) => (Some(user.to_owned()), endpoint),
            None => (None, address),
        };
        let (host, port) = split_port(endpoint).map_err(|reason| invalid(text, reason))?;

        if host.is_empty() {
            return Err(invalid(text, "the host is empty"));
        }
        if user.as_deref() == Some("") {
            return Err(invalid(text, "the user is empty"));
        }
        if address.contains(WILDCARDS) {
            return Err(invalid(
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
}

fn split_port(endpoint: &str) -> Result<(&str, Option<u16>), &'static str> {
    if let Some(rest) = endpoint.strip_prefix('[') {
        let (host, after) = rest.split_once(']').ok_or("missing ']'")?;
        let port = match after.strip_prefix(':') {
            Some(port) => Some(parse_port(port)?),
            None if after.is_empty() => None,
            None => return Err("unexpected text after ']'"),
        };
        return Ok((host, port));
    }

    if endpoint.matches(':').count() != 1 {
        return Ok((endpoint, None));
    }

    let (host, port) = endpoint.split_once(':').ok_or("missing ':'")?;
    Ok((host, Some(parse_port(port)?)))
}

fn parse_port(text: &str) -> Result<u16, &'static str> {
    match text.parse::<u16>() {
        Ok(0) | Err(_) => Err("the port must be a number from 1 to 65535"),
        Ok(port) => Ok(port),
    }
}

fn invalid(text: &str, reason: &str) -> SshError {
    SshError::Resolve {
        target: text.to_owned(),
        reason: reason.to_owned(),
    }
}
