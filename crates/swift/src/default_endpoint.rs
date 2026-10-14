use synt_platform::Endpoint;

use crate::SyntError;

#[uniffi::export]
pub fn default_endpoint() -> Result<String, SyntError> {
    let endpoint = Endpoint::for_user().map_err(|error| SyntError::Failed {
        reason: error.to_string(),
    })?;

    Ok(endpoint.address().to_string_lossy().into_owned())
}
