use serde::Serialize;

use super::Access;
use crate::id::MemberId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MemberInfo {
    pub member: MemberId,
    pub access: Access,
    pub this_device: bool,
}
