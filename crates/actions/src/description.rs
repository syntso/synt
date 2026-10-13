use serde::Serialize;
use serde_json::Value;
use synt_journal::ActionId;

use crate::Risk;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Description {
    pub id: ActionId,
    pub title: &'static str,
    pub risk: Risk,
    pub person_only: bool,
    pub takes_target: bool,
    pub params: Value,
}
