use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::SyncError;
use crate::encoding::PublicKey;
use crate::id::MemberId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberKey {
    pub member: MemberId,
    pub public_key: PublicKey,
}

impl MemberKey {
    pub fn new(public_key: PublicKey) -> Self {
        Self {
            member: member_of(&public_key),
            public_key,
        }
    }

    pub fn check(&self) -> Result<(), SyncError> {
        if member_of(&self.public_key) == self.member {
            return Ok(());
        }

        Err(SyncError::refused(format!(
            "member {} does not match its public key",
            self.member
        )))
    }
}

fn member_of(public_key: &PublicKey) -> MemberId {
    MemberId::from_digest(&Sha256::digest(public_key.get()))
}

#[cfg(test)]
mod tests {
    use super::MemberKey;
    use crate::encoding::PublicKey;

    #[test]
    fn the_member_id_comes_from_the_public_key() {
        let one = MemberKey::new(PublicKey::new([1; 32]));
        let other = MemberKey::new(PublicKey::new([2; 32]));

        assert_ne!(one.member, other.member);
        assert!(one.check().is_ok());
    }

    #[test]
    fn a_member_id_paired_with_another_key_is_refused() {
        let mut forged = MemberKey::new(PublicKey::new([1; 32]));
        forged.public_key = PublicKey::new([2; 32]);

        assert!(forged.check().is_err());
    }
}
