use super::{Revealed, SecretHeader, SecretSource};
use crate::error::HttpError;
use crate::http::Destination;
use crate::manifest::Capabilities;

pub(crate) struct SecretAccess<'a> {
    source: &'a dyn SecretSource,
    extension: &'a str,
}

impl<'a> SecretAccess<'a> {
    pub(crate) fn new(source: &'a dyn SecretSource, extension: &'a str) -> Self {
        Self { source, extension }
    }

    pub(crate) fn reveal(
        &self,
        attached: &[SecretHeader],
        destination: &Destination,
        capabilities: &Capabilities,
    ) -> Result<Vec<Revealed>, HttpError> {
        if !destination.keeps_secrets_private() {
            return Err(HttpError::SecretOverPlainHttp(destination.listed()));
        }

        attached
            .iter()
            .map(|secret| self.reveal_one(secret, destination, capabilities))
            .collect()
    }

    fn reveal_one(
        &self,
        attached: &SecretHeader,
        destination: &Destination,
        capabilities: &Capabilities,
    ) -> Result<Revealed, HttpError> {
        let secret = attached.secret();

        if !destination.may_carry(secret, capabilities) {
            return Err(HttpError::SecretNotAllowed {
                secret: secret.to_owned(),
                destination: destination.listed(),
            });
        }

        let value = self
            .source
            .reveal(secret, self.extension)
            .map_err(|error| HttpError::SecretUnreadable(error.to_string()))?
            .ok_or_else(|| HttpError::NoSuchSecret(secret.to_owned()))?;

        Ok(Revealed::new(attached, value))
    }
}
