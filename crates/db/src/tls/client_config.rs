use std::sync::Arc;

use rustls::pki_types::CertificateDer;
use rustls::{ClientConfig, RootCertStore};

use super::any_certificate::AnyCertificate;
use crate::{DbError, TlsMode};

pub(crate) fn client_config(mode: TlsMode) -> Result<Option<Arc<ClientConfig>>, DbError> {
    if mode == TlsMode::Disable {
        return Ok(None);
    }

    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let builder = ClientConfig::builder_with_provider(Arc::clone(&provider))
        .with_safe_default_protocol_versions()
        .map_err(|error| DbError::Tls(error.to_string()))?;

    let config = if mode.verifies() {
        builder
            .with_root_certificates(root_store()?)
            .with_no_client_auth()
    } else {
        builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(AnyCertificate::new(&provider)))
            .with_no_client_auth()
    };

    Ok(Some(Arc::new(config)))
}

pub(crate) fn system_roots() -> Vec<CertificateDer<'static>> {
    rustls_native_certs::load_native_certs().certs
}

fn root_store() -> Result<RootCertStore, DbError> {
    let mut store = RootCertStore::empty();
    let (added, _skipped) = store.add_parsable_certificates(system_roots());

    if added == 0 {
        return Err(DbError::Tls(
            "no trusted root certificates were found on this computer, so verify-full \
             cannot check the server"
                .to_owned(),
        ));
    }

    Ok(store)
}
