use std::collections::HashMap;
use std::sync::Mutex;

use russh::keys::PublicKey;
use synt_platform::Lock;

use crate::Target;

#[derive(Default)]
pub(crate) struct Logins {
    accepted: Mutex<HashMap<String, PublicKey>>,
}

impl Logins {
    pub(crate) fn accepted_key(&self, target: &Target) -> Option<PublicKey> {
        self.accepted.locked().get(&target.label()).cloned()
    }

    pub(crate) fn remember(&self, target: &Target, key: PublicKey) {
        self.accepted.locked().insert(target.label(), key);
    }
}
