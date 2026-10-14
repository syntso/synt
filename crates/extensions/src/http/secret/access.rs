use super::{Revealed, SecretHeader, SecretSource};
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
    ) -> Result<Vec<Revealed>, String> {
        if !destination.keeps_secrets_private() {
            return Err(format!(
                "secrets go only over https or to this computer, not to http://{}",
                destination.listed()
            ));
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
    ) -> Result<Revealed, String> {
        if !destination.may_carry(&attached.secret, capabilities) {
            return Err(format!(
                "capabilities.secrets does not let secret://{} go to {}",
                attached.secret,
                destination.listed()
            ));
        }

        let value = self
            .source
            .reveal(&attached.secret, self.extension)?
            .ok_or_else(|| format!("there is no secret secret://{}", attached.secret))?;

        Ok(Revealed::new(attached, value))
    }
}
