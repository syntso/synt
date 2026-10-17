use std::sync::{Arc, OnceLock};
use std::time::Duration;

use ureq::Agent;

use crate::ExtensionError;
use crate::http::TrustedTls;

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
        let agent = self.agent.get_or_init(Self::build_agent);

        let mut response = agent
            .get(&url)
            .call()
            .map_err(|source| ExtensionError::Fetch {
                url: url.clone(),
                source,
            })?;

        response
            .body_mut()
            .with_config()
            .limit(limit)
            .read_to_vec()
            .map_err(|source| ExtensionError::Fetch { url, source })
    }

    fn build_agent() -> Agent {
        Agent::config_builder()
            .tls_config(TrustedTls::config())
            .timeout_global(Some(TIMEOUT))
            .build()
            .new_agent()
    }
}
