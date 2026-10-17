use synt_actions::{Action, ActionError, Context, Risk};
use synt_platform::Pty;
use synt_ssh::TerminalSize;
use tokio::task;

use super::checks::{connection, term, terminal_size};
use super::revocation::close_when_revoked;
use super::{AttachedSession, SessionOpenParams};
use crate::terminal::{Console, ConsoleOutput, Origin, SessionId, Terminals};

#[derive(Debug, Clone)]
pub struct LocalSessionOpen {
    terminals: Terminals,
}

impl LocalSessionOpen {
    pub fn new(terminals: Terminals) -> Self {
        Self { terminals }
    }
}

impl Action for LocalSessionOpen {
    const ID: &'static str = "local.session.open";
    const TITLE: &'static str = "Open an interactive shell on this computer";
    const RISK: Risk = Risk::Write;
    const PERSON_ONLY: bool = true;
    const FINISHES_LATER: bool = true;

    type Params = SessionOpenParams;
    type Output = AttachedSession;

    async fn run(
        &self,
        context: &Context,
        params: SessionOpenParams,
    ) -> Result<AttachedSession, ActionError> {
        let size = terminal_size(params.columns, params.rows)?;
        let term = term(params.term.as_deref())?.to_owned();
        let link = connection(context)?;

        let opened = open_pty(term, size).await?;

        let id = SessionId::new(context.request().get());
        let origin = Origin {
            action: Self::ID,
            actor: context.actor().clone(),
            target: None,
            request: context.request(),
            risk: context.risk(),
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

async fn open_pty(
    term: String,
    size: TerminalSize,
) -> Result<(Console, ConsoleOutput), ActionError> {
    let opened = task::spawn_blocking(move || Pty::open(&term, size.columns, size.rows))
        .await
        .map_err(|_| ActionError::failed("the shell could not be started"))?;

    let (pty, output) = opened
        .map_err(|error| ActionError::failed(format!("could not start the shell: {error}")))?;

    Ok((Console::Local(pty), ConsoleOutput::Local(output)))
}
