use serde::Serialize;

use crate::terminal::SessionId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SessionSize {
    pub session: SessionId,
    #[serde(rename = "cols")]
    pub columns: u16,
    pub rows: u16,
}
