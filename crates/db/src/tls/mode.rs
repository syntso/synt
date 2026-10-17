use std::fmt::{self, Display, Formatter};
use std::sync::Arc;

use rustls::ClientConfig;
use rustls::crypto::ring;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::any_certificate::AnyCertificate;
use super::roots::SystemRoots;
use crate::DbError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum TlsMode {
    Disable,
    Prefer,
    Require,
    VerifyFull,
}

impl TlsMode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Disable => "disable",
            Self::Prefer => "prefer",
            Self::Require => "require",
            Self::VerifyFull => "verify-full",
        }
    }

    pub(crate) fn verifies(self) -> bool {
        self == Self::VerifyFull
    }

    pub(crate) fn client_config(self) -> Result<Option<Arc<ClientConfig>>, DbError> {
        if self == Self::Disable {
            return Ok(None);
        }

        let provider = Arc::new(ring::default_provider());
        let builder = ClientConfig::builder_with_provider(Arc::clone(&provider))
            .with_safe_default_protocol_versions()?;

        let config = if self.verifies() {
            builder
                .with_root_certificates(SystemRoots::load().into_store()?)
                .with_no_client_auth()
        } else {
            builder
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(AnyCertificate::new(&provider)))
                .with_no_client_auth()
        };

        Ok(Some(Arc::new(config)))
    }
}

impl Display for TlsMode {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}
