use std::fmt::{self, Display, Formatter};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub(crate) struct ForwardId(u64);

impl ForwardId {
    pub(crate) fn new(number: u64) -> Self {
        Self(number)
    }
}

impl Display for ForwardId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}
