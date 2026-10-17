use serde_json::{Value, json};
use synt_actions::Action;
use synt_core::{Broker, Core, GrantList, GrantRevoke};
use synt_journal::{EventKind, Filter, Seq};
use synt_proto::{ActionCall, ApprovalDecision, ErrorCode, RequestSeq};
use synt_testkit::TestError;
use synt_testkit::core::{Grounds, Guarded, Harness, Waiting, eventually, kinds, rejection};

fn core(grounds: &Grounds) -> Result<Core, TestError> {
    let always = grounds.always()?;
    let mut registry = grounds.registry()?;

    registry.register(GrantList::new(always.clone()))?;
    registry.register(GrantRevoke::new(always.clone(), grounds.journal()))?;

    Ok(Core::new(
        registry,
        grounds.journal(),
        Broker::with_always(always),
    ))
}

fn start() -> Result<Harness, TestError> {
    let grounds = Grounds::new()?;
    let core = core(&grounds)?;

    Harness::serve(grounds, core)
}

fn approval(harness: &Harness, request: RequestSeq) -> Result<Value, TestError> {
    let trail = harness.trail(request)?;
    let Some(approved) = trail
        .iter()
        .find(|event| event.kind() == EventKind::Approved)
    else {
        panic!("the call was not approved");
    };

    Ok(approved.details().value().clone())
}

fn revoke(id: u64) -> ActionCall {
    ActionCall::new(GrantRevoke::ID).params(json!({ "id": id }))
}

async fn grants(harness: &Harness) -> Result<Value, TestError> {
    let app = harness.app().await?;

    Ok(app.call(ActionCall::new(GrantList::ID)).await?.value)
}

