#![allow(clippy::unwrap_used)]

mod common;

use common::{RunbookDesk, eventually, kinds, rejection};
use serde_json::{Value, json};
use synt_journal::{Actor, Event, EventKind, Filter, Name, Seq};
use synt_proto::{
    ActionCall, ApprovalDecision, ErrorCode, RequestSeq, Risk, RunPhase, RunbookStep, StepState,
};

const DEPLOY: &str = r#"
name = "Deploy"

[[params]]
name = "host"
kind = "target"
default = "web-1"

[[params]]
name = "tag"
kind = "word"

[[steps]]
title = "Pull {tag}"
action = "ssh.exec"
target = "{host}"
params = { command = "docker pull app:{tag}" }

[[steps]]
title = "Count the deploy"
action = "test.touch"
"#;

const BREAKING: &str = r#"
name = "Breaking"

[[steps]]
title = "Count"
action = "test.touch"

[[steps]]
title = "Break"
action = "test.fail"
on_failure = "rollback"

[[steps]]
title = "Never reached"
action = "test.touch"

[[rollback]]
title = "Clean up"
action = "ssh.exec"
target = "web-1"
params = { command = "rm -rf /srv/app/next" }

[[rollback]]
title = "Count the cleanup"
action = "test.touch"
"#;

const ESCALATING: &str = r#"
name = "Escalating"

[[steps]]
title = "Restart as root"
action = "ssh.exec"
target = "web-1"
params = { command = "sudo reboot", privileged = true }
"#;

const WAITING: &str = r#"
name = "Waiting"

[[steps]]
title = "Wait"
action = "test.wait"

[[steps]]
title = "After the wait"
action = "test.touch"
"#;

const AIMING: &str = r#"
name = "Aiming"

[[steps]]
title = "Aim"
action = "test.aim"
target = "web-1"
"#;

fn runbook_actor(name: &str) -> Actor {
    Actor::Runbook(Name::new(RunbookDesk::reference(name)).unwrap())
}

fn step_requests(desk: &RunbookDesk, run: RequestSeq) -> Vec<Event> {
    desk.harness()
        .matching(&Filter::all().kind(EventKind::Requested))
        .into_iter()
        .filter(|event| event.details().value()["parent"] == json!(run.get()))
        .collect()
}

