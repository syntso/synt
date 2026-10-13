use std::sync::Arc;

use russh::client::Handler;
use russh::keys::{HashAlg, PublicKey, PublicKeyOrCertificate};

use super::HandshakeError;
use crate::known_hosts::{KnownHosts, Verdict};
use crate::{SshError, Target};

pub(crate) struct ClientHandler {
    host: String,
    port: u16,
    address: String,
    known_hosts: Arc<KnownHosts>,
}

impl ClientHandler {
    pub(crate) fn new(hop: &Target, known_hosts: Arc<KnownHosts>) -> Self {
        Self {
            host: hop.host().to_owned(),
            port: hop.port(),
            address: hop.address(),
            known_hosts,
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
}

impl Handler for ClientHandler {
    type Error = HandshakeError;

    async fn check_server_key(
        &mut self,
        presented: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        let key = host_key(presented);
        let verdict = self.known_hosts.verdict(&self.host, self.port, &key);

        match self.rejection(verdict, &key) {
            Some(error) => Err(HandshakeError::Rejected(error)),
            None => Ok(true),
        }
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
