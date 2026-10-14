use std::time::Duration;

use synt_proto::{RunPhase, RunbookStep, StepState};
use tokio::time;
use tokio_util::sync::CancellationToken;

use super::{Progress, RunReport, RunState, StepCaller, StepReport};
use crate::runbooks::document::OnFailure;
use crate::runbooks::plan::{Plan, PlannedStep};

pub(crate) struct Runner<C, P> {
    caller: C,
    progress: P,
    stop: CancellationToken,
    run: u64,
}

impl<C: StepCaller, P: Progress> Runner<C, P> {
    pub(crate) fn new(caller: C, progress: P, stop: CancellationToken, run: u64) -> Self {
        Self {
            caller,
            progress,
            stop,
            run,
        }
    }

    pub(crate) async fn run(&self, plan: &Plan) -> RunReport {
        let mut report = RunReport::new(self.run, plan);

        report.state = self.run_steps(plan, &mut report.steps).await;

        if report.state == RunState::RolledBack {
            let (rollback, stopped) = self.roll_back(plan).await;
            report.rollback = rollback;

            if stopped {
                report.state = RunState::Stopped;
            }
        }

        report
    }

    async fn run_steps(&self, plan: &Plan, reports: &mut [StepReport]) -> RunState {
        for (step, report) in plan.steps.iter().zip(reports) {
            if self.stop.is_cancelled() {
                return RunState::Stopped;
            }

            *report = self.perform(plan, RunPhase::Steps, step).await;

            if self.stop.is_cancelled() {
                return RunState::Stopped;
            }

            if !report.failed() {
                continue;
            }

            match step.on_failure {
                OnFailure::Continue => {}
                OnFailure::Stop => return RunState::Failed,
                OnFailure::Rollback => return RunState::RolledBack,
            }
        }

        RunState::Completed
    }

    async fn roll_back(&self, plan: &Plan) -> (Vec<StepReport>, bool) {
        let mut reports = Vec::with_capacity(plan.rollback.len());

        for step in plan.rollback.iter().rev() {
            if self.stop.is_cancelled() {
                reports.push(StepReport::skipped(step));
                continue;
            }

            reports.push(self.perform(plan, RunPhase::Rollback, step).await);
        }

        (reports, self.stop.is_cancelled())
    }

    async fn perform(&self, plan: &Plan, phase: RunPhase, step: &PlannedStep) -> StepReport {
        let mut report = StepReport::skipped(step);
        let attempts = step.retry.attempts;

        for attempt in 1..=attempts {
            if attempt > 1 && !self.pause(step.retry.pause()).await {
                break;
            }

            report.attempts = attempt;
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

        self.announce(plan, phase, &report, report.state).await;

        report
    }

    async fn pause(&self, pause: Duration) -> bool {
        tokio::select! {
            () = self.stop.cancelled() => false,
            () = time::sleep(pause) => true,
        }
    }

    async fn announce(&self, plan: &Plan, phase: RunPhase, report: &StepReport, state: StepState) {
        let notice = RunbookStep {
            run: self.run,
            runbook: plan.runbook.clone(),
            phase,
            step: report.step,
            title: report.title.clone(),
            action: report.action.clone(),
            state,
            attempt: report.attempts,
            request: report.request.filter(|_| state != StepState::Started),
            error: report.error.clone().filter(|_| state != StepState::Started),
        };

        self.progress.report(notice).await;
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, VecDeque};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use serde_json::{Value, json};
    use synt_actions::Risk;
    use synt_journal::ActionId;
    use synt_proto::{
        ActionOutcome, ErrorCode, RequestSeq, RpcError, RunPhase, RunbookStep, StepState,
    };
    use tokio_util::sync::CancellationToken;

    use super::Runner;
    use crate::runbooks::document::{Expect, OnFailure, Retry};
    use crate::runbooks::param::Values;
    use crate::runbooks::plan::{Plan, PlannedStep};
    use crate::runbooks::run::{Progress, RunState, StepCaller};

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
            calls.push(step.title.clone());
            let request = calls.len() as u64 + 100;
            drop(calls);

            if let Some((title, stop)) = self.stop_after.lock().unwrap().as_ref()
                && *title == step.title
            {
                stop.cancel();
            }

            let answer = self
                .answers
                .lock()
                .unwrap()
                .get_mut(&step.title)
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
        PlannedStep {
            number,
            title: title.to_owned(),
            action: ActionId::new("ssh.exec").unwrap(),
            target: None,
            params: json!({}),
            risk: Risk::Write,
            expect: Expect::default(),
            retry: Retry::ONCE,
            on_failure: OnFailure::Stop,
        }
    }

    fn plan(steps: Vec<PlannedStep>, rollback: Vec<PlannedStep>) -> Plan {
        Plan {
            runbook: "@lab/deploy".to_owned(),
            name: "Deploy".to_owned(),
            values: Values::default(),
            steps,
            rollback,
        }
    }

