use std::env;
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use synt_platform::{Endpoint, LocalStream, start_detached};
use tokio::{task, time};

use crate::ClientError;

const CORE_VARIABLE: &str = "SYNT_CORE";
const CORE_NAME: &str = "synt-core";
const ENDPOINT_FLAG: &str = "--endpoint";
const PATIENCE: Duration = Duration::from_secs(5);
const RETRY_PAUSE: Duration = Duration::from_millis(50);

#[derive(Debug, Clone)]
pub struct Autostart {
    enabled: bool,
    program: Option<PathBuf>,
    arguments: Vec<OsString>,
}

impl Autostart {
    pub fn off() -> Self {
        Self {
            enabled: false,
            program: None,
            arguments: Vec::new(),
        }
    }

    pub fn core() -> Self {
        Self {
            enabled: true,
            program: None,
            arguments: Vec::new(),
        }
    }

    #[must_use]
    pub fn program(mut self, program: impl Into<PathBuf>) -> Self {
        self.program = Some(program.into());
        self
    }

    #[must_use]
    pub fn arguments<I, S>(mut self, arguments: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.arguments.extend(arguments.into_iter().map(Into::into));
        self
    }

    pub(crate) fn applies_to(&self, error: &io::Error) -> bool {
        self.enabled
            && matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
            )
    }

    pub(crate) async fn start(&self, endpoint: &Endpoint) -> Result<LocalStream, ClientError> {
        let program = self.resolve()?;

        self.launch(&program, endpoint).await?;

        wait_for(endpoint)
            .await
            .ok_or_else(|| ClientError::CoreSilent {
                program: program.display().to_string(),
                seconds: PATIENCE.as_secs(),
            })
    }

    fn resolve(&self) -> Result<PathBuf, ClientError> {
        if let Some(program) = &self.program {
            return absolute(program.clone());
        }

        if cfg!(debug_assertions)
            && let Some(program) = env::var_os(CORE_VARIABLE)
        {
            return absolute(PathBuf::from(program));
        }

        let running = env::current_exe().map_err(ClientError::CoreNotFound)?;
        let folder = running.parent().ok_or_else(|| {
            ClientError::CoreNotFound(io::Error::new(
                io::ErrorKind::NotFound,
                "the running program has no folder",
            ))
        })?;

        Ok(folder.join(format!("{CORE_NAME}{}", env::consts::EXE_SUFFIX)))
    }

    async fn launch(&self, program: &Path, endpoint: &Endpoint) -> Result<(), ClientError> {
        let mut arguments = vec![OsString::from(ENDPOINT_FLAG), endpoint.absolute_address()];
        arguments.extend(self.arguments.iter().cloned());

        let launched = program.to_owned();
        let started = task::spawn_blocking(move || start_detached(&launched, &arguments)).await;

        let reason = match started {
            Ok(Ok(())) => return Ok(()),
            Ok(Err(error)) => error.to_string(),
            Err(error) => error.to_string(),
        };

        Err(ClientError::CoreStart {
            program: program.display().to_string(),
            reason,
        })
    }
}

fn absolute(program: PathBuf) -> Result<PathBuf, ClientError> {
    if program.is_absolute() {
        return Ok(program);
    }

    Err(ClientError::CoreStart {
        program: program.display().to_string(),
        reason: "the core must be named by an absolute path".to_owned(),
    })
}

async fn wait_for(endpoint: &Endpoint) -> Option<LocalStream> {
    let deadline = time::Instant::now() + PATIENCE;

    while time::Instant::now() < deadline {
        if let Ok(stream) = LocalStream::connect(endpoint).await {
            return Some(stream);
        }

        time::sleep(RETRY_PAUSE).await;
    }

    None
}
