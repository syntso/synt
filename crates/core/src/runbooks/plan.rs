mod step;

pub(crate) use step::PlannedStep;

use serde_json::{Map, Value, json};
use synt_actions::Handle;
use synt_journal::{ActionId, Actor, Name};
use synt_proto::{OneLine, RunPhase};

use super::RunbookError;
use super::document::{RunbookDocument, Step};
use super::param::Values;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Plan {
    reference: String,
    name: OneLine,
    values: Values,
    steps: Vec<PlannedStep>,
    rollback: Vec<PlannedStep>,
}

impl Plan {
    pub(crate) fn make(
        reference: String,
        document: &RunbookDocument,
        given: &Map<String, Value>,
        lookup: &impl Fn(&ActionId) -> Option<Handle>,
    ) -> Result<Self, RunbookError> {
        let values = Values::resolve(document.params(), given)?;

        let steps = Self::plan_steps(document.steps(), RunPhase::Steps, &values, lookup)?;
        let rollback = Self::plan_steps(document.rollback(), RunPhase::Rollback, &values, lookup)?;

        Ok(Self {
            reference,
            name: document.name().clone(),
            values,
            steps,
            rollback,
        })
    }

    #[cfg(test)]
    pub(crate) fn sample(steps: Vec<PlannedStep>, rollback: Vec<PlannedStep>) -> Self {
        Self {
            reference: "@lab/deploy".to_owned(),
            name: OneLine::new("Deploy").unwrap(),
            values: Values::default(),
            steps,
            rollback,
        }
    }

    pub(crate) fn reference(&self) -> &str {
        &self.reference
    }

    pub(crate) fn name(&self) -> &OneLine {
        &self.name
    }

    pub(crate) fn steps(&self) -> &[PlannedStep] {
        &self.steps
    }

    pub(crate) fn rollback(&self) -> &[PlannedStep] {
        &self.rollback
    }

    pub(crate) fn actor(&self) -> Result<Actor, RunbookError> {
        let name = Name::new(self.reference.as_str()).map_err(RunbookError::BadReference)?;

        Ok(Actor::Runbook(name))
    }

    pub(crate) fn shown(&self, approve_steps: bool) -> Value {
        json!({
            "runbook": self.reference,
            "name": self.name,
            "params": self.values,
            "approve_steps": approve_steps,
            "steps": self.steps,
            "rollback": self.rollback,
        })
    }

