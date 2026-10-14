use serde_json::json;
use synt_db::{Database, DbError, Engine, Page, Server, TlsMode};
use synt_test_servers::DbServers;
use zeroize::Zeroizing;

fn redis(port: u16) -> Server {
    Server::network(Engine::Redis, DbServers::ADDRESS, port)
        .with_password(Some(Zeroizing::new(DbServers::PASSWORD.to_owned())))
}

fn page() -> Page {
    Page::new(100, 1 << 20)
}

#[tokio::test]
#[ignore = "needs the database test servers: tests/db/up.sh, then SYNT_DB_TESTS=1"]
async fn commands_run_and_replies_become_rows() {
    let Some(servers) = DbServers::running() else {
        return;
    };
    let mut database = Database::connect(&redis(servers.redis_port()))
        .await
        .unwrap();

    database
        .command(&["DEL", "synt:list"], page())
        .await
        .unwrap();
    database
        .command(&["RPUSH", "synt:list", "a", "b", "c"], page())
        .await
        .unwrap();
    let set = database
        .command(&["SET", "synt:name", "ann"], page())
        .await
        .unwrap();
    let got = database
        .command(&["GET", "synt:name"], page())
        .await
        .unwrap();
    let listed = database
        .command(&["LRANGE", "synt:list", "0", "-1"], Page::new(2, 1 << 20))
        .await
        .unwrap();

    assert_eq!(set.rows, vec![vec![json!("OK")]]);
    assert_eq!(got.rows, vec![vec![json!("ann")]]);
    assert_eq!(listed.rows, vec![vec![json!("a")], vec![json!("b")]]);
    assert_eq!(listed.next_offset, Some(2));
}

#[tokio::test]
#[ignore = "needs the database test servers: tests/db/up.sh, then SYNT_DB_TESTS=1"]
async fn a_database_number_selects_the_keyspace() {
    let Some(servers) = DbServers::running() else {
        return;
    };
    let zero = redis(servers.redis_port());
    let three = redis(servers.redis_port()).with_database(Some("3".to_owned()));

    let mut first = Database::connect(&zero).await.unwrap();
    let mut second = Database::connect(&three).await.unwrap();
    first
        .command(&["SET", "synt:where", "zero"], page())
        .await
        .unwrap();
    second
        .command(&["SET", "synt:where", "three"], page())
        .await
        .unwrap();

    let seen = first.command(&["GET", "synt:where"], page()).await.unwrap();

    assert_eq!(seen.rows, vec![vec![json!("zero")]]);
}

#[tokio::test]
#[ignore = "needs the database test servers: tests/db/up.sh, then SYNT_DB_TESTS=1"]
async fn tls_require_encrypts_and_verify_full_refuses_an_untrusted_certificate() {
    let Some(servers) = DbServers::running() else {
        return;
    };
    let secured = redis(servers.redis_tls_port()).with_tls(TlsMode::Require);
    let verified = redis(servers.redis_tls_port()).with_tls(TlsMode::VerifyFull);

    let mut database = Database::connect(&secured).await.unwrap();

    assert!(!database.ping().await.unwrap().is_empty());
    assert!(Database::connect(&verified).await.is_err());
}

#[tokio::test]
#[ignore = "needs the database test servers: tests/db/up.sh, then SYNT_DB_TESTS=1"]
async fn a_wrong_password_is_refused() {
    let Some(servers) = DbServers::running() else {
        return;
    };
    let wrong = redis(servers.redis_port()).with_password(Some(Zeroizing::new("no".to_owned())));

    let refused = Database::connect(&wrong).await.err().unwrap();

    assert!(matches!(refused, DbError::Server { .. }), "{refused:?}");
}

#[tokio::test]
#[ignore = "needs the database test servers: tests/db/up.sh, then SYNT_DB_TESTS=1"]
async fn sql_is_not_sent_to_redis() {
    let Some(servers) = DbServers::running() else {
        return;
    };
    let mut database = Database::connect(&redis(servers.redis_port()))
        .await
        .unwrap();

    let refused = database.tables().await.unwrap_err();

    assert!(matches!(refused, DbError::Unsupported(_)));
}
