use std::collections::HashMap;
use std::sync::Mutex;

use russh::keys::PublicKey;

use crate::Target;
use crate::pool::locked;

#[derive(Default)]
pub(crate) struct Logins {
    accepted: Mutex<HashMap<String, PublicKey>>,
}

impl Logins {
    pub(crate) fn accepted_key(&self, target: &Target) -> Option<PublicKey> {
        locked(&self.accepted).get(&target.label()).cloned()
    }

    pub(crate) fn remember(&self, target: &Target, key: PublicKey) {
        locked(&self.accepted).insert(target.label(), key);
    }
}
