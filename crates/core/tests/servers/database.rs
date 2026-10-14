use serde_json::json;
use synt_proto::ActionCall;
use synt_test_servers::DbServers;

use crate::Lab;
use crate::lab::{POSTGRES, POSTGRES_PASSWORD};

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the ssh and database test servers: tests/ssh/up.sh, tests/db/up.sh, \
            then SYNT_SSH_TESTS=1 SYNT_DB_TESTS=1"]
async fn a_database_behind_two_ssh_hops_answers_through_a_private_tunnel() {
    if DbServers::running().is_none() {
        return;
    }
    let Some(lab) = Lab::start().await else {
        return;
    };
    let storing = ActionCall::new("password.set")
        .params(json!({ "name": POSTGRES_PASSWORD, "password": DbServers::PASSWORD }));
    lab.client.call(storing).await.unwrap();

    let seen = lab
        .call(
            POSTGRES,
            "db.query",
            json!({ "sql": "SELECT inet_client_addr()::text, ssl FROM pg_stat_ssl \
                            WHERE pid = pg_backend_pid()" }),
        )
        .await
        .unwrap();
    let schema = lab.call(POSTGRES, "db.schema", json!(null)).await.unwrap();
    let finished = lab.finished(seen.request).await;
    let client_address = seen.value["rows"][0][0].as_str().unwrap().to_owned();

    assert!(!client_address.starts_with("127."), "{client_address}");
    assert_eq!(seen.value["rows"][0][1], json!(true));
    assert!(schema.value["tables"].is_array());
    assert_eq!(finished["rows"], json!(1));

    lab.stop().await;
}
