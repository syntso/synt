use std::sync::Arc;

use ureq::tls::{Certificate, RootCerts, TlsConfig, TlsProvider};

pub(crate) struct TrustedTls;

impl TrustedTls {
    pub(crate) fn config() -> TlsConfig {
        TlsConfig::builder()
            .provider(TlsProvider::Rustls)
            .root_certs(Self::system_roots())
            .unversioned_rustls_crypto_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .build()
    }

    fn system_roots() -> RootCerts {
        let found = rustls_native_certs::load_native_certs();

        for error in &found.errors {
            eprintln!("synt-core: extensions: a system certificate was skipped: {error}");
        }

        found
            .certs
            .iter()
            .map(|certificate| Certificate::from_der(certificate.as_ref()).to_owned())
            .into()
    }
}
