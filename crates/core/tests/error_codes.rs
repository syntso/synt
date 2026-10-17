use serde_json::json;
use synt_actions::Action;
use synt_journal::EventKind;
use synt_proto::{
    ActionCall, ApprovalDecision, ApprovalId, ErrorCode, Method, Outcome, ProtocolVersion,
    RequestId, RpcError,
};
use synt_testkit::TestError;
use synt_testkit::core::{
    Confused, Guarded, Harness, RawConnection, Touch, kinds, ping, rejection,
};

fn error(outcome: Outcome) -> Result<RpcError, TestError> {
    match outcome.into_result() {
        Err(error) => Ok(error),
        Ok(value) => Err(format!("expected an error, got {value:?}").into()),
    }
}

#[test]
fn the_codes_are_the_ones_the_protocol_documents() {
    let documented = [
        (ErrorCode::PARSE_ERROR, -32_700),
        (ErrorCode::INVALID_REQUEST, -32_600),
        (ErrorCode::METHOD_NOT_FOUND, -32_601),
        (ErrorCode::INVALID_PARAMS, -32_602),
        (ErrorCode::INTERNAL, -32_603),
        (ErrorCode::HANDSHAKE_REQUIRED, -32_000),
        (ErrorCode::PROTOCOL_MISMATCH, -32_001),
        (ErrorCode::UNKNOWN_ACTION, -32_010),
        (ErrorCode::ACTION_FAILED, -32_011),
        (ErrorCode::DENIED, -32_012),
        (ErrorCode::JOURNAL_UNAVAILABLE, -32_013),
        (ErrorCode::APPROVAL_UNKNOWN, -32_014),
    ];

    for (code, number) in documented {
        assert_eq!(code.get(), number);
        assert_eq!(serde_json::to_value(code).unwrap(), json!(number));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn parse_error_is_for_bytes_that_are_not_json() -> Result<(), TestError> {
    let harness = Harness::start()?;
    let mut connection = RawConnection::greeted(harness.endpoint()).await?;

    connection.send_bytes(&b"\xff\xfe"[..]).await?;

    assert_eq!(
        error(connection.receive().await?.outcome)?.code,
        ErrorCode::PARSE_ERROR
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_request_is_for_json_that_is_not_a_request() -> Result<(), TestError> {
    let harness = Harness::start()?;
    let mut connection = RawConnection::greeted(harness.endpoint()).await?;

    for body in [
        &b"[1, 2, 3]"[..],
        &br#"{"jsonrpc":"1.0","id":2,"method":"actions.describe"}"#[..],
    ] {
        connection.send_bytes(body).await?;
        let response = connection.receive().await?;

        assert_eq!(error(response.outcome)?.code, ErrorCode::INVALID_REQUEST);
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_request_is_for_a_second_hello() -> Result<(), TestError> {
    let harness = Harness::start()?;
    let mut connection = RawConnection::greeted(harness.endpoint()).await?;

    connection
        .hello(RequestId::new(2), &synt_testkit::core::app_hello())
        .await?;

    assert_eq!(
        error(connection.receive().await?.outcome)?.code,
        ErrorCode::INVALID_REQUEST
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn method_not_found_is_for_a_method_the_core_does_not_have() -> Result<(), TestError> {
    let harness = Harness::start()?;
    let mut connection = RawConnection::greeted(harness.endpoint()).await?;

    connection
        .request_named(RequestId::new(2), "actions.forget", json!({}))
        .await?;

    assert_eq!(
        error(connection.receive().await?.outcome)?.code,
        ErrorCode::METHOD_NOT_FOUND
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_params_is_for_params_the_action_cannot_take() -> Result<(), TestError> {
    let harness = Harness::start()?;
    let app = harness.app().await?;

    let call = ActionCall::new(Guarded::ID).params(json!({ "risk": "unknown" }));

    assert_eq!(
        rejection(app.call(call).await).code,
        ErrorCode::INVALID_PARAMS
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_params_is_for_an_approval_answer_of_the_wrong_shape() -> Result<(), TestError> {
    let harness = Harness::start()?;
    let mut connection = RawConnection::greeted(harness.endpoint()).await?;

    connection
        .request(
            RequestId::new(2),
            Method::ApprovalRespond,
            &json!({ "id": 1, "decision": "maybe" }),
        )
        .await?;

    assert_eq!(
        error(connection.receive().await?.outcome)?.code,
        ErrorCode::INVALID_PARAMS
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn internal_is_for_a_bug_in_the_core_and_is_journaled_as_failed() -> Result<(), TestError> {
    let harness = Harness::start()?;
    let app = harness.app().await?;

    let error = rejection(app.call(ActionCall::new(Confused::ID)).await);

    assert_eq!(error.code, ErrorCode::INTERNAL);
    assert_eq!(
        kinds(&harness.events()?),
        [EventKind::Requested, EventKind::Started, EventKind::Failed]
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn handshake_required_is_for_a_first_request_that_is_not_hello() -> Result<(), TestError> {
    let harness = Harness::start()?;
    let mut connection = RawConnection::open(harness.endpoint()).await?;

    connection
        .request(RequestId::new(1), Method::ActionsDescribe, &json!({}))
        .await?;

    assert_eq!(
        error(connection.receive().await?.outcome)?.code,
        ErrorCode::HANDSHAKE_REQUIRED
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn protocol_mismatch_is_for_another_major_version() -> Result<(), TestError> {
    let harness = Harness::start()?;
    let mut connection = RawConnection::open(harness.endpoint()).await?;

    let mut older = synt_testkit::core::app_hello();
    older.protocol = ProtocolVersion {
        major: ProtocolVersion::CURRENT.major.wrapping_sub(1),
        minor: 0,
    };
    connection.hello(RequestId::new(1), &older).await?;

    assert_eq!(
        error(connection.receive().await?.outcome)?.code,
        ErrorCode::PROTOCOL_MISMATCH
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn unknown_action_is_for_an_action_no_one_registered() -> Result<(), TestError> {
    let harness = Harness::start()?;
    let app = harness.app().await?;

    let error = rejection(app.call(ActionCall::new("test.nothing")).await);

    assert_eq!(error.code, ErrorCode::UNKNOWN_ACTION);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn action_failed_is_for_an_action_that_ran_and_failed() -> Result<(), TestError> {
    let harness = Harness::start()?;
    let app = harness.app().await?;

    let error = rejection(
        app.call(ActionCall::new(synt_testkit::core::Failing::ID))
            .await,
    );

    assert_eq!(error.code, ErrorCode::ACTION_FAILED);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn denied_is_for_a_call_the_broker_refuses() -> Result<(), TestError> {
    let harness = Harness::start()?;
    let agent = harness.agent("deploy-bot").await?;

    let error = rejection(agent.call(ActionCall::new(Touch::ID)).await);

    assert_eq!(error.code, ErrorCode::DENIED);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn denied_is_for_an_agent_answering_an_approval() -> Result<(), TestError> {
    let harness = Harness::start()?;
    let agent = harness.agent("deploy-bot").await?;

    let error = rejection(
        agent
            .respond(ApprovalId::new(1), ApprovalDecision::AllowOnce)
            .await,
    );

    assert_eq!(error.code, ErrorCode::DENIED);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn journal_unavailable_is_for_a_call_that_cannot_be_journaled_and_so_never_runs()
-> Result<(), TestError> {
    let harness = Harness::start()?;
    let app = harness.app().await?;

    harness.grounds().fail_journal(true);
    let error = rejection(app.call(ActionCall::new(Touch::ID)).await);

    assert_eq!(error.code, ErrorCode::JOURNAL_UNAVAILABLE);
    assert_eq!(error.message, "the journal is unavailable");
    assert_eq!(harness.touches(), 0);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn journal_unavailable_passes_once_the_journal_recovers() -> Result<(), TestError> {
    let harness = Harness::start()?;
    let app = harness.app().await?;

    harness.grounds().fail_journal(true);
    rejection(app.call(ping()).await);
    harness.grounds().fail_journal(false);
    let after = app.call(ping()).await;

    assert!(after.is_ok(), "{after:?}");
    assert!(harness.is_intact()?);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn approval_unknown_is_for_an_answer_no_question_waits_for() -> Result<(), TestError> {
    let harness = Harness::start()?;
    let app = harness.app().await?;

    let error = rejection(
        app.respond(ApprovalId::new(1), ApprovalDecision::Deny)
            .await,
    );

    assert_eq!(error.code, ErrorCode::APPROVAL_UNKNOWN);

    Ok(())
}
