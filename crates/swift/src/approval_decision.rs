#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ApprovalDecision {
    AllowOnce,
    AllowUntilRestart,
    AllowAlways,
    Deny,
}

impl From<ApprovalDecision> for synt_proto::ApprovalDecision {
    fn from(decision: ApprovalDecision) -> Self {
        match decision {
            ApprovalDecision::AllowOnce => Self::AllowOnce,
            ApprovalDecision::AllowUntilRestart => Self::AllowUntilRestart,
            ApprovalDecision::AllowAlways => Self::AllowAlways,
            ApprovalDecision::Deny => Self::Deny,
        }
    }
}
