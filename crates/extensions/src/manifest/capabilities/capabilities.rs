use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{EventTopic, FileAccess, check_http_host, check_secret_name};
use crate::ExtensionError;
use crate::manifest::checks::refused;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    #[serde(default)]
    pub call: Vec<String>,

    #[serde(default)]
    pub storage: bool,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub http: Vec<String>,

    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub secrets: BTreeMap<String, Vec<String>>,

    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub documents: bool,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<EventTopic>,

    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub timers: bool,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files: Option<FileAccess>,
}

impl Capabilities {
    pub fn may_call(&self, action: &str) -> bool {
        self.call.iter().any(|allowed| allowed == action)
    }

    pub fn reaches(&self, host: &str) -> bool {
        self.http.iter().any(|listed| listed == host)
    }

    pub fn may_attach(&self, secret: &str, host: &str) -> bool {
        self.secrets
            .get(secret)
            .is_some_and(|hosts| hosts.iter().any(|listed| listed == host))
    }

    pub fn hears(&self, topic: EventTopic) -> bool {
        self.events.contains(&topic)
    }

    pub(crate) fn uses_api_0_2(&self) -> bool {
        !self.http.is_empty()
            || !self.secrets.is_empty()
            || self.documents
            || !self.events.is_empty()
            || self.timers
            || self.files.is_some()
    }

    pub(crate) fn check_http(&self) -> Result<(), ExtensionError> {
        for host in &self.http {
            check_http_host(host).map_err(refused)?;
        }

        for (secret, hosts) in &self.secrets {
            check_secret_name(secret).map_err(refused)?;
            self.check_secret_hosts(secret, hosts)?;
        }

        Ok(())
    }

    fn check_secret_hosts(&self, secret: &str, hosts: &[String]) -> Result<(), ExtensionError> {
        if hosts.is_empty() {
            return Err(refused(format!(
                "secret {secret} names no host it may be sent to"
            )));
        }

        match hosts.iter().find(|host| !self.reaches(host)) {
            Some(host) => Err(refused(format!(
                "secret {secret} may go to {host}, which is not listed in capabilities.http"
            ))),
            None => Ok(()),
        }
    }
}
