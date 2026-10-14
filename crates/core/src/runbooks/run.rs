mod progress;
mod report;
mod state;
mod step;

pub(crate) use progress::{LinkProgress, Progress};
pub use report::RunReport;
pub use state::RunState;
pub use step::StepReport;
pub(crate) use step::{CoreSteps, StepCaller};

use std::time::Duration;

use synt_proto::{RunPhase, StepState};
use tokio::time;
use tokio_util::sync::CancellationToken;

use super::document::OnFailure;
use super::plan::{Plan, PlannedStep};

pub(crate) struct Run<C, P> {
    caller: C,
    progress: P,
    stop: CancellationToken,
    id: u64,
}

impl<C: StepCaller, P: Progress> Run<C, P> {
    pub(crate) fn new(caller: C, progress: P, stop: CancellationToken, id: u64) -> Self {
        Self {
            caller,
            progress,
            stop,
            id,
        }
    }

    pub(crate) async fn perform(&self, plan: &Plan) -> RunReport {
        let mut steps: Vec<StepReport> = plan.steps().iter().map(StepReport::skipped).collect();
        let state = self.run_steps(plan, &mut steps).await;

        if state != RunState::RolledBack {
            return RunReport::new(self.id, plan, state, steps, Vec::new());
        }

        let rollback = self.roll_back(plan).await;
        let state = if self.stop.is_cancelled() {
            RunState::Stopped
        } else {
            RunState::RolledBack
        };

        RunReport::new(self.id, plan, state, steps, rollback)
    }

    async fn run_steps(&self, plan: &Plan, reports: &mut [StepReport]) -> RunState {
        for (step, report) in plan.steps().iter().zip(reports) {
            if self.stop.is_cancelled() {
                return RunState::Stopped;
            }

            *report = self.attempt(plan, RunPhase::Steps, step).await;

            if self.stop.is_cancelled() {
                return RunState::Stopped;
            }

            if !report.failed() {
                continue;
            }

            match step.on_failure() {
                OnFailure::Continue => {}
                OnFailure::Stop => return RunState::Failed,
                OnFailure::Rollback => return RunState::RolledBack,
            }
        }

        RunState::Completed
    }

    async fn roll_back(&self, plan: &Plan) -> Vec<StepReport> {
        let mut reports = Vec::with_capacity(plan.rollback().len());

        for step in plan.rollback().iter().rev() {
            let report = if self.stop.is_cancelled() {
                StepReport::skipped(step)
            } else {
                self.attempt(plan, RunPhase::Rollback, step).await
            };

            reports.push(report);
        }

        reports
    }

    async fn attempt(&self, plan: &Plan, phase: RunPhase, step: &PlannedStep) -> StepReport {
        let mut report = StepReport::skipped(step);
        let attempts = step.retry().attempts();

        for attempt in 1..=attempts {
            if attempt > 1 && !self.pause(step.retry().pause()).await {
                break;
            }

            report.begin_attempt(attempt);
            self.announce(plan, phase, &report, StepState::Started)
                .await;

            let outcome = self.caller.call(step).await;
            report.settle(step, outcome);

            if report.succeeded() || self.stop.is_cancelled() {
                break;
            }

            if attempt < attempts {
                self.announce(plan, phase, &report, StepState::Retrying)
                    .await;
            }
        }

        self.announce(plan, phase, &report, report.state()).await;

        report
    }

    async fn pause(&self, pause: Duration) -> bool {
        tokio::select! {
            () = self.stop.cancelled() => false,
            () = time::sleep(pause) => true,
        }
    }

    async fn announce(&self, plan: &Plan, phase: RunPhase, report: &StepReport, state: StepState) {
        let notice = report.notice(self.id, plan.reference(), phase, state);

        self.progress.report(notice).await;
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, VecDeque};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use serde_json::{Value, json};
    use synt_proto::{
        ActionOutcome, ErrorCode, RequestSeq, RpcError, RunPhase, RunbookStep, StepState,
    };
    use tokio_util::sync::CancellationToken;

    use super::{Progress, Run, StepCaller};
    use crate::runbooks::document::{OnFailure, Retry};
    use crate::runbooks::plan::{Plan, PlannedStep};

    type Answer = Result<Value, &'static str>;

    #[derive(Clone, Default)]
    struct Script {
        answers: Arc<Mutex<HashMap<String, VecDeque<Answer>>>>,
        calls: Arc<Mutex<Vec<String>>>,
        stop_after: Arc<Mutex<Option<(String, CancellationToken)>>>,
    }

    impl Script {
        fn answer(&self, title: &str, answers: impl IntoIterator<Item = Answer>) -> &Self {
            self.answers
                .lock()
                .unwrap()
                .insert(title.to_owned(), answers.into_iter().collect());
            self
        }

