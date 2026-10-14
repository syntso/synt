use serde::Serialize;
use synt_kube::ForwardStatus;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ForwardInfo {
    pub(crate) id: u64,
    pub(crate) cluster: String,

    #[serde(flatten)]
    pub(crate) status: ForwardStatus,
}
