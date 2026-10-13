use std::sync::{Arc, OnceLock};
use std::time::Duration;

use ureq::Agent;
use ureq::tls::{Certificate, RootCerts, TlsConfig, TlsProvider};

use crate::ExtensionError;

const TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone)]
pub(super) struct Fetcher {
    agent: Arc<OnceLock<Agent>>,
    base: String,
}

impl Fetcher {
    pub(super) fn new(base: &str) -> Self {
        Self {
            agent: Arc::new(OnceLock::new()),
            base: base.trim_end_matches('/').to_owned(),
        }
    }

    pub(super) fn get(&self, file: &str, limit: u64) -> Result<Vec<u8>, ExtensionError> {
        let url = format!("{}/{file}", self.base);
        let agent = self.agent.get_or_init(agent);

        let mut response = agent
            .get(&url)
            .call()
            .map_err(|error| ExtensionError::Fetch(format!("{url}: {error}")))?;

        response
            .body_mut()
            .with_config()
            .limit(limit)
            .read_to_vec()
            .map_err(|error| ExtensionError::Fetch(format!("{url}: {error}")))
    }
}

fn agent() -> Agent {
    let tls = TlsConfig::builder()
        .provider(TlsProvider::Rustls)
        .root_certs(system_roots())
        .unversioned_rustls_crypto_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .build();

    Agent::config_builder()
        .tls_config(tls)
        .timeout_global(Some(TIMEOUT))
        .build()
        .new_agent()
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
