#![cfg(unix)]
#![allow(clippy::unwrap_used)]

mod common;

use std::path::Path;

use common::{app_hello, ping};
use synt_client::{Autostart, Client, ClientError};
use synt_core::VERSION;
use synt_platform::Endpoint;
use tempfile::TempDir;

const CORE: &str = env!("CARGO_BIN_EXE_synt-core");

struct Place {
    folder: TempDir,
    endpoint: Endpoint,
}

impl Place {
    fn new() -> Self {
        let folder = tempfile::tempdir().unwrap();
        let unique = folder.path().file_name().unwrap().to_string_lossy();
        let endpoint = Endpoint::named(folder.path(), &format!("synt-auto-{unique}"));

        Self { folder, endpoint }
    }

    fn autostart(&self) -> Autostart {
        let data = self.folder.path().join("data");

        Autostart::core().program(CORE).arguments([
            "--data".as_ref(),
            data.as_os_str(),
            "--idle-minutes".as_ref(),
            "1".as_ref(),
        ])
    }

    async fn connect(&self, autostart: &Autostart) -> Result<Client, ClientError> {
        Client::connect(&self.endpoint, app_hello(), autostart).await
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_core_that_is_not_running_is_started_and_answers() {
    let place = Place::new();

    let client = place.connect(&place.autostart()).await.unwrap();
    let answered = client.call(ping()).await;

    assert_eq!(client.welcome().core, VERSION);
    assert!(answered.is_ok());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_running_core_is_reached_and_not_started_again() {
    let place = Place::new();
    let autostart = place.autostart();

    let first = place.connect(&autostart).await.unwrap();
    let first_ping = first.call(ping()).await.unwrap();
    let second = place.connect(&autostart).await.unwrap();
    let second_ping = second.call(ping()).await.unwrap();

    assert!(second_ping.request.get() > first_ping.request.get());
}

#[tokio::test(flavor = "multi_thread")]
async fn without_autostart_a_missing_core_is_a_connection_error() {
    let place = Place::new();

    let refused = place.connect(&Autostart::off()).await;

    assert!(matches!(refused, Err(ClientError::Io(_))));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_core_must_be_named_by_an_absolute_path() {
    let place = Place::new();

    let refused = place.connect(&Autostart::core().program("synt-core")).await;

    assert!(matches!(refused, Err(ClientError::CoreNotAbsolute { .. })));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_core_program_that_does_not_exist_is_reported() {
    let place = Place::new();
    let missing = place.folder.path().join("no-such-core");

    let refused = place.connect(&Autostart::core().program(&missing)).await;

    assert!(matches!(refused, Err(ClientError::CoreStart { .. })));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_program_that_starts_but_never_listens_is_reported_as_silent() {
    let place = Place::new();
    let silent = Path::new("/usr/bin/true");

    let refused = place.connect(&Autostart::core().program(silent)).await;

    assert!(matches!(
        refused,
        Err(ClientError::CoreSilent { seconds: 5, .. })
    ));
}
