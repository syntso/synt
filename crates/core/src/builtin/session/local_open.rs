use synt_actions::{Action, ActionError, Context, Risk};
use synt_platform::Pty;
use synt_ssh::TerminalSize;

use super::checks::{connection, term, terminal_size};
use super::revocation::close_when_revoked;
use super::{AttachedSession, SessionOpenParams};
use crate::CoreError;
use crate::blocking::blocking;
use crate::terminals::{OpenConsole, Origin, SessionId, Terminals};

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

        let id = SessionId::from(context.request());
        let origin = Origin {
            action: Self::ID,
            actor: context.actor().clone(),
            target: None,
            request: context.request(),
            risk: context.risk(),
        };

        let session = self
            .terminals
            .start(id, origin, opened, size)
            .ok_or_else(|| ActionError::failed("the core is stopping, so no session was opened"))?;
        let attached = session.attach(&link).await;

        close_when_revoked(context, session);

        Ok(attached)
    }
}

async fn open_pty(term: String, size: TerminalSize) -> Result<OpenConsole, ActionError> {
    let (pty, output) = blocking::<_, ActionError>(move || {
        Pty::open(&term, size.columns, size.rows).map_err(CoreError::PtyStart)
    })
    .await?;

    Ok(OpenConsole::new(pty, output))
}
