mod action;

pub use action::StepAction;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::{Expect, OnFailure, OneLine, Retry};
use crate::runbooks::RunbookError;
use crate::runbooks::param::ParamName;
use crate::runbooks::template::JsonTemplate;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Step {
    title: OneLine,
    action: StepAction,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    target: Option<String>,

    #[serde(default, skip_serializing_if = "Map::is_empty")]
    params: Map<String, Value>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    expect: Option<Expect>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    retry: Option<Retry>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    on_failure: Option<OnFailure>,
}

impl Step {
    pub(crate) const SHELL_ACTION: &str = "ssh.exec";
    pub(crate) const COMMAND: &str = "command";

    pub(crate) fn new(
        title: OneLine,
        action: StepAction,
        target: Option<String>,
        params: Map<String, Value>,
    ) -> Self {
        Self {
            title,
            action,
            target,
            params,
            expect: None,
            retry: None,
            on_failure: None,
        }
    }

    pub(crate) fn check(&self) -> Result<(), RunbookError> {
        match &self.retry {
            Some(retry) => retry.check(),
            None => Ok(()),
        }
    }

    pub(crate) fn title(&self) -> &OneLine {
        &self.title
    }

    pub(crate) fn action(&self) -> &StepAction {
        &self.action
    }

    pub(crate) fn target(&self) -> Option<&str> {
        self.target.as_deref()
    }

    pub(crate) fn params(&self) -> &Map<String, Value> {
        &self.params
    }

    pub(crate) fn expect(&self) -> Option<&Expect> {
        self.expect.as_ref()
    }

    pub(crate) fn on_failure(&self) -> OnFailure {
        self.on_failure.unwrap_or_default()
    }

    pub(crate) fn retry(&self) -> Retry {
        self.retry.unwrap_or(Retry::ONCE)
    }

    pub(crate) fn exit_wanted(&self) -> i64 {
        self.expect.as_ref().map_or(0, Expect::exit_wanted)
    }

    pub(crate) fn shell_command(&self) -> Option<&str> {
        if self.action.as_str() != Self::SHELL_ACTION {
            return None;
        }

        self.params.get(Self::COMMAND)?.as_str()
    }

    pub(crate) fn params_used(&self) -> Vec<ParamName> {
        let contains = self.expect.as_ref().and_then(Expect::contains);
        let texts = [Some(self.title.as_str()), self.target.as_deref(), contains];

        let mut used: Vec<ParamName> = texts
            .into_iter()
            .flatten()
            .flat_map(|text| Value::from(text).placeholders())
            .collect();

        used.extend(self.params.placeholders());
        used
    }
}
