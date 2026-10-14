use serde::{Deserialize, Serialize};

use crate::encoding::{Base64, PublicKey};
use crate::id::{Generation, MemberId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WrappedKey {
    pub member: MemberId,
    pub by: MemberId,
    pub generation: Generation,
    pub ephemeral: PublicKey,
    pub sealed: Base64,
}
