use std::num::NonZeroU16;

use super::TargetProblem;
use crate::SshError;

const URI_PREFIX: &str = "ssh://";
const WILDCARDS: [char; 2] = ['*', '?'];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Spec {
    user: Option<String>,
    host: String,
    port: Option<NonZeroU16>,
}

impl Spec {
    pub(super) fn parse(text: &str) -> Result<Self, SshError> {
        let trimmed = text.trim();
        let address = trimmed.strip_prefix(URI_PREFIX).unwrap_or(trimmed);

        let (user, endpoint) = match address.rsplit_once('@') {
            Some((user, endpoint)) => (Some(user.to_owned()), endpoint),
            None => (None, address),
        };
        let (host, port) =
            Self::split_port(endpoint).map_err(|reason| Self::invalid(text, reason))?;

        if host.is_empty() {
            return Err(Self::invalid(text, TargetProblem::EmptyHost));
        }

        if user.as_deref() == Some("") {
            return Err(Self::invalid(text, TargetProblem::EmptyUser));
        }

        if address.contains(WILDCARDS) {
            return Err(Self::invalid(text, TargetProblem::Wildcard));
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
        self.port.map(NonZeroU16::get)
    }

    fn split_port(endpoint: &str) -> Result<(&str, Option<NonZeroU16>), TargetProblem> {
        if let Some(rest) = endpoint.strip_prefix('[') {
            return Self::split_bracketed(rest);
        }

        if endpoint.matches(':').count() != 1 {
            return Ok((endpoint, None));
        }

        let Some((host, port)) = endpoint.split_once(':') else {
            return Ok((endpoint, None));
        };

        Ok((host, Some(Self::parse_port(port)?)))
    }

    fn split_bracketed(rest: &str) -> Result<(&str, Option<NonZeroU16>), TargetProblem> {
        let (host, after) = rest.split_once(']').ok_or(TargetProblem::MissingBracket)?;

        let port = match after.strip_prefix(':') {
            Some(port) => Some(Self::parse_port(port)?),
            None if after.is_empty() => None,
            None => return Err(TargetProblem::TextAfterBracket),
        };

        Ok((host, port))
    }

    fn parse_port(text: &str) -> Result<NonZeroU16, TargetProblem> {
        text.parse().map_err(|_| TargetProblem::BadPort)
    }

    fn invalid(text: &str, reason: TargetProblem) -> SshError {
        SshError::Resolve {
            target: text.to_owned(),
            reason,
        }
    }
}