    async fn run(plan: &Plan, script: &Script) -> (super::RunReport, Heard) {
        run_with(plan, script, CancellationToken::new()).await
    }

    async fn run_with(
        plan: &Plan,
        script: &Script,
        stop: CancellationToken,
    ) -> (super::RunReport, Heard) {
        let heard = Heard::default();
        let runner = Runner::new(script.clone(), heard.clone(), stop, 40);

        (runner.run(plan).await, heard)
    }

    fn states(report: &[super::StepReport]) -> Vec<StepState> {
        report.iter().map(|step| step.state).collect()
    }

    #[tokio::test]
    async fn steps_run_in_order_and_the_run_completes() {
        let script = Script::default();
        let plan = plan(vec![step(1, "a"), step(2, "b")], Vec::new());

        let (report, heard) = run(&plan, &script).await;

        assert_eq!(report.state, RunState::Completed);
        assert_eq!(script.calls(), ["a", "b"]);
        assert_eq!(report.steps[1].request, Some(102));
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
        let plan = plan(vec![step(1, "a"), step(2, "b")], Vec::new());

        let (report, _) = run(&plan, &script).await;

        assert_eq!(report.state, RunState::Failed);
        assert_eq!(
            states(&report.steps),
            [StepState::Failed, StepState::Skipped]
        );
        assert_eq!(report.steps[0].error.as_deref(), Some("the disk is full"));
        assert_eq!(script.calls(), ["a"]);
    }

    #[tokio::test]
    async fn an_unmet_expectation_fails_the_step_with_its_reason() {
        let script = Script::default();
        script.answer("a", [Ok(json!({ "exit": 2, "stdout": "" }))]);
        let plan = plan(vec![step(1, "a")], Vec::new());

        let (report, _) = run(&plan, &script).await;

        assert_eq!(report.steps[0].state, StepState::Failed);
        assert_eq!(report.steps[0].error.as_deref(), Some("exit 2, expected 0"));
        assert_eq!(report.steps[0].request, Some(101));
    }

    #[tokio::test]
    async fn a_step_that_may_fail_lets_the_run_go_on() {
        let script = Script::default();
        script.answer("a", [Err("no such unit")]);
        let mut first = step(1, "a");
        first.on_failure = OnFailure::Continue;
        let plan = plan(vec![first, step(2, "b")], Vec::new());

        let (report, _) = run(&plan, &script).await;

        assert_eq!(report.state, RunState::Completed);
        assert_eq!(
            states(&report.steps),
            [StepState::Failed, StepState::Succeeded]
        );
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
        let mut health = step(1, "health");
        health.retry = Retry {
            attempts: 5,
            every_seconds: 3,
        };
        let plan = plan(vec![health], Vec::new());

        let started = tokio::time::Instant::now();
        let (report, heard) = run(&plan, &script).await;

        assert_eq!(report.steps[0].state, StepState::Succeeded);
        assert_eq!(report.steps[0].attempts, 3);
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
        let mut health = step(1, "health");
        health.retry = Retry {
            attempts: 2,
            every_seconds: 1,
        };
        let plan = plan(vec![health], Vec::new());

        let (report, heard) = run(&plan, &script).await;

        assert_eq!(report.state, RunState::Failed);
        assert_eq!(report.steps[0].attempts, 2);
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
        let mut migrate = step(2, "migrate");
        migrate.on_failure = OnFailure::Rollback;
        let plan = plan(
            vec![step(1, "copy"), migrate, step(3, "restart")],
            vec![step(1, "undo copy"), step(2, "undo migrate")],
        );

        let (report, heard) = run(&plan, &script).await;

        assert_eq!(report.state, RunState::RolledBack);
        assert_eq!(
            script.calls(),
            ["copy", "migrate", "undo migrate", "undo copy"]
        );
        assert_eq!(
            states(&report.steps),
            [StepState::Succeeded, StepState::Failed, StepState::Skipped]
        );
        assert_eq!(
            states(&report.rollback),
            [StepState::Succeeded, StepState::Failed]
        );
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
        let mut first = step(1, "a");
        first.on_failure = OnFailure::Rollback;
        let plan = plan(vec![first, step(2, "b")], vec![step(1, "undo")]);

        let (report, _) = run_with(&plan, &script, stop).await;

        assert_eq!(report.state, RunState::Stopped);
        assert_eq!(script.calls(), ["a"]);
        assert!(report.rollback.is_empty());
    }

    #[tokio::test]
    async fn the_summary_keeps_outcomes_and_leaves_out_values_and_errors() {
        let script = Script::default();
        script.answer("a", [Ok(json!({ "exit": 1, "stdout": "secret output" }))]);
        let plan = plan(vec![step(1, "a")], Vec::new());

        let (report, _) = run(&plan, &script).await;
        let summary = report.summary();

        assert_eq!(
            summary,
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
            report.steps[0].value,
            Some(json!({ "exit": 1, "stdout": "secret output" }))
        );
    }
}
