use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use synt_ssh::Destination;

use crate::projects::names::check_name;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TunnelResource {
    pub host: String,
    pub remote: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local: Option<u16>,
}

impl TunnelResource {
    pub(crate) fn check(&self) -> Result<(), String> {
        check_name(&self.host).map_err(|reason| format!("host: {reason}"))?;

        if self.local == Some(0) {
            return Err("local must be between 1 and 65535".to_owned());
        }

        self.destination().map(|_| ())
    }

    pub(crate) fn destination(&self) -> Result<Destination, String> {
        let invalid = || {
            format!(
                "remote must be host:port, such as orders-db:5432, got {:?}",
                self.remote
            )
        };

        let (host, port) = self.remote.rsplit_once(':').ok_or_else(invalid)?;
        let host = host.trim_start_matches('[').trim_end_matches(']');
        let port: u16 = port.parse().map_err(|_| invalid())?;

        if host.is_empty() || port == 0 || host.contains(char::is_whitespace) {
            return Err(invalid());
        }

        Ok(Destination::new(host, port))
    }
}
