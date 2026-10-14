use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use synt_journal::ActionId;

use super::{Expect, OnFailure, Retry};
use crate::runbooks::template::JsonTemplate;

const LONGEST_TITLE: usize = 200;
const RUNBOOK_ACTIONS: &str = "runbook.";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub title: String,
    pub action: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,

    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub params: Map<String, Value>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expect: Option<Expect>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry: Option<Retry>,

    #[serde(default, skip_serializing_if = "is_default")]
    pub on_failure: OnFailure,
}

impl Step {
    pub(crate) fn check(&self) -> Result<(), String> {
        let fits = !self.title.trim().is_empty() && self.title.len() <= LONGEST_TITLE;

        if !fits || self.title.contains(char::is_control) {
            return Err(format!(
                "a title must be 1 to {LONGEST_TITLE} characters on one line"
            ));
        }

        let action = ActionId::new(&self.action).map_err(|error| format!("action: {error}"))?;

        if action.as_str().starts_with(RUNBOOK_ACTIONS) {
            return Err(format!("{action} cannot be a step of a runbook"));
        }

        if let Some(expect) = &self.expect {
            expect.check()?;
        }

        match &self.retry {
            Some(retry) => retry.check(),
            None => Ok(()),
        }
    }

    pub(crate) fn params_used(&self) -> Vec<String> {
        let contains = self
            .expect
            .as_ref()
            .and_then(|expect| expect.contains.as_deref());
        let texts = [Some(self.title.as_str()), self.target.as_deref(), contains];

        let mut used: Vec<String> = texts
            .into_iter()
            .flatten()
            .flat_map(|text| Value::from(text).placeholders())
            .collect();

        used.extend(self.params.placeholders());
        used
    }

    pub(crate) fn retry(&self) -> Retry {
        self.retry.unwrap_or(Retry::ONCE)
    }
}

fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
}
