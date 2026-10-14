use serde::Serialize;
use serde_json::Value;
use synt_actions::{Handle, Risk};
use synt_journal::{ActionId, ResourceRef};

use crate::runbooks::document::{Expect, OnFailure, Retry, Step};
use crate::runbooks::param::{ParamKind, Values};
use crate::runbooks::template::{Template, render_fields};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct PlannedStep {
    #[serde(rename = "step")]
    pub(crate) number: usize,
    pub(crate) title: String,
    pub(crate) action: ActionId,
    pub(crate) target: Option<ResourceRef>,
    pub(crate) params: Value,
    pub(crate) risk: Risk,

    #[serde(skip)]
    pub(crate) expect: Expect,

    #[serde(skip)]
    pub(crate) retry: Retry,

    #[serde(skip)]
    pub(crate) on_failure: OnFailure,
}

impl PlannedStep {
    pub(crate) fn make(
        number: usize,
        step: &Step,
        values: &Values,
        lookup: &impl Fn(&ActionId) -> Option<Handle>,
    ) -> Result<Self, String> {
        let action = ActionId::new(&step.action).map_err(|error| error.to_string())?;
        let target = step
            .target
            .as_deref()
            .map(|target| rendered_target(target, values))
            .transpose()?;
        let params = Value::Object(render_fields(&step.params, values)?);

        let handle = lookup(&action).ok_or_else(|| format!("there is no action {action}"))?;
        let risk = judge(&handle, target.as_ref(), &params)?;

        Ok(Self {
            number,
            title: Template::parse(&step.title).render_text(values)?,
            action,
            target,
            params,
            risk,
            expect: rendered_expect(step.expect.as_ref(), values)?,
            retry: step.retry(),
            on_failure: step.on_failure,
        })
    }

    pub(crate) fn target_text(&self) -> Option<String> {
        self.target
            .as_ref()
            .map(|target| target.as_str().to_owned())
    }
}

fn rendered_target(target: &str, values: &Values) -> Result<ResourceRef, String> {
    let text = Template::parse(target).render_text(values)?;
    let checked = ParamKind::Target.accept(&Value::from(text.as_str()))?;

    ResourceRef::new(checked.as_str().unwrap_or_default()).map_err(|error| error.to_string())
}

fn rendered_expect(expect: Option<&Expect>, values: &Values) -> Result<Expect, String> {
    let Some(expect) = expect else {
        return Ok(Expect::default());
    };

    let contains = expect
        .contains
        .as_deref()
        .map(|text| Template::parse(text).render_text(values))
        .transpose()?;

    Ok(Expect {
        contains,
        ..expect.clone()
    })
}

fn judge(handle: &Handle, target: Option<&ResourceRef>, params: &Value) -> Result<Risk, String> {
    let action = handle.id();

    if handle.person_only() {
        return Err(format!(
            "{action} is for the person only, so a runbook cannot run it"
        ));
    }

    match (handle.takes_target(), target) {
        (true, None) => return Err(format!("{action} needs a target")),
        (false, Some(_)) => return Err(format!("{action} takes no target")),
        _ => {}
    }

    let prepared = handle
        .prepare(target, params.clone())
        .map_err(|error| error.to_string())?;

    if let Some(target) = target.filter(|_| prepared.destination().is_none()) {
        return Err(format!("{target} does not name a host this core can reach"));
    }

    Ok(prepared.risk())
}
