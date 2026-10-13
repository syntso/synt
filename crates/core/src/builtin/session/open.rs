use std::sync::Arc;
use std::time::Duration;

use schemars::JsonSchema;
use serde::Deserialize;
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_proto::EndReason;
use synt_ssh::{Pool, Shell, ShellOutput, Target, TerminalSize};
use tokio::time;

use super::AttachedSession;
use super::checks::{connection, terminal_size};
use crate::builtin::ssh::{destination, failure, logs_in_as_root, reach_host};
use crate::projects::Projects;
use crate::terminal::{Origin, SessionId, Terminal, Terminals};

const DEFAULT_TERM: &str = "xterm-256color";
const LONGEST_TERM: usize = 64;
const CONNECT_LIMIT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct SshSessionOpen {
    pool: Pool,
    terminals: Terminals,
    projects: Projects,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct SessionOpenParams {
    #[serde(rename = "cols")]
    pub columns: u16,
    pub rows: u16,

    #[serde(default)]
    pub term: Option<String>,
}

impl SshSessionOpen {
    pub fn new(pool: Pool, terminals: Terminals, projects: Projects) -> Self {
        Self {
            pool,
            terminals,
            projects,
        }
    }

    async fn shell(
        &self,
        target: &Target,
        term: &str,
        size: TerminalSize,
    ) -> Result<(Shell, ShellOutput), ActionError> {
        let lease = time::timeout(CONNECT_LIMIT, self.pool.acquire(target))
            .await
            .map_err(|_| {
                ActionError::failed(format!(
                    "could not connect within {} s",
                    CONNECT_LIMIT.as_secs()
                ))
            })?
            .map_err(|error| failure(&error))?;

        Shell::open(lease, term, size)
            .await
            .map_err(|error| failure(&error))
    }
}

impl Action for SshSessionOpen {
    const ID: &'static str = "ssh.session.open";
    const TITLE: &'static str = "Open an interactive shell on a host";
    const RISK: Risk = Risk::Write;
    const PERSON_ONLY: bool = true;
    const TAKES_TARGET: bool = true;
    const FINISHES_LATER: bool = true;

    type Params = SessionOpenParams;
    type Output = AttachedSession;

    fn risk(&self, target: Option<&ResourceRef>, _params: &SessionOpenParams) -> Risk {
        if logs_in_as_root(&self.projects, target) {
            return Risk::Privileged;
        }

        Risk::Write
    }

    fn destination(
        &self,
        target: Option<&ResourceRef>,
        _params: &SessionOpenParams,
    ) -> Option<String> {
        destination(&self.projects, target)
    }

    async fn run(
        &self,
        context: &Context,
        params: SessionOpenParams,
    ) -> Result<AttachedSession, ActionError> {
        let host = context.target().ok_or(ActionError::TargetRequired)?;
        let size = terminal_size(params.columns, params.rows)?;
        let term = term(params.term.as_deref())?;
        let link = connection(context)?;

        let target = reach_host(&self.projects, context).await?;
        let risk = context.risk();

        let opened = self.shell(&target, term, size).await?;

        let id = SessionId::from(context.request());
        let origin = Origin {
            actor: context.actor().clone(),
            target: host.clone(),
            request: context.request(),
            risk,
        };

        let terminal = self
            .terminals
            .start(id, origin, opened, size)
            .ok_or_else(|| ActionError::failed("the core is stopping, so no session was opened"))?;
        let attached = terminal.attach(&link).await;

        close_when_revoked(context, terminal);

        Ok(AttachedSession::new(id, &attached))
    }
}

fn close_when_revoked(context: &Context, terminal: Arc<Terminal>) {
    let Some(hold) = context.hold() else {
        return;
    };
    let stop = context.stop().clone();

    tokio::spawn(async move {
        tokio::select! {
            () = stop.cancelled() => {
                terminal.close(EndReason::Closed).await;
            }
            () = terminal.finished() => {}
        }

        drop(hold);
    });
}

fn term(requested: Option<&str>) -> Result<&str, ActionError> {
    let Some(term) = requested else {
        return Ok(DEFAULT_TERM);
    };

    let allowed = |character: char| character.is_ascii_alphanumeric() || "-_.+".contains(character);

    if term.is_empty() || term.len() > LONGEST_TERM || !term.chars().all(allowed) {
        return Err(ActionError::InvalidParams(format!(
            "term must be 1 to {LONGEST_TERM} letters, digits or -_.+"
        )));
    }

    Ok(term)
}