        fn stop_after(&self, title: &str, stop: &CancellationToken) {
            *self.stop_after.lock().unwrap() = Some((title.to_owned(), stop.clone()));
        }

        fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl StepCaller for Script {
        async fn call(&self, step: &PlannedStep) -> Result<ActionOutcome, RpcError> {
            let mut calls = self.calls.lock().unwrap();
            calls.push(step.title().to_owned());
            let request = calls.len() as u64 + 100;
            drop(calls);

            if let Some((title, stop)) = self.stop_after.lock().unwrap().as_ref()
                && title == step.title()
            {
                stop.cancel();
            }

            let answer = self
                .answers
                .lock()
                .unwrap()
                .get_mut(step.title())
                .and_then(VecDeque::pop_front)
                .unwrap_or(Ok(json!({ "exit": 0 })));

            match answer {
                Ok(value) => Ok(ActionOutcome {
                    request: RequestSeq(request),
                    value,
                }),
                Err(reason) => Err(RpcError::new(ErrorCode::ACTION_FAILED, reason)),
            }
        }
    }

    #[derive(Clone, Default)]
    struct Heard {
        notices: Arc<Mutex<Vec<RunbookStep>>>,
    }

    impl Heard {
        fn states(&self) -> Vec<(RunPhase, usize, StepState, u32)> {
            self.notices
                .lock()
                .unwrap()
                .iter()
                .map(|notice| (notice.phase, notice.step, notice.state, notice.attempt))
                .collect()
        }
    }

    impl Progress for Heard {
        async fn report(&self, step: RunbookStep) {
            self.notices.lock().unwrap().push(step);
        }
    }

    fn step(number: usize, title: &str) -> PlannedStep {
        PlannedStep::sample(number, title)
    }

    async fn run(plan: &Plan, script: &Script) -> (Value, Heard) {
        run_with(plan, script, CancellationToken::new()).await
    }

    async fn run_with(plan: &Plan, script: &Script, stop: CancellationToken) -> (Value, Heard) {
        let heard = Heard::default();
        let run = Run::new(script.clone(), heard.clone(), stop, 40);
        let report = serde_json::to_value(run.perform(plan).await).unwrap();

        (report, heard)
    }

    fn states(reports: &Value) -> Vec<&str> {
        reports
            .as_array()
            .unwrap()
            .iter()
            .map(|report| report["state"].as_str().unwrap())
            .collect()
    }

    #[tokio::test]
    async fn steps_run_in_order_and_the_run_completes() {
        let script = Script::default();
        let plan = Plan::sample(vec![step(1, "a"), step(2, "b")], Vec::new());

        let (report, heard) = run(&plan, &script).await;

        assert_eq!(report["state"], json!("completed"));
        assert_eq!(script.calls(), ["a", "b"]);
        assert_eq!(report["steps"][1]["request"], json!(102));
        assert_eq!(
            heard.states(),
            [
                (RunPhase::Steps, 1, StepState::Started, 1),
                (RunPhase::Steps, 1, StepState::Succeeded, 1),
                (RunPhase::Steps, 2, StepState::Started, 1),
                (RunPhase::Steps, 2, StepState::Succeeded, 1),
            ]
        );
    }

    #[tokio::test]
    async fn a_failing_step_stops_the_run_and_skips_the_rest() {
        let script = Script::default();
        script.answer("a", [Err("the disk is full")]);
        let plan = Plan::sample(vec![step(1, "a"), step(2, "b")], Vec::new());

        let (report, _) = run(&plan, &script).await;

        assert_eq!(report["state"], json!("failed"));
        assert_eq!(states(&report["steps"]), ["failed", "skipped"]);
        assert_eq!(report["steps"][0]["error"], json!("the disk is full"));
        assert_eq!(script.calls(), ["a"]);
    }

    #[tokio::test]
    async fn an_unmet_expectation_fails_the_step_with_its_reason() {
        let script = Script::default();
        script.answer("a", [Ok(json!({ "exit": 2, "stdout": "" }))]);
        let plan = Plan::sample(vec![step(1, "a")], Vec::new());

        let (report, _) = run(&plan, &script).await;

        assert_eq!(report["steps"][0]["state"], json!("failed"));
        assert_eq!(report["steps"][0]["error"], json!("exit 2, expected 0"));
        assert_eq!(report["steps"][0]["request"], json!(101));
    }

    #[tokio::test]
    async fn a_step_that_may_fail_lets_the_run_go_on() {
        let script = Script::default();
        script.answer("a", [Err("no such unit")]);
        let first = step(1, "a").with_on_failure(OnFailure::Continue);
        let plan = Plan::sample(vec![first, step(2, "b")], Vec::new());

        let (report, _) = run(&plan, &script).await;

        assert_eq!(report["state"], json!("completed"));
        assert_eq!(states(&report["steps"]), ["failed", "succeeded"]);
    }

