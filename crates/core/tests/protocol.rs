#![allow(clippy::unwrap_used)]

mod common;

use common::{Harness, RawConnection, app_hello, ping};
use serde_json::json;
use synt_core::Pong;
use synt_proto::{
    ActionOutcome, ErrorCode, Method, Outcome, ProtocolVersion, RequestId, RequestSeq, RpcError,
    Welcome,
};

#[tokio::test(flavor = "multi_thread")]
async fn a_request_before_hello_is_refused_and_the_connection_closes() {
    let harness = Harness::start();
    let mut connection = RawConnection::open(harness.endpoint()).await;

    connection
        .request(RequestId(1), Method::ActionCall, &ping())
        .await;
    let response = connection.receive().await;

    assert_eq!(response.id, Some(RequestId(1)));
    assert_eq!(error(response.outcome).code, ErrorCode::HANDSHAKE_REQUIRED);
    assert!(connection.is_closed().await);
    assert!(harness.events().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn garbage_before_hello_is_refused_and_the_connection_closes() {
    let harness = Harness::start();
    let mut connection = RawConnection::open(harness.endpoint()).await;

    connection.send_bytes(&b"{not json"[..]).await;
    let response = connection.receive().await;

    assert_eq!(response.id, None);
    assert_eq!(error(response.outcome).code, ErrorCode::PARSE_ERROR);
    assert!(connection.is_closed().await);
}

#[tokio::test(flavor = "multi_thread")]
async fn hello_is_answered_with_a_welcome() {
    let harness = Harness::start();
    let mut connection = RawConnection::open(harness.endpoint()).await;

    connection.hello(RequestId(7), &app_hello()).await;
    let response = connection.receive().await;
    let welcome: Welcome = serde_json::from_value(response.outcome.into_result().unwrap()).unwrap();

    assert_eq!(response.id, Some(RequestId(7)));
    assert_eq!(welcome.protocol, ProtocolVersion::CURRENT);
}

#[tokio::test(flavor = "multi_thread")]
async fn another_major_version_is_refused() {
    let harness = Harness::start();
    let mut connection = RawConnection::open(harness.endpoint()).await;

    let mut future = app_hello();
    future.protocol = ProtocolVersion {
        major: ProtocolVersion::CURRENT.major + 1,
        minor: 0,
    };

    connection.hello(RequestId(1), &future).await;
    let response = connection.receive().await;

    assert_eq!(error(response.outcome).code, ErrorCode::PROTOCOL_MISMATCH);
    assert!(connection.is_closed().await);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_newer_minor_version_is_welcomed() {
    let harness = Harness::start();
    let mut connection = RawConnection::open(harness.endpoint()).await;

    let mut newer = app_hello();
    newer.protocol.minor += 1;

    connection.hello(RequestId(1), &newer).await;
    let response = connection.receive().await;

    assert!(response.outcome.into_result().is_ok());
}

#[tokio::test(flavor = "multi_thread")]
async fn garbage_after_hello_is_a_parse_error_and_the_connection_survives() {
    let harness = Harness::start();
    let mut connection = RawConnection::greeted(harness.endpoint()).await;

    connection.send_bytes(&b"{not json"[..]).await;
    let garbage = connection.receive().await;

    connection
        .request(RequestId(2), Method::ActionCall, &ping())
        .await;
    let answered = connection.receive().await;
    let outcome: ActionOutcome =
        serde_json::from_value(answered.outcome.into_result().unwrap()).unwrap();
    let pong: Pong = serde_json::from_value(outcome.value).unwrap();

    assert_eq!(garbage.id, None);
    assert_eq!(error(garbage.outcome).code, ErrorCode::PARSE_ERROR);
    assert_eq!(answered.id, Some(RequestId(2)));
    assert_eq!(outcome.request, RequestSeq(1));
    assert_eq!(pong.request, outcome.request);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_frame_over_the_request_limit_closes_the_connection() {
    let harness = Harness::start();
    let mut connection = RawConnection::greeted(harness.endpoint()).await;

    connection
        .send_bytes_until_refused(vec![b' '; 128 * 1024])
        .await;

    assert!(connection.is_closed().await);
    assert!(harness.events().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_method_is_not_found() {
    let harness = Harness::start();
    let mut connection = RawConnection::greeted(harness.endpoint()).await;

    connection
        .request_named(RequestId(3), "core.shutdown", json!({}))
        .await;
    let response = connection.receive().await;

    assert_eq!(response.id, Some(RequestId(3)));
    assert_eq!(error(response.outcome).code, ErrorCode::METHOD_NOT_FOUND);
    assert!(harness.events().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_second_hello_is_an_invalid_request() {
    let harness = Harness::start();
    let mut connection = RawConnection::greeted(harness.endpoint()).await;

    connection.hello(RequestId(4), &app_hello()).await;
    let response = connection.receive().await;

    assert_eq!(error(response.outcome).code, ErrorCode::INVALID_REQUEST);
}

#[tokio::test(flavor = "multi_thread")]
async fn malformed_call_params_are_invalid_and_leave_no_trace() {
    let harness = Harness::start();
    let mut connection = RawConnection::greeted(harness.endpoint()).await;

    connection
        .request(RequestId(5), Method::ActionCall, &json!({ "params": 1 }))
        .await;
    let response = connection.receive().await;

    assert_eq!(error(response.outcome).code, ErrorCode::INVALID_PARAMS);
    assert!(harness.events().is_empty());
}

fn error(outcome: Outcome) -> RpcError {
    outcome.into_result().unwrap_err()
}
