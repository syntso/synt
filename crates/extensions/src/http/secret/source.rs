use synt_platform::SecretName;
use zeroize::Zeroizing;

use crate::ExtensionError;
use crate::manifest::ExtensionId;

pub trait SecretSource: Send + Sync + 'static {
    fn reveal(
        &self,
        name: &SecretName,
        extension: &ExtensionId,
    ) -> Result<Option<Zeroizing<Vec<u8>>>, ExtensionError>;
}