    #[tokio::test(start_paused = true)]
    async fn a_retried_step_is_called_again_until_it_succeeds() {
        let script = Script::default();
        script.answer(
            "health",
            [
                Ok(json!({ "exit": 7 })),
                Err("refused"),
                Ok(json!({ "exit": 0 })),
            ],
        );
        let health = step(1, "health").with_retry(Retry::new(5, 3));
        let plan = Plan::sample(vec![health], Vec::new());

        let started = tokio::time::Instant::now();
        let (report, heard) = run(&plan, &script).await;

        assert_eq!(report["steps"][0]["state"], json!("succeeded"));
        assert_eq!(report["steps"][0]["attempts"], json!(3));
        assert_eq!(started.elapsed(), Duration::from_secs(6));
        assert_eq!(
            heard.states(),
            [
                (RunPhase::Steps, 1, StepState::Started, 1),
                (RunPhase::Steps, 1, StepState::Retrying, 1),
                (RunPhase::Steps, 1, StepState::Started, 2),
                (RunPhase::Steps, 1, StepState::Retrying, 2),
                (RunPhase::Steps, 1, StepState::Started, 3),
                (RunPhase::Steps, 1, StepState::Succeeded, 3),
            ]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_step_that_never_succeeds_fails_after_its_last_attempt() {
        let script = Script::default();
        script.answer("health", [Err("refused"), Err("refused")]);
        let health = step(1, "health").with_retry(Retry::new(2, 1));
        let plan = Plan::sample(vec![health], Vec::new());

        let (report, heard) = run(&plan, &script).await;

        assert_eq!(report["state"], json!("failed"));
        assert_eq!(report["steps"][0]["attempts"], json!(2));
        assert_eq!(
            heard.states().last(),
            Some(&(RunPhase::Steps, 1, StepState::Failed, 2))
        );
    }

    #[tokio::test]
    async fn a_failure_that_rolls_back_runs_the_rollback_steps_in_reverse() {
        let script = Script::default();
        script.answer("migrate", [Err("lock timeout")]);
        script.answer("undo copy", [Err("already gone")]);
        let migrate = step(2, "migrate").with_on_failure(OnFailure::Rollback);
        let plan = Plan::sample(
            vec![step(1, "copy"), migrate, step(3, "restart")],
            vec![step(1, "undo copy"), step(2, "undo migrate")],
        );

        let (report, heard) = run(&plan, &script).await;

        assert_eq!(report["state"], json!("rolled_back"));
        assert_eq!(
            script.calls(),
            ["copy", "migrate", "undo migrate", "undo copy"]
        );
        assert_eq!(states(&report["steps"]), ["succeeded", "failed", "skipped"]);
        assert_eq!(states(&report["rollback"]), ["succeeded", "failed"]);
        assert!(
            heard
                .states()
                .contains(&(RunPhase::Rollback, 2, StepState::Succeeded, 1))
        );
    }

    #[tokio::test]
    async fn a_stopped_run_starts_no_further_step_and_does_not_roll_back() {
        let script = Script::default();
        let stop = CancellationToken::new();
        script.stop_after("a", &stop);
        let first = step(1, "a").with_on_failure(OnFailure::Rollback);
        let plan = Plan::sample(vec![first, step(2, "b")], vec![step(1, "undo")]);

        let (report, _) = run_with(&plan, &script, stop).await;

        assert_eq!(report["state"], json!("stopped"));
        assert_eq!(script.calls(), ["a"]);
        assert_eq!(report["rollback"], json!([]));
    }

    #[tokio::test]
    async fn the_summary_keeps_outcomes_and_leaves_out_values_and_errors() {
        let script = Script::default();
        script.answer("a", [Ok(json!({ "exit": 1, "stdout": "secret output" }))]);
        let plan = Plan::sample(vec![step(1, "a")], Vec::new());
        let run = Run::new(
            script.clone(),
            Heard::default(),
            CancellationToken::new(),
            40,
        );

        let report = run.perform(&plan).await;

        assert_eq!(
            report.summary(),
            json!({
                "runbook": "@lab/deploy",
                "state": "failed",
                "steps": [{
                    "step": 1,
                    "action": "ssh.exec",
                    "target": null,
                    "state": "failed",
                    "attempts": 1,
                    "request": 101,
                }],
                "rollback": [],
            })
        );
        assert_eq!(
            serde_json::to_value(&report).unwrap()["steps"][0]["value"],
            json!({ "exit": 1, "stdout": "secret output" })
        );
    }
}
