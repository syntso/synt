use std::sync::{Arc, OnceLock};

use russh::Names;
use russh::client::{Handler, Session};
use russh::keys::{HashAlg, PublicKey, PublicKeyOrCertificate};

use super::Algorithms;
use crate::known_hosts::{KnownHosts, Verdict};
use crate::{SshError, Target};

pub(crate) struct ClientHandler {
    host: String,
    port: u16,
    address: String,
    known_hosts: Arc<KnownHosts>,
    negotiated: Arc<OnceLock<Algorithms>>,
}

impl ClientHandler {
    pub(super) fn new(
        hop: &Target,
        known_hosts: Arc<KnownHosts>,
        negotiated: Arc<OnceLock<Algorithms>>,
    ) -> Self {
        Self {
            host: hop.host().to_owned(),
            port: hop.port(),
            address: hop.address(),
            known_hosts,
            negotiated,
        }
    }

    fn rejection(&self, verdict: Verdict, key: &PublicKey) -> Option<SshError> {
        let host = self.address.clone();
        let fingerprint = key.fingerprint(HashAlg::Sha256).to_string();

        match verdict {
            Verdict::Trusted => None,
            Verdict::Unknown => Some(SshError::HostKeyUnknown { host, fingerprint }),
            Verdict::Changed => Some(SshError::HostKeyChanged { host, fingerprint }),
            Verdict::OtherTypeKnown => Some(SshError::HostKeyOtherType { host, fingerprint }),
        }
    }

    fn host_key(presented: &PublicKeyOrCertificate) -> PublicKey {
        match presented {
            PublicKeyOrCertificate::PublicKey { key, .. } => key.clone(),
            PublicKeyOrCertificate::Certificate(certificate) => {
                PublicKey::from(certificate.public_key().clone())
            }
        }
    }
}

impl Handler for ClientHandler {
    type Error = SshError;

    async fn check_server_key(
        &mut self,
        presented: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        let key = Self::host_key(presented);
        let verdict = self.known_hosts.verdict(&self.host, self.port, &key);

        match self.rejection(verdict, &key) {
            Some(rejection) => Err(rejection),
            None => Ok(true),
        }
    }

    async fn kex_done(
        &mut self,
        _shared_secret: Option<&[u8]>,
        names: &Names,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        let _ = self.negotiated.set(Algorithms::negotiated(names));

        Ok(())
    }
}
