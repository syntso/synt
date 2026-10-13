use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use super::GrantInfo;
use crate::grants::AlwaysGrants;

#[derive(Debug, Clone)]
pub struct GrantList {
    grants: AlwaysGrants,
}

impl GrantList {
    pub fn new(grants: AlwaysGrants) -> Self {
        Self { grants }
    }
}

impl Action for GrantList {
    const ID: &'static str = "grant.list";
    const TITLE: &'static str = "List the standing grants";
    const RISK: Risk = Risk::Read;
    const PERSON_ONLY: bool = true;

    type Params = ();
    type Output = Vec<GrantInfo>;

    fn summary(&self, output: &Vec<GrantInfo>) -> Value {
        json!({ "grants": output.len() })
    }

    async fn run(&self, _context: &Context, (): ()) -> Result<Vec<GrantInfo>, ActionError> {
        Ok(self
            .grants
            .list()
            .into_iter()
            .map(GrantInfo::from)
            .collect())
    }
}