fn finished(desk: &RunbookDesk, request: RequestSeq) -> Value {
    let trail = desk.harness().trail(request);
    let last = trail.last().unwrap();

    assert_eq!(last.kind(), EventKind::Finished);
    last.details().value().clone()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_run_approved_up_front_journals_each_step_under_the_runbook() {
    let desk = RunbookDesk::start().await;
    desk.write("deploy", DEPLOY);
    let mut approver = desk.harness().approver().await;
    let app = approver.app().clone();

    let outcome = desk
        .run(&app, "deploy", &json!({ "tag": "v7" }), true)
        .await
        .unwrap();
    let steps = step_requests(&desk, outcome.request);

    assert_eq!(outcome.value["state"], json!("completed"));
    assert!(approver.was_not_asked());
    assert_eq!(desk.harness().touches(), 1);
    assert_eq!(steps.len(), 2);
    assert!(
        steps
            .iter()
            .all(|step| step.actor() == &runbook_actor("deploy"))
    );
    assert_eq!(steps[0].action().as_str(), "ssh.exec");
    assert_eq!(steps[0].resource().unwrap().as_str(), "web-1");
    assert_eq!(
        steps[0].details().value(),
        &json!({ "params": { "command": "docker pull app:v7" }, "parent": outcome.request.get() })
    );

    let pulled = desk.harness().matching(&Filter::all().call(steps[0].seq()));
    assert_eq!(
        kinds(&pulled),
        [
            EventKind::Requested,
            EventKind::Approved,
            EventKind::Started,
            EventKind::Finished
        ]
    );
    assert_eq!(
        pulled[1].details().value(),
        &json!({ "scope": "run", "by": "person" })
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn without_approval_up_front_every_write_step_asks_the_person() {
    let desk = RunbookDesk::start().await;
    desk.write("deploy", DEPLOY);
    let mut approver = desk.harness().approver().await;
    let app = approver.app().clone();

    let running = tokio::spawn({
        let app = app.clone();
        async move {
            app.call(RunbookDesk::run_call(
                "deploy",
                &json!({ "tag": "v7" }),
                false,
            ))
            .await
        }
    });

    let pull = approver.next().await;
    approver
        .answer(&pull, ApprovalDecision::AllowOnce)
        .await
        .unwrap();
    let count = approver.next().await;
    approver
        .answer(&count, ApprovalDecision::AllowOnce)
        .await
        .unwrap();
    let outcome = running.await.unwrap().unwrap();

    assert_eq!(
        pull.actor,
        Actor::Runbook(Name::new(RunbookDesk::reference("deploy")).unwrap())
    );
    assert_eq!(pull.on_behalf_of, Some(Actor::Person));
    assert_eq!(pull.action, "ssh.exec");
    assert_eq!(pull.params, json!({ "command": "docker pull app:v7" }));
    assert_eq!(count.action, "test.touch");
    assert_eq!(outcome.value["state"], json!("completed"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_privileged_step_asks_every_time_even_in_a_run_approved_up_front() {
    let desk = RunbookDesk::start().await;
    desk.write("escalating", ESCALATING);
    let mut approver = desk.harness().approver().await;
    let app = approver.app().clone();

    let (question, outcome) = approver
        .decide(
            &app,
            RunbookDesk::run_call("escalating", &json!({}), true),
            ApprovalDecision::AllowUntilRestart,
        )
        .await;
    let outcome = outcome.unwrap();
    let step = &step_requests(&desk, outcome.request)[0];
    let approval = &desk.harness().matching(&Filter::all().call(step.seq()))[1];

    assert_eq!(question.risk, Risk::Privileged);
    assert_eq!(
        question.actor,
        Actor::Runbook(Name::new(RunbookDesk::reference("escalating")).unwrap())
    );
    assert_eq!(
        approval.details().value(),
        &json!({ "scope": "once", "by": "person" })
    );
    assert_eq!(outcome.value["state"], json!("completed"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_failing_step_rolls_back_in_reverse_and_skips_the_rest() {
    let desk = RunbookDesk::start().await;
    desk.write("breaking", BREAKING);
    let app = desk.harness().app().await;
    let mut notifications = app.notifications();

    let outcome = desk.run(&app, "breaking", &json!({}), true).await.unwrap();
    let report = &outcome.value;
    let steps = step_requests(&desk, outcome.request);
    let actions: Vec<&str> = steps.iter().map(|step| step.action().as_str()).collect();

    assert_eq!(report["state"], json!("rolled_back"));
    assert_eq!(
        report["steps"]
            .as_array()
            .unwrap()
            .iter()
            .map(|step| step["state"].clone())
            .collect::<Vec<_>>(),
        [json!("succeeded"), json!("failed"), json!("skipped")]
    );
    assert_eq!(report["steps"][1]["error"], json!("the disk is full"));
    assert_eq!(report["rollback"][0]["title"], json!("Count the cleanup"));
    assert_eq!(report["rollback"][1]["title"], json!("Clean up"));
    assert_eq!(
        actions,
        ["test.touch", "test.fail", "test.touch", "ssh.exec"]
    );
    assert_eq!(desk.harness().touches(), 2);

    let heard = RunbookDesk::hear(&mut notifications, 8).await;
    let rollback: Vec<(usize, StepState)> = heard
        .iter()
        .filter(|step| step.phase == RunPhase::Rollback)
        .map(|step| (step.step, step.state))
        .collect();
    assert_eq!(
        rollback,
        [
            (2, StepState::Started),
            (2, StepState::Succeeded),
            (1, StepState::Started),
            (1, StepState::Succeeded)
        ]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_finished_run_lists_every_outcome_without_output() {
    let desk = RunbookDesk::start().await;
    desk.write("breaking", BREAKING);
    let app = desk.harness().app().await;

    let outcome = desk.run(&app, "breaking", &json!({}), true).await.unwrap();
    let summary = finished(&desk, outcome.request);

    assert_eq!(
        summary["runbook"],
        json!(RunbookDesk::reference("breaking"))
    );
    assert_eq!(summary["state"], json!("rolled_back"));
    assert_eq!(
        summary["steps"][1],
        json!({
            "step": 2,
            "action": "test.fail",
            "target": null,
            "state": "failed",
            "attempts": 1,
            "request": null,
        })
    );
    assert_eq!(summary["rollback"].as_array().unwrap().len(), 2);
    assert!(!summary.to_string().contains("disk is full"));
}

#[tokio::test(flavor = "multi_thread")]
async fn progress_reaches_the_caller_step_by_step() {
    let desk = RunbookDesk::start().await;
    desk.write("deploy", DEPLOY);
    let app = desk.harness().app().await;
    let mut notifications = app.notifications();

    let outcome = desk
        .run(&app, "deploy", &json!({ "tag": "v7" }), true)
        .await
        .unwrap();
    let heard: Vec<RunbookStep> = RunbookDesk::hear(&mut notifications, 4).await;

    assert!(heard.iter().all(|step| step.run == outcome.request.get()));
    assert_eq!(
        heard
            .iter()
            .map(|step| (step.step, step.state))
            .collect::<Vec<_>>(),
        [
            (1, StepState::Started),
            (1, StepState::Succeeded),
            (2, StepState::Started),
            (2, StepState::Succeeded),
        ]
    );
    assert_eq!(heard[0].title, "Pull v7");
    assert_eq!(
        heard[1].request,
        outcome.value["steps"][0]["request"].as_u64()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_asks_once_for_the_whole_run_and_shows_its_steps() {
    let desk = RunbookDesk::start().await;
    desk.write("deploy", DEPLOY);
    let mut approver = desk.harness().approver().await;
    let agent = desk.harness().agent("deploy-bot").await;

    let (question, outcome) = approver
        .decide(
            &agent,
            RunbookDesk::run_call("deploy", &json!({ "tag": "v8" }), true),
            ApprovalDecision::AllowOnce,
        )
        .await;

    assert_eq!(question.action, "runbook.run");
    assert_eq!(question.risk, Risk::Write);
    assert_eq!(question.destination, Some(RunbookDesk::reference("deploy")));
    assert_eq!(
        question.params["steps"][0]["params"],
        json!({ "command": "docker pull app:v8" })
    );
    assert_eq!(question.params["steps"][0]["risk"], json!("write"));
    assert_eq!(outcome.unwrap().value["state"], json!("completed"));
    assert!(approver.was_not_asked());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_standing_grant_never_approves_the_steps_of_a_later_run() {
    let desk = RunbookDesk::start().await;
    desk.write("deploy", DEPLOY);
    let mut approver = desk.harness().approver().await;
    let agent = desk.harness().agent("deploy-bot").await;
    let call = || RunbookDesk::run_call("deploy", &json!({ "tag": "v8" }), true);

    let (_, first) = approver
        .decide(&agent, call(), ApprovalDecision::AllowUntilRestart)
        .await;
    desk.write(
        "deploy",
        &DEPLOY.replace("docker pull", "rm -rf /srv; docker pull"),
    );
    let (question, second) = approver
        .decide(&agent, call(), ApprovalDecision::AllowOnce)
        .await;

    assert_eq!(first.unwrap().value["state"], json!("completed"));
    assert_eq!(
        question.params["steps"][0]["params"],
        json!({ "command": "rm -rf /srv; docker pull app:v8" })
    );
    assert_eq!(second.unwrap().value["state"], json!("completed"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_step_that_moved_after_the_run_was_approved_is_asked_again() {
    let desk = RunbookDesk::start().await;
    desk.write("aiming", AIMING);
    let mut approver = desk.harness().approver().await;
    let agent = desk.harness().agent("deploy-bot").await;

    let running = tokio::spawn({
        let agent = agent.clone();
        async move {
            agent
                .call(RunbookDesk::run_call("aiming", &json!({}), true))
                .await
        }
    });
    let run_question = approver.next().await;
    desk.harness().grounds().aim_at("10.6.6.6");
    approver
        .answer(&run_question, ApprovalDecision::AllowOnce)
        .await
        .unwrap();
    let step_question = approver.next().await;
    approver
        .answer(&step_question, ApprovalDecision::AllowOnce)
        .await
        .unwrap();
    let outcome = running.await.unwrap().unwrap();

    assert_eq!(
        run_question.params["steps"][0]["destination"],
        json!("web-1 = 10.0.0.1")
    );
    assert_eq!(step_question.action, "test.aim");
    assert_eq!(
        step_question.destination.as_deref(),
        Some("web-1 = 10.6.6.6")
    );
    assert_eq!(outcome.value["state"], json!("completed"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_run_can_be_stopped_while_a_step_is_running() {
    let desk = RunbookDesk::start().await;
    desk.write("waiting", WAITING);
    let app = desk.harness().app().await;
    let mut notifications = app.notifications();

    let running = tokio::spawn({
        let app = app.clone();
        async move {
            app.call(RunbookDesk::run_call("waiting", &json!({}), true))
                .await
        }
    });
    let run = eventually("the first step starts", async || {
        RunbookDesk::steps_heard(&mut notifications)
            .first()
            .map(|step| step.run)
    })
    .await;
    eventually("the wait begins", async || {
        (desk.harness().grounds().waits() > 0).then_some(())
    })
    .await;

    let stopped = app
        .call(ActionCall::new("runbook.stop").params(json!({ "run": run })))
        .await
        .unwrap();
    let outcome = running.await.unwrap().unwrap();

    assert_eq!(
        stopped.value,
        json!({ "run": run, "runbook": RunbookDesk::reference("waiting") })
    );
    assert_eq!(outcome.value["state"], json!("stopped"));
    assert_eq!(
        outcome.value["steps"][0]["error"],
        json!("stopped: the work this call belongs to was stopped")
    );
    assert_eq!(outcome.value["steps"][1]["state"], json!("skipped"));
    assert_eq!(desk.harness().touches(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn stopping_an_unknown_run_fails() {
    let desk = RunbookDesk::start().await;
    let app = desk.harness().app().await;

    let error = rejection(
        app.call(ActionCall::new("runbook.stop").params(json!({ "run": 999 })))
            .await,
    );

    assert_eq!(error.code, ErrorCode::ACTION_FAILED);
    assert_eq!(error.message, "there is no running run 999");
}

#[tokio::test(flavor = "multi_thread")]
async fn params_are_checked_before_any_step_runs() {
    let desk = RunbookDesk::start().await;
    desk.write("deploy", DEPLOY);
    let app = desk.harness().app().await;

    let missing = rejection(desk.run(&app, "deploy", &json!({}), true).await);
    let malformed = rejection(
        desk.run(&app, "deploy", &json!({ "tag": "v1; reboot" }), true)
            .await,
    );
    let unknown = rejection(
        desk.run(&app, "deploy", &json!({ "tag": "v1", "force": true }), true)
            .await,
    );

    assert_eq!(missing.code, ErrorCode::INVALID_PARAMS);
    assert_eq!(
        missing.message,
        "invalid parameters: runbook deploy: param tag is required"
    );
    assert!(
        malformed
            .message
            .contains("param tag: \"v1; reboot\" is not a word"),
        "{}",
        malformed.message
    );
    assert!(unknown.message.ends_with("the runbook has no param force"));
    assert_eq!(desk.harness().touches(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_runbook_that_calls_a_missing_action_is_refused_whole() {
    let desk = RunbookDesk::start().await;
    desk.write(
        "typo",
        "name = \"Typo\"\n[[steps]]\ntitle = \"a\"\naction = \"test.touch\"\n[[steps]]\ntitle = \"b\"\naction = \"test.tuch\"\n",
    );
    let app = desk.harness().app().await;

    let error = rejection(desk.run(&app, "typo", &json!({}), true).await);

    assert_eq!(
        error.message,
        "invalid parameters: runbook typo: step 2: there is no action test.tuch"
    );
    assert_eq!(desk.harness().touches(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_runbook_is_put_listed_read_and_exported() {
    let desk = RunbookDesk::start().await;
    let app = desk.harness().app().await;
    let runbook: Value = json!({
        "name": "Deploy",
        "params": [{ "name": "tag", "kind": "word", "default": "v1" }],
        "steps": [{
            "title": "Pull",
            "action": "ssh.exec",
            "target": "web-1",
            "params": { "command": "docker pull app:{tag}" },
        }],
    });

    let put = app
        .call(ActionCall::new("runbook.put").params(json!({
            "project": RunbookDesk::PROJECT,
            "name": "deploy",
            "runbook": runbook,
        })))
        .await
        .unwrap();
    let listed = app
        .call(ActionCall::new("runbook.list").params(json!({ "project": RunbookDesk::PROJECT })))
        .await
        .unwrap();
    let read = app
        .call(
            ActionCall::new("runbook.get")
                .params(json!({ "project": RunbookDesk::PROJECT, "name": "deploy" })),
        )
        .await
        .unwrap();
    let exported = app
        .call(
            ActionCall::new("runbook.export")
                .params(json!({ "project": RunbookDesk::PROJECT, "name": "deploy" })),
        )
        .await
        .unwrap();

    assert_eq!(put.value["runbook"], runbook);
    assert!(desk.file("deploy").contains("docker pull app:{tag}"));
    assert_eq!(listed.value["runbooks"][0]["name"], json!("deploy"));
    assert_eq!(listed.value["runbooks"][0]["steps"], json!(1));
    assert_eq!(read.value["runbook"], runbook);
    assert_eq!(
        exported.value["runbook"],
        json!(RunbookDesk::reference("deploy"))
    );
    assert!(
        exported.value["script"]
            .as_str()
            .unwrap()
            .contains("ssh -- 'web-1' 'docker pull app:'\"${tag}\" || exit 1")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_is_asked_before_it_writes_a_runbook() {
    let desk = RunbookDesk::start().await;
    let mut approver = desk.harness().approver().await;
    let agent = desk.harness().agent("deploy-bot").await;
    let call = ActionCall::new("runbook.put").params(json!({
        "project": RunbookDesk::PROJECT,
        "name": "noop",
        "runbook": { "name": "Noop", "steps": [{ "title": "t", "action": "test.touch" }] },
    }));

    let (question, outcome) = approver.decide(&agent, call, ApprovalDecision::Deny).await;

    assert_eq!(question.destination, Some(RunbookDesk::reference("noop")));
    assert_eq!(rejection(outcome).code, ErrorCode::DENIED);
}

#[tokio::test(flavor = "multi_thread")]
async fn broken_runbook_files_are_listed_as_problems() {
    let desk = RunbookDesk::start().await;
    desk.write("broken", "name = \"Broken\"\n");
    desk.write("deploy", DEPLOY);
    let app = desk.harness().app().await;

    let listed = app
        .call(ActionCall::new("runbook.list").params(json!({ "project": RunbookDesk::PROJECT })))
        .await
        .unwrap();

    assert_eq!(listed.value["runbooks"].as_array().unwrap().len(), 1);
    assert_eq!(
        listed.value["problems"][0]["file"],
        json!("runbooks/broken.toml")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn calls_in_the_journal_become_a_runbook_with_their_shared_values_as_params() {
    let desk = RunbookDesk::start().await;
    let app = desk.harness().app().await;
    let exec = |command: &str| {
        ActionCall::new("ssh.exec")
            .target("web-1")
            .params(json!({ "command": command }))
    };

    let uptime = app.call(exec("uptime")).await.unwrap();
    let disks = app.call(exec("df -h")).await.unwrap();
    let disks_finished = desk.harness().trail(disks.request).last().unwrap().seq();

    let drafted = app
        .call(ActionCall::new("runbook.from_journal").params(json!({
            "events": [uptime.request.get(), disks_finished.get(), uptime.request.get()],
            "name": "Look around",
        })))
        .await
        .unwrap();
    let runbook = &drafted.value["runbook"];

    assert_eq!(runbook["name"], json!("Look around"));
    assert_eq!(
        runbook["params"],
        json!([{ "name": "host", "kind": "target", "default": "web-1" }])
    );
    assert_eq!(
        runbook["steps"][0],
        json!({
            "title": "uptime",
            "action": "ssh.exec",
            "target": "{host}",
            "params": { "command": "uptime" },
        })
    );
    assert_eq!(runbook["steps"][1]["params"], json!({ "command": "df -h" }));
    assert_eq!(runbook["steps"].as_array().unwrap().len(), 2);
    assert!(
        drafted.value["toml"]
            .as_str()
            .unwrap()
            .contains("target = \"{host}\"")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_draft_from_an_unknown_event_is_refused() {
    let desk = RunbookDesk::start().await;
    let app = desk.harness().app().await;

    let error = rejection(
        app.call(
            ActionCall::new("runbook.from_journal")
                .params(json!({ "events": [Seq::new(9_999).get()] })),
        )
        .await,
    );

    assert_eq!(error.code, ErrorCode::INVALID_PARAMS);
    assert!(
        error.message.ends_with("the journal has no event #9999"),
        "{}",
        error.message
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn runbook_actions_describe_their_risk_and_params() {
    let desk = RunbookDesk::start().await;
    let agent = desk.harness().agent("deploy-bot").await;

    let described = agent.describe().await.unwrap();
    let find = |id: &str| described.iter().find(|action| action.id == id);
    let run = find("runbook.run").unwrap();

    assert_eq!(run.title, "Run a runbook step by step");
    assert_eq!(run.risk, Risk::Read);
    assert_eq!(run.params["required"], json!(["project", "name"]));
    assert!(run.params["properties"]["approve_steps"].is_object());
    assert!(run.params["properties"].get("plan").is_none());
    assert_eq!(find("runbook.put").unwrap().risk, Risk::Write);
    assert_eq!(find("runbook.stop").unwrap().risk, Risk::Write);
    assert!(find("runbook.export").is_some());
    assert!(find("runbook.from_journal").is_some());
    assert!(find("runbook.from_blocks").is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn commands_of_a_local_shell_cannot_become_runbook_steps() {
    let desk = RunbookDesk::start().await;
    let app = desk.harness().app().await;
    let opening = ActionCall::new("local.session.open").params(json!({ "cols": 80, "rows": 24 }));
    let session = app.call(opening).await.unwrap().value["session"].clone();

    let drafting = |session: Value| {
        ActionCall::new("runbook.from_blocks").params(json!({ "session": session, "blocks": [1] }))
    };
    let local = rejection(app.call(drafting(session.clone())).await);
    let missing = rejection(app.call(drafting(json!(4_242))).await);

    assert_eq!(local.code, ErrorCode::INVALID_PARAMS);
    assert!(
        local
            .message
            .ends_with("is a local shell, so its commands have no host to run on")
    );
    assert_eq!(missing.code, ErrorCode::ACTION_FAILED);
    assert_eq!(missing.message, "there is no open session 4242");

    let closing = ActionCall::new("session.close").params(json!({ "session": session }));
    app.call(closing).await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn only_the_person_drafts_from_terminal_blocks() {
    let desk = RunbookDesk::start().await;
    let agent = desk.harness().agent("deploy-bot").await;
    let drafting =
        ActionCall::new("runbook.from_blocks").params(json!({ "session": 1, "blocks": [1] }));

    let error = rejection(agent.call(drafting).await);

    assert_eq!(error.code, ErrorCode::DENIED);
}
