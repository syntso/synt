#![cfg(unix)]

use std::path::Path;

use synt_client::{Autostart, Client, ClientError};
use synt_core::VERSION;
use synt_platform::Endpoint;
use synt_testkit::TestError;
use synt_testkit::core::{app_hello, ping};
use tempfile::TempDir;

const CORE: &str = env!("CARGO_BIN_EXE_synt-core");

struct Place {
    folder: TempDir,
    endpoint: Endpoint,
}

impl Place {
    fn new() -> Result<Self, TestError> {
        let folder = tempfile::tempdir()?;
        let unique = folder
            .path()
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let endpoint = Endpoint::named(folder.path(), &format!("synt-auto-{unique}"));

        Ok(Self { folder, endpoint })
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
async fn a_core_that_is_not_running_is_started_and_answers() -> Result<(), TestError> {
    let place = Place::new()?;

    let client = place.connect(&place.autostart()).await.unwrap();
    let answered = client.call(ping()).await;

    assert_eq!(client.welcome().core, VERSION);
    assert!(answered.is_ok());

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_running_core_is_reached_and_not_started_again() -> Result<(), TestError> {
    let place = Place::new()?;
    let autostart = place.autostart();

    let first = place.connect(&autostart).await.unwrap();
    let first_ping = first.call(ping()).await.unwrap();
    let second = place.connect(&autostart).await.unwrap();
    let second_ping = second.call(ping()).await.unwrap();

    assert!(second_ping.request.get() > first_ping.request.get());

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn without_autostart_a_missing_core_is_a_connection_error() -> Result<(), TestError> {
    let place = Place::new()?;

    let refused = place.connect(&Autostart::off()).await;

    assert!(matches!(refused, Err(ClientError::Io(_))));

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn the_core_must_be_named_by_an_absolute_path() -> Result<(), TestError> {
    let place = Place::new()?;

    let refused = place.connect(&Autostart::core().program("synt-core")).await;

    assert!(matches!(refused, Err(ClientError::CoreNotAbsolute { .. })));

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_core_program_that_does_not_exist_is_reported() -> Result<(), TestError> {
    let place = Place::new()?;
    let missing = place.folder.path().join("no-such-core");

    let refused = place.connect(&Autostart::core().program(&missing)).await;

    assert!(matches!(refused, Err(ClientError::CoreStart { .. })));

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_program_that_starts_but_never_listens_is_reported_as_silent() -> Result<(), TestError> {
    let place = Place::new()?;
    let silent = Path::new("/usr/bin/true");

    let refused = place.connect(&Autostart::core().program(silent)).await;

    assert!(matches!(
        refused,
        Err(ClientError::CoreSilent { seconds: 5, .. })
    ));

    Ok(())
}
