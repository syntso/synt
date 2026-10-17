use std::time::Duration;

use synt_client::{Autostart, Client};
use synt_core::{Activity, Broker, Core, IdleWatch, Server};
use synt_platform::AppCheck;
use synt_testkit::TestError;
use synt_testkit::core::{Grounds, Switch, app_hello, eventually};
use tokio::time::{self, Instant};

const LIMIT: Duration = Duration::from_mins(10);
const LOOK_EVERY: Duration = Duration::from_secs(5);
const EARLIEST_IDLE: Duration = Duration::from_secs(595);
const A_DAY: Duration = Duration::from_hours(24);

#[tokio::test(start_paused = true)]
async fn without_a_limit_the_core_never_idles() {
    let idle = IdleWatch::new(None).watch(Switch::default());

    let waited = time::timeout(A_DAY, idle.idle()).await;

    assert!(waited.is_err());
}

#[tokio::test(start_paused = true)]
async fn a_quiet_core_idles_once_the_limit_passes() {
    let idle = IdleWatch::new(Some(LIMIT)).watch(Switch::default());
    let started = Instant::now();

    idle.idle().await;
    let quiet = started.elapsed();

    assert!(quiet >= LIMIT, "{quiet:?}");
    assert!(quiet <= LIMIT + LOOK_EVERY, "{quiet:?}");
}

#[tokio::test(start_paused = true)]
async fn activity_keeps_the_core_awake_and_the_limit_counts_from_its_end() {
    let busy = Switch::default();
    busy.set(true);
    let idle = tokio::spawn(IdleWatch::new(Some(LIMIT)).watch(busy.clone()).idle());

    time::sleep(LIMIT * 3).await;
    assert!(!idle.is_finished());

    busy.set(false);
    let quiet_from = Instant::now();
    idle.await.unwrap();
    let quiet = quiet_from.elapsed();

    assert!(quiet >= EARLIEST_IDLE, "{quiet:?}");
    assert!(quiet <= LIMIT + LOOK_EVERY, "{quiet:?}");
}

#[tokio::test(start_paused = true)]
async fn one_active_source_is_enough_to_stay_awake() {
    let busy = Switch::default();
    busy.set(true);
    let idle = IdleWatch::new(Some(LIMIT))
        .watch(Switch::default())
        .watch(busy);

    let waited = time::timeout(A_DAY, idle.idle()).await;

    assert!(waited.is_err());
}

#[tokio::test(start_paused = true)]
async fn a_limit_shorter_than_a_look_is_still_kept() {
    let limit = Duration::from_secs(2);
    let idle = IdleWatch::new(Some(limit)).watch(Switch::default());
    let started = Instant::now();

    idle.idle().await;

    assert!(started.elapsed() <= limit + Duration::from_secs(1));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_connected_client_counts_as_activity_until_it_leaves() -> Result<(), TestError> {
    let grounds = Grounds::new()?;
    let endpoint = grounds.endpoint();
    let core = Core::new(grounds.registry()?, grounds.journal(), Broker::new());
    let server = Server::bind(&endpoint, core, AppCheck::for_this_core().unwrap()).unwrap();
    let activity = server.activity();
    let running = tokio::spawn(server.run(std::future::pending()));

    let quiet_at_start = !activity.is_active();
    let client = Client::connect(&endpoint, app_hello(), &Autostart::off())
        .await
        .unwrap();
    let busy_while_connected = activity.is_active();
    drop(client);
    eventually("the session to end", async || {
        (!activity.is_active()).then_some(())
    })
    .await;

    assert!(quiet_at_start);
    assert!(busy_while_connected);
    running.abort();

    Ok(())
}