    fn plan_steps(
        steps: &[Step],
        phase: RunPhase,
        values: &Values,
        lookup: &impl Fn(&ActionId) -> Option<Handle>,
    ) -> Result<Vec<PlannedStep>, RunbookError> {
        (1..)
            .zip(steps)
            .map(|(number, step)| {
                PlannedStep::make(number, step, values, lookup)
                    .map_err(|error| error.in_step(phase, number))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Map, Value, json};
    use synt_actions::{Action, ActionError, Context, Handle, Registry, Risk};
    use synt_journal::{ActionId, Actor, ResourceRef};

    use super::Plan;
    use crate::runbooks::document::RunbookDocument;

    struct Shell;

    #[derive(serde::Deserialize, schemars::JsonSchema)]
    struct Command {
        command: String,
    }

    impl Action for Shell {
        const ID: &'static str = "test.shell";
        const TITLE: &'static str = "Pretend to run a command";
        const RISK: Risk = Risk::Write;
        const TAKES_TARGET: bool = true;

        type Params = Command;
        type Output = ();

        fn risk(&self, _target: Option<&ResourceRef>, params: &Command) -> Risk {
            if params.command.starts_with("sudo") {
                return Risk::Privileged;
            }

            Risk::Write
        }

        fn destination(&self, target: Option<&ResourceRef>, _params: &Command) -> Option<String> {
            target
                .filter(|target| target.as_str() != "nowhere")
                .map(|target| target.as_str().to_owned())
        }

        async fn run(&self, _context: &Context, _params: Command) -> Result<(), ActionError> {
            Ok(())
        }
    }

    struct Secret;

    impl Action for Secret {
        const ID: &'static str = "test.secret";
        const TITLE: &'static str = "Only for the person";
        const RISK: Risk = Risk::Read;
        const PERSON_ONLY: bool = true;

        type Params = ();
        type Output = ();

        async fn run(&self, _context: &Context, (): ()) -> Result<(), ActionError> {
            Ok(())
        }
    }

    fn registry() -> Registry {
        let mut registry = Registry::new();
        registry.register(Shell).unwrap();
        registry.register(Secret).unwrap();
        registry
    }

    fn runbook(steps: &str) -> RunbookDocument {
        let text = format!(
            "name = \"Deploy {{tag}}\"\n\
             [[params]]\nname = \"host\"\nkind = \"target\"\ndefault = \"api-1\"\n\
             [[params]]\nname = \"tag\"\nkind = \"word\"\n\
             {steps}"
        );

        RunbookDocument::from_toml(&text).unwrap()
    }

    fn plan(runbook: &RunbookDocument, given: &Value) -> Result<Plan, String> {
        let registry = registry();
        let lookup = |id: &ActionId| -> Option<Handle> { registry.get(id) };
        let given: Map<String, Value> = given.as_object().cloned().unwrap();

        Plan::make("@lab/deploy".to_owned(), runbook, &given, &lookup)
            .map_err(|error| error.to_string())
    }

    const DEPLOY: &str = "[[steps]]\ntitle = \"Pull {tag}\"\naction = \"test.shell\"\ntarget = \"{host}\"\n\
                          params = { command = \"docker pull app:{tag}\" }\n\
                          [[steps]]\ntitle = \"Switch\"\naction = \"test.shell\"\ntarget = \"{host}\"\n\
                          params = { command = \"sudo ln -sfn {tag} current\" }\n";

    #[test]
    fn a_plan_renders_every_step_with_the_given_values() {
        let plan = plan(&runbook(DEPLOY), &json!({ "tag": "v7" })).unwrap();

        assert_eq!(plan.steps()[0].title(), "Pull v7");
        assert_eq!(plan.steps()[0].target_text().as_deref(), Some("api-1"));
        assert_eq!(
            plan.steps()[0].params(),
            &json!({ "command": "docker pull app:v7" })
        );
        assert_eq!(
            plan.actor().unwrap(),
            Actor::Runbook(synt_journal::Name::new("@lab/deploy").unwrap())
        );
    }

    #[test]
    fn each_step_carries_the_risk_its_action_gives_it() {
        let plan = plan(&runbook(DEPLOY), &json!({ "tag": "v7" })).unwrap();

        assert_eq!(plan.steps()[0].risk(), Risk::Write);
        assert_eq!(plan.steps()[1].risk(), Risk::Privileged);
    }

    #[test]
    fn the_shown_plan_lists_steps_with_their_destinations_without_the_runner_settings() {
        let shown = plan(&runbook(DEPLOY), &json!({ "tag": "v7" }))
            .unwrap()
            .shown(true);

        assert_eq!(shown["approve_steps"], json!(true));
        assert_eq!(shown["params"], json!({ "host": "api-1", "tag": "v7" }));
        assert_eq!(
            shown["steps"][1],
            json!({
                "step": 2,
                "title": "Switch",
                "action": "test.shell",
                "target": "api-1",
                "params": { "command": "sudo ln -sfn v7 current" },
                "risk": "privileged",
                "destination": "api-1",
            })
        );
    }

    #[test]
    fn missing_values_stop_the_plan() {
        assert_eq!(
            plan(&runbook(DEPLOY), &json!({})).unwrap_err(),
            "param tag is required"
        );
    }

    #[test]
    fn steps_are_checked_against_their_actions_before_anything_runs() {
        let unknown = "[[steps]]\ntitle = \"x\"\naction = \"test.nothing\"\n";
        let untargeted =
            "[[steps]]\ntitle = \"x\"\naction = \"test.shell\"\nparams = { command = \"ls\" }\n";
        let targeted = "[[steps]]\ntitle = \"x\"\naction = \"test.secret\"\ntarget = \"{host}\"\n";
        let malformed = "[[steps]]\ntitle = \"x\"\naction = \"test.shell\"\ntarget = \"{host}\"\nparams = { cmd = \"ls\" }\n";
        let unreachable = "[[steps]]\ntitle = \"x\"\naction = \"test.shell\"\ntarget = \"nowhere\"\nparams = { command = \"ls\" }\n";

        let error = |steps: &str| plan(&runbook(steps), &json!({ "tag": "v1" })).unwrap_err();

        assert_eq!(error(unknown), "step 1: there is no action test.nothing");
        assert_eq!(error(untargeted), "step 1: test.shell needs a target");
        assert_eq!(
            error(targeted),
            "step 1: test.secret is for the person only, so a runbook cannot run it"
        );
        assert!(
            error(malformed).starts_with("step 1: invalid parameters"),
            "{}",
            error(malformed)
        );
        assert_eq!(
            error(unreachable),
            "step 1: nowhere does not name a host this core can reach"
        );
    }

    #[test]
    fn a_rendered_target_must_still_name_one_host() {
        let steps = "[[steps]]\ntitle = \"x\"\naction = \"test.shell\"\ntarget = \"{tag}*\"\nparams = { command = \"ls\" }\n";

        let error = plan(&runbook(steps), &json!({ "tag": "web" })).unwrap_err();

        assert!(error.contains("is not a target"), "{error}");
    }
}
