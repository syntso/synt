use serde::Serialize;
use serde_json::Value;
use synt_actions::{Prepared, RegisteredAction, Risk};
use synt_journal::{ActionId, ResourceRef};

use crate::runbooks::RunbookError;
use crate::runbooks::document::{Expect, OnFailure, Retry, Step};
use crate::runbooks::param::{ParamKind, Values};
use crate::runbooks::template::{JsonTemplate, TextTemplate};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct PlannedStep {
    #[serde(rename = "step")]
    number: usize,
    title: String,
    action: ActionId,
    target: Option<ResourceRef>,
    params: Value,
    risk: Risk,

    #[serde(skip_serializing_if = "Option::is_none")]
    destination: Option<String>,

    #[serde(skip)]
    expect: Expect,

    #[serde(skip)]
    retry: Retry,

    #[serde(skip)]
    on_failure: OnFailure,
}

impl PlannedStep {
    pub(crate) fn make(
        number: usize,
        step: &Step,
        values: &Values,
        lookup: &impl Fn(&ActionId) -> Option<RegisteredAction>,
    ) -> Result<Self, RunbookError> {
        let action = step.action().id().clone();
        let target = Self::render_target(step.target(), values)?;
        let params = Value::Object(step.params().render(values)?);

        let handle = lookup(&action).ok_or_else(|| RunbookError::NoAction(action.clone()))?;
        let prepared = Self::prepare(&handle, target.as_ref(), &params)?;

        Ok(Self {
            number,
            title: TextTemplate::parse(step.title().as_str()).render_text(values)?,
            action,
            target,
            params,
            risk: prepared.risk(),
            destination: prepared.destination().map(str::to_owned),
            expect: Self::render_expect(step.expect(), values)?,
            retry: step.retry(),
            on_failure: step.on_failure(),
        })
    }

    #[cfg(test)]
    pub(crate) fn sample(number: usize, title: &str) -> Self {
        Self {
            number,
            title: title.to_owned(),
            action: ActionId::new(Step::SHELL_ACTION).unwrap(),
            target: None,
            params: Value::Object(serde_json::Map::new()),
            risk: Risk::Write,
            destination: None,
            expect: Expect::default(),
            retry: Retry::ONCE,
            on_failure: OnFailure::Stop,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_retry(self, retry: Retry) -> Self {
        Self { retry, ..self }
    }

    #[cfg(test)]
    pub(crate) fn with_on_failure(self, on_failure: OnFailure) -> Self {
        Self { on_failure, ..self }
    }

    pub(crate) fn number(&self) -> usize {
        self.number
    }

    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    pub(crate) fn action(&self) -> &ActionId {
        &self.action
    }

    pub(crate) fn params(&self) -> &Value {
        &self.params
    }

    pub(crate) fn destination(&self) -> Option<&str> {
        self.destination.as_deref()
    }

    pub(crate) fn expect(&self) -> &Expect {
        &self.expect
    }

    pub(crate) fn retry(&self) -> Retry {
        self.retry
    }

    pub(crate) fn on_failure(&self) -> OnFailure {
        self.on_failure
    }

    #[cfg(test)]
    pub(crate) fn risk(&self) -> Risk {
        self.risk
    }

    pub(crate) fn target_text(&self) -> Option<String> {
        self.target
            .as_ref()
            .map(|target| target.as_str().to_owned())
    }

    fn render_target(
        target: Option<&str>,
        values: &Values,
    ) -> Result<Option<ResourceRef>, RunbookError> {
        let Some(target) = target else {
            return Ok(None);
        };

        let text = TextTemplate::parse(target).render_text(values)?;
        let checked = ParamKind::Target.accept(&Value::from(text))?;
        let reference = ResourceRef::new(checked.as_str().unwrap_or_default())
            .map_err(RunbookError::BadReference)?;

        Ok(Some(reference))
    }

    fn render_expect(expect: Option<&Expect>, values: &Values) -> Result<Expect, RunbookError> {
        match expect {
            Some(expect) => expect.render(values),
            None => Ok(Expect::default()),
        }
    }

    fn prepare(
        handle: &RegisteredAction,
        target: Option<&ResourceRef>,
        params: &Value,
    ) -> Result<Prepared, RunbookError> {
        Self::admit(handle, target)?;

        let prepared = handle.prepare(target, params.clone())?;

        if let Some(target) = target.filter(|_| prepared.destination().is_none()) {
            return Err(RunbookError::Unreachable(target.clone()));
        }

        Ok(prepared)
    }

    fn admit(handle: &RegisteredAction, target: Option<&ResourceRef>) -> Result<(), RunbookError> {
        let action = handle.id();

        if handle.person_only() {
            return Err(RunbookError::PersonOnly(action.clone()));
        }

        match (handle.takes_target(), target) {
            (true, None) => Err(RunbookError::NeedsTarget(action.clone())),
            (false, Some(_)) => Err(RunbookError::TakesNoTarget(action.clone())),
            _ => Ok(()),
        }
    }
}
