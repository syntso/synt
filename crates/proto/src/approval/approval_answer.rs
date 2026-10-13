use serde::{Deserialize, Serialize};

use super::{ApprovalDecision, ApprovalId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalAnswer {
    pub id: ApprovalId,
    pub decision: ApprovalDecision,
}
