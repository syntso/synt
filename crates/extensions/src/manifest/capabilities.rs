use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use synt_actions::Risk;

use synt_platform::SecretName;

use super::{ActionName, EventTopic, HttpHost};
use crate::ExtensionError;
use crate::files::FileAccess;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    #[serde(default)]
    call: Vec<ActionName>,

    #[serde(default)]
    storage: bool,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    http: Vec<HttpHost>,

    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    secrets: BTreeMap<SecretName, Vec<HttpHost>>,

    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    documents: bool,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    events: Vec<EventTopic>,

    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    timers: bool,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    files: Option<FileAccess>,
}

impl Capabilities {
    pub(crate) fn calls(&self) -> impl Iterator<Item = &ActionName> {
        self.call.iter()
    }

    pub(crate) fn may_call(&self, action: &str) -> bool {
        self.call.iter().any(|allowed| allowed.as_str() == action)
    }

    pub(crate) fn storage(&self) -> bool {
        self.storage
    }

    pub(crate) fn uses_http(&self) -> bool {
        !self.http.is_empty()
    }

    pub(crate) fn reaches(&self, host: &str) -> bool {
        self.http.iter().any(|listed| listed.as_str() == host)
    }

    pub(crate) fn secret_named(&self, secret: &str) -> Option<&SecretName> {
        self.secrets.get_key_value(secret).map(|(name, _)| name)
    }

    pub(crate) fn may_attach(&self, secret: &str, host: &str) -> bool {
        self.secrets
            .get(secret)
            .is_some_and(|hosts| hosts.iter().any(|listed| listed.as_str() == host))
    }

    pub(crate) fn documents(&self) -> bool {
        self.documents
    }

    pub(crate) fn hears(&self, topic: EventTopic) -> bool {
        self.events.contains(&topic)
    }

    pub(crate) fn timers(&self) -> bool {
        self.timers
    }

    pub(crate) fn files(&self) -> Option<FileAccess> {
        self.files
    }

    pub(crate) fn ceiling(&self) -> Risk {
        let writes_files = self.files == Some(FileAccess::ReadWrite);

        if self.http.is_empty() && !self.documents && !writes_files {
            return Risk::Read;
        }

        Risk::Write
    }

    pub(crate) fn uses_api_0_2(&self) -> bool {
        !self.http.is_empty()
            || !self.secrets.is_empty()
            || self.documents
            || !self.events.is_empty()
            || self.timers
            || self.files.is_some()
    }

    pub(crate) fn check_secret_hosts(&self) -> Result<(), ExtensionError> {
        self.secrets
            .iter()
            .try_for_each(|(secret, hosts)| self.check_secret(secret, hosts))
    }

    fn check_secret(&self, secret: &SecretName, hosts: &[HttpHost]) -> Result<(), ExtensionError> {
        if hosts.is_empty() {
            return Err(ExtensionError::SecretWithoutHost(secret.clone()));
        }

        match hosts.iter().find(|host| !self.http.contains(host)) {
            Some(host) => Err(ExtensionError::SecretHostNotListed {
                secret: secret.clone(),
                host: host.clone(),
            }),
            None => Ok(()),
        }
    }
}
