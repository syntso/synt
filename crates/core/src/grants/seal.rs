use std::fmt;
use std::sync::Arc;

use hmac::digest::Key;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use synt_journal::ResourceRef;
use synt_platform::Vault;
use zeroize::Zeroizing;

use super::StandingGrant;
use crate::CoreError;

const KEY_ITEM: &str = "grants.key";
const SEAL_ITEM: &str = "grants.seal";
const KEY_LENGTH: usize = 32;

pub struct GrantSeal {
    vault: Arc<dyn Vault>,
    key: Zeroizing<Vec<u8>>,
}

impl GrantSeal {
    pub fn open(vault: Arc<dyn Vault>) -> Result<Self, CoreError> {
        let key = match vault.read(KEY_ITEM)? {
            Some(key) if key.len() == KEY_LENGTH => key,
            Some(_) => return Err(CoreError::DamagedKey(KEY_ITEM.to_owned())),
            None => Self::new_key(vault.as_ref())?,
        };

        Ok(Self { vault, key })
    }

    pub(crate) fn matches(&self, grants: &[StandingGrant]) -> Result<bool, CoreError> {
        let Some(seal) = self.vault.read(SEAL_ITEM)? else {
            return Ok(grants.is_empty());
        };

        Ok(self.mac(grants).verify_slice(&seal).is_ok())
    }

    pub(crate) fn save(&self, grants: &[StandingGrant]) -> Result<(), CoreError> {
        let seal = self.mac(grants).finalize().into_bytes();

        self.vault.write(SEAL_ITEM, &seal)?;

        Ok(())
    }

    fn new_key(vault: &dyn Vault) -> Result<Zeroizing<Vec<u8>>, CoreError> {
        let mut key = Zeroizing::new(vec![0_u8; KEY_LENGTH]);
        getrandom::fill(&mut key)?;

        vault.write(KEY_ITEM, &key)?;

        Ok(key)
    }

    fn mac(&self, grants: &[StandingGrant]) -> Hmac<Sha256> {
        let mut mac = self.keyed();

        let mut sorted: Vec<&StandingGrant> = grants.iter().collect();
        sorted.sort_by_key(|grant| grant.id());

        for grant in sorted {
            mac.update(&Self::canonical(grant));
        }

        mac
    }

    fn keyed(&self) -> Hmac<Sha256> {
        let mut block = Key::<Hmac<Sha256>>::default();
        block[..self.key.len()].copy_from_slice(&self.key);

        Hmac::new(&block)
    }

    fn canonical(grant: &StandingGrant) -> Vec<u8> {
        let scope = grant.scope();
        let actor = scope.actor().to_string();
        let created_at = u64::try_from(grant.created_at().as_millis()).unwrap_or_default();

        let mut bytes = Vec::new();

        bytes.extend_from_slice(&grant.id().get().to_be_bytes());
        Self::push_text(&mut bytes, Some(&actor));
        Self::push_text(&mut bytes, Some(scope.action().as_str()));
        Self::push_text(&mut bytes, scope.target().map(ResourceRef::as_str));
        Self::push_text(&mut bytes, scope.destination());
        bytes.extend_from_slice(&created_at.to_be_bytes());

        bytes
    }

    fn push_text(bytes: &mut Vec<u8>, text: Option<&str>) {
        let Some(text) = text else {
            bytes.push(0);
            return;
        };

        bytes.push(1);
        bytes.extend_from_slice(&(text.len() as u64).to_be_bytes());
        bytes.extend_from_slice(text.as_bytes());
    }
}

impl fmt::Debug for GrantSeal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GrantSeal")
            .field("place", &self.vault.place())
            .finish_non_exhaustive()
    }
}