#[tokio::test(flavor = "multi_thread")]
async fn allowed_once_is_asked_again_next_time() -> Result<(), TestError> {
    let harness = start()?;
    let mut approver = harness.approver().await?;
    let agent = harness.agent("deploy-bot").await?;

    approver
        .approve(&agent, Guarded::call("write"), ApprovalDecision::AllowOnce)
        .await?;
    let (again, outcome) = approver
        .decide(&agent, Guarded::call("write"), ApprovalDecision::AllowOnce)
        .await?;

    assert_eq!(again.action, Guarded::ID);
    assert!(outcome.is_ok());
    assert_eq!(harness.runs(), 2);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn allowed_until_restart_repeats_without_asking_and_is_journaled_each_time()
-> Result<(), TestError> {
    let harness = start()?;
    let mut approver = harness.approver().await?;
    let agent = harness.agent("deploy-bot").await?;

    approver
        .approve(
            &agent,
            Guarded::call("write"),
            ApprovalDecision::AllowUntilRestart,
        )
        .await?;
    let repeated = agent.call(Guarded::call("write")).await.unwrap();

    assert!(approver.was_not_asked());
    assert_eq!(harness.runs(), 2);
    assert_eq!(
        approval(&harness, repeated.request)?,
        json!({ "scope": "until_restart", "by": "person" })
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_grant_covers_only_its_actor_action_target_and_destination() -> Result<(), TestError> {
    let harness = start()?;
    let mut approver = harness.approver().await?;
    let agent = harness.agent("deploy-bot").await?;
    let other = harness.agent("reviewer").await?;

    approver
        .approve(
            &agent,
            Guarded::call_to("write", "db-1"),
            ApprovalDecision::AllowUntilRestart,
        )
        .await?;
    agent.call(Guarded::call_to("write", "db-1")).await.unwrap();
    assert!(approver.was_not_asked());

    let elsewhere = Guarded::call_to("write", "db-2");
    let other_target = Guarded::call_to("write", "db-1").target("api-1");
    let other_action = ActionCall::new(Waiting::ID);

    for (caller, call) in [
        (&other, Guarded::call_to("write", "db-1")),
        (&agent, elsewhere),
        (&agent, other_target),
        (&agent, other_action),
    ] {
        let (_, outcome) = approver
            .decide(caller, call, ApprovalDecision::Deny)
            .await?;
        assert_eq!(rejection(outcome).code, ErrorCode::DENIED);
    }
    assert_eq!(harness.runs(), 2);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_privileged_call_of_an_agent_is_approved_once_even_when_granted_until_restart()
-> Result<(), TestError> {
    let harness = start()?;
    let mut approver = harness.approver().await?;
    let agent = harness.agent("deploy-bot").await?;

    let (_, first) = approver
        .decide(
            &agent,
            Guarded::call("privileged"),
            ApprovalDecision::AllowUntilRestart,
        )
        .await?;
    let (again, _) = approver
        .decide(&agent, Guarded::call("privileged"), ApprovalDecision::Deny)
        .await?;

    assert_eq!(
        approval(&harness, first.unwrap().request)?,
        json!({ "scope": "once", "by": "person" })
    );
    assert_eq!(again.action, Guarded::ID);
    assert_eq!(harness.runs(), 1);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_privileged_call_of_an_agent_is_never_kept_always() -> Result<(), TestError> {
    let harness = start()?;
    let mut approver = harness.approver().await?;
    let agent = harness.agent("deploy-bot").await?;

    let (_, first) = approver
        .decide(
            &agent,
            Guarded::call("privileged"),
            ApprovalDecision::AllowAlways,
        )
        .await?;

    assert_eq!(
        approval(&harness, first.unwrap().request)?,
        json!({ "scope": "once", "by": "person" })
    );
    assert_eq!(grants(&harness).await?, json!([]));

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn the_person_may_grant_their_own_privileged_calls() -> Result<(), TestError> {
    let harness = start()?;
    let mut approver = harness.approver().await?;
    let app = approver.app().clone();

    approver
        .approve(
            &app,
            Guarded::call("privileged"),
            ApprovalDecision::AllowUntilRestart,
        )
        .await?;
    app.call(Guarded::call("privileged")).await.unwrap();

    assert!(approver.was_not_asked());
    assert_eq!(harness.runs(), 2);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn grants_until_restart_end_with_the_core() -> Result<(), TestError> {
    let harness = start()?;
    let mut approver = harness.approver().await?;
    let agent = harness.agent("deploy-bot").await?;
    approver
        .approve(
            &agent,
            Guarded::call("write"),
            ApprovalDecision::AllowUntilRestart,
        )
        .await?;

    let harness = harness.restart(core).await?;
    let mut approver = harness.approver().await?;
    let agent = harness.agent("deploy-bot").await?;
    let (asked, _) = approver
        .decide(&agent, Guarded::call("write"), ApprovalDecision::Deny)
        .await?;

    assert_eq!(asked.action, Guarded::ID);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn an_always_grant_is_listed_and_outlives_a_restart() -> Result<(), TestError> {
    let harness = start()?;
    let mut approver = harness.approver().await?;
    let agent = harness.agent("deploy-bot").await?;

    let (_, first) = approver
        .decide(
            &agent,
            Guarded::call_to("write", "db-1"),
            ApprovalDecision::AllowAlways,
        )
        .await?;
    let listed = grants(&harness).await?;

    let harness = harness.restart(core).await?;
    let mut approver = harness.approver().await?;
    let agent = harness.agent("deploy-bot").await?;
    let repeated = agent.call(Guarded::call_to("write", "db-1")).await.unwrap();

    let kept = json!({ "scope": "always", "grant": 1, "by": "person" });
    assert_eq!(approval(&harness, first.unwrap().request)?, kept);
    assert_eq!(approval(&harness, repeated.request)?, kept);
    assert!(approver.was_not_asked());
    assert_eq!(listed[0]["id"], json!(1));
    assert_eq!(
        listed[0]["actor"],
        json!({ "type": "agent", "name": "deploy-bot" })
    );
    assert_eq!(listed[0]["action"], json!(Guarded::ID));
    assert_eq!(listed[0]["destination"], json!("db-1"));
    assert_eq!(listed[0]["scope"], json!("always"));

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn grants_changed_outside_the_core_are_all_dropped() -> Result<(), TestError> {
    let harness = start()?;
    let mut approver = harness.approver().await?;
    let agent = harness.agent("deploy-bot").await?;
    approver
        .approve(
            &agent,
            Guarded::call("write"),
            ApprovalDecision::AllowAlways,
        )
        .await?;

    let tampered = rusqlite::Connection::open(harness.grounds().state_path()).unwrap();
    tampered
        .execute("UPDATE grants SET actor = 'agent:intruder'", [])
        .unwrap();

    let harness = harness.restart(core).await?;
    let mut approver = harness.approver().await?;
    let intruder = harness.agent("intruder").await?;
    let (asked, _) = approver
        .decide(&intruder, Guarded::call("write"), ApprovalDecision::Deny)
        .await?;

    assert_eq!(asked.action, Guarded::ID);
    assert_eq!(grants(&harness).await?, json!([]));

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn only_the_person_lists_and_revokes_grants() -> Result<(), TestError> {
    let harness = start()?;
    let agent = harness.agent("deploy-bot").await?;

    let listing = rejection(agent.call(ActionCall::new(GrantList::ID)).await);
    let revoking = rejection(agent.call(revoke(1)).await);

    assert_eq!(listing.code, ErrorCode::DENIED);
    assert_eq!(revoking.code, ErrorCode::DENIED);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn revoking_a_grant_stops_the_calls_that_use_it_and_asks_again() -> Result<(), TestError> {
    let harness = start()?;
    let mut approver = harness.approver().await?;
    let agent = harness.agent("deploy-bot").await?;

    let caller = agent.clone();
    let running = tokio::spawn(async move { caller.call(ActionCall::new(Waiting::ID)).await });
    let question = approver.next().await?;
    approver
        .answer(&question, ApprovalDecision::AllowAlways)
        .await
        .unwrap();
    eventually("the call to start", async || {
        (harness.grounds().waits() == 1).then_some(())
    })
    .await;

    let revoked = approver.app().call(revoke(1)).await.unwrap();
    let stopped = rejection(running.await.unwrap());
    let (asked_again, _) = approver
        .decide(&agent, ActionCall::new(Waiting::ID), ApprovalDecision::Deny)
        .await?;
    let notes = harness.matching(&Filter::all().kind(EventKind::Note))?;

    assert_eq!(revoked.value["stopped"], json!(1));
    assert_eq!(revoked.value["grant"]["id"], json!(1));
    assert_eq!(stopped.code, ErrorCode::ACTION_FAILED);
    assert_eq!(asked_again.action, Waiting::ID);
    assert_eq!(grants(&harness).await?, json!([]));
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].details().value()["revoked"], json!(1));
    assert_eq!(notes[0].call(), Some(Seq::new(revoked.request.get())));

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_call_stopped_by_a_revocation_is_journaled_as_failed() -> Result<(), TestError> {
    let harness = start()?;
    let mut approver = harness.approver().await?;
    let agent = harness.agent("deploy-bot").await?;

    let running = tokio::spawn(async move { agent.call(ActionCall::new(Waiting::ID)).await });
    let question = approver.next().await?;
    approver
        .answer(&question, ApprovalDecision::AllowAlways)
        .await
        .unwrap();
    eventually("the call to start", async || {
        (harness.grounds().waits() == 1).then_some(())
    })
    .await;
    approver.app().call(revoke(1)).await.unwrap();
    running.await.unwrap().unwrap_err();

    let failed = harness.matching(&Filter::all().kind(EventKind::Failed))?;

    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].action().as_str(), Waiting::ID);
    assert!(harness.is_intact()?);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn revoking_a_grant_that_does_not_exist_fails() -> Result<(), TestError> {
    let harness = start()?;
    let app = harness.app().await?;

    let error = rejection(app.call(revoke(7)).await);

    assert_eq!(error.code, ErrorCode::ACTION_FAILED);
    assert_eq!(error.message, "there is no grant 7");
    assert_eq!(
        kinds(&harness.events()?),
        [EventKind::Requested, EventKind::Started, EventKind::Failed]
    );

    Ok(())
}
