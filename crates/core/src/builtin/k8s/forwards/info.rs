use serde::Serialize;
use synt_kube::ForwardStatus;

use super::ForwardId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ForwardInfo {
    pub(crate) id: ForwardId,
    pub(crate) cluster: String,

    #[serde(flatten)]
    pub(crate) status: ForwardStatus,
}
