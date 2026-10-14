use rustls::RootCertStore;
use rustls::pki_types::CertificateDer;

use crate::DbError;

pub(crate) struct SystemRoots {
    certificates: Vec<CertificateDer<'static>>,
}

impl SystemRoots {
    pub(crate) fn load() -> Self {
        Self {
            certificates: rustls_native_certs::load_native_certs().certs,
        }
    }

    pub(crate) fn into_der(self) -> impl Iterator<Item = Vec<u8>> {
        self.certificates
            .into_iter()
            .map(|certificate| certificate.to_vec())
    }

    pub(crate) fn into_store(self) -> Result<RootCertStore, DbError> {
        let mut store = RootCertStore::empty();
        let (added, _skipped) = store.add_parsable_certificates(self.certificates);

        if added == 0 {
            return Err(DbError::Tls(
                "no trusted root certificates were found on this computer, so verify-full \
                 cannot check the server"
                    .to_owned(),
            ));
        }

        Ok(store)
    }
}
