use super::{Revealed, SecretHeader, SecretSource};
use crate::ExtensionError;
use crate::http::Destination;
use crate::manifest::{Capabilities, ExtensionId};

pub(crate) struct SecretAccess<'a> {
    source: &'a dyn SecretSource,
    extension: &'a ExtensionId,
}

impl<'a> SecretAccess<'a> {
    pub(crate) fn new(source: &'a dyn SecretSource, extension: &'a ExtensionId) -> Self {
        Self { source, extension }
    }

    pub(crate) fn reveal(
        &self,
        attached: &[SecretHeader],
        destination: &Destination,
        capabilities: &Capabilities,
    ) -> Result<Vec<Revealed>, ExtensionError> {
        if !destination.keeps_secrets_private() {
            return Err(ExtensionError::SecretOverPlainHttp(destination.listed()));
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
    ) -> Result<Revealed, ExtensionError> {
        let secret = attached.secret();

        let name = capabilities
            .secret_named(secret)
            .filter(|_| destination.may_carry(secret, capabilities))
            .ok_or_else(|| ExtensionError::SecretNotAllowed {
                secret: secret.to_owned(),
                destination: destination.listed(),
            })?;

        let value = self
            .source
            .reveal(name, self.extension)?
            .ok_or_else(|| ExtensionError::NoSuchSecret(secret.to_owned()))?;

        Ok(Revealed::new(attached, value))
    }
}
