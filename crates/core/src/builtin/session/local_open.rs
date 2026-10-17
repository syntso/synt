use synt_actions::{Action, ActionError, Context, Risk};
use synt_platform::Pty;
use synt_ssh::TerminalSize;

use super::open::CARRIED;
use super::{AttachedSession, Dimensions, SessionOpenParams, Term};
use crate::CoreError;
use crate::blocking::blocking;
use crate::link::Link;
use crate::terminals::{OpenConsole, Origin, SessionId, Terminals};

#[derive(Debug, Clone)]
pub struct LocalSessionOpen {
    terminals: Terminals,
}

impl LocalSessionOpen {
    pub fn new(terminals: Terminals) -> Self {
        Self { terminals }
    }

    async fn pty(term: Term, size: TerminalSize) -> Result<OpenConsole, ActionError> {
        let (pty, output) = blocking::<_, ActionError>(move || {
            Pty::open(term.as_str(), size.columns, size.rows).map_err(CoreError::PtyStart)
        })
        .await?;

        Ok(OpenConsole::new(pty, output))
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
        let size = params.size()?;
        let link = Link::of_call(context, CARRIED)?;

        let opened = Self::pty(params.term, size).await?;

        let id = SessionId::new(context.request().get());
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

        session.close_when_revoked(context);

        Ok(attached)
    }
}
