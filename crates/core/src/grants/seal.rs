use std::fmt;
use std::sync::Arc;

use hmac::digest::Key;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use synt_platform::Vault;
use zeroize::Zeroizing;

use crate::CoreError;
use crate::state::StoredGrant;

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
            None => new_key(vault.as_ref())?,
        };

        Ok(Self { vault, key })
    }

    pub(crate) fn matches(&self, grants: &[StoredGrant]) -> Result<bool, CoreError> {
        let Some(seal) = self.vault.read(SEAL_ITEM)? else {
            return Ok(grants.is_empty());
        };

        Ok(self.mac(grants).verify_slice(&seal).is_ok())
    }

    pub(crate) fn save(&self, grants: &[StoredGrant]) -> Result<(), CoreError> {
        let seal = self.mac(grants).finalize().into_bytes();

        self.vault.write(SEAL_ITEM, &seal)?;

        Ok(())
    }

    fn mac(&self, grants: &[StoredGrant]) -> Hmac<Sha256> {
        let mut mac = self.keyed();

        let mut sorted: Vec<&StoredGrant> = grants.iter().collect();
        sorted.sort_by_key(|grant| grant.id);

        for grant in sorted {
            mac.update(&canonical(grant));
        }

        mac
    }

    fn keyed(&self) -> Hmac<Sha256> {
        let mut block = Key::<Hmac<Sha256>>::default();
        block[..self.key.len()].copy_from_slice(&self.key);

        Hmac::new(&block)
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

fn new_key(vault: &dyn Vault) -> Result<Zeroizing<Vec<u8>>, CoreError> {
    let mut key = Zeroizing::new(vec![0_u8; KEY_LENGTH]);
    getrandom::fill(&mut key).map_err(|error| CoreError::Random(error.to_string()))?;

    vault.write(KEY_ITEM, &key)?;

    Ok(key)
}

fn canonical(grant: &StoredGrant) -> Vec<u8> {
    let mut bytes = Vec::new();

    bytes.extend_from_slice(&grant.id.to_be_bytes());
    push_text(&mut bytes, Some(&grant.actor));
    push_text(&mut bytes, Some(&grant.action));
    push_text(&mut bytes, grant.resource.as_deref());
    push_text(&mut bytes, grant.destination.as_deref());
    bytes.extend_from_slice(&grant.created_at.to_be_bytes());

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
