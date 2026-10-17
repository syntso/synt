use thiserror::Error;

use super::Spend;

#[derive(Debug, Clone, Copy, Error)]
#[error("more than {} {} in one call", .0.most(), .0.what())]
pub struct QuotaSpent(Spend);

impl QuotaSpent {
    pub(super) fn new(spend: Spend) -> Self {
        Self(spend)
    }
}
