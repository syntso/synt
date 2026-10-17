use schemars::JsonSchema;
use serde::Deserialize;
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_ssh::{Pool, Shell, Target, TerminalSize};

use super::{AttachedSession, Dimensions, Term};
use crate::CoreError;
use crate::builtin::ssh::within_connect_limit;
use crate::link::Link;
use crate::projects::Projects;
use crate::terminals::{OpenConsole, Origin, SessionId, Terminals};

pub(super) const CARRIED: &str = "a terminal";

#[derive(Debug, Clone)]
pub struct SshSessionOpen {
    pool: Pool,
    terminals: Terminals,
    projects: Projects,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct SessionOpenParams {
    #[serde(rename = "cols")]
    pub(crate) columns: u16,
    pub(crate) rows: u16,

    #[serde(default)]
    pub(crate) term: Term,
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
        term: &Term,
        size: TerminalSize,
    ) -> Result<OpenConsole, ActionError> {
        let lease = within_connect_limit(self.pool.acquire(target)).await?;

        let (shell, output) = Shell::open(lease, term.as_str(), size)
            .await
            .map_err(CoreError::from)?;

        Ok(OpenConsole::new(shell, output))
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
        self.projects.login_risk(target, Self::RISK)
    }

    fn destination(
        &self,
        target: Option<&ResourceRef>,
        _params: &SessionOpenParams,
    ) -> Option<String> {
        self.projects.host_destination(target)
    }

    async fn run(
        &self,
        context: &Context,
        params: SessionOpenParams,
    ) -> Result<AttachedSession, ActionError> {
        let host = context.target().ok_or(ActionError::TargetRequired)?;
        let size = params.size()?;
        let link = Link::of_call(context, CARRIED)?;

        let target = self.projects.reach_host(context).await?;
        let risk = context.risk();

        let opened = self.shell(&target, &params.term, size).await?;

        let id = SessionId::new(context.request().get());
        let origin = Origin {
            action: Self::ID,
            actor: context.actor().clone(),
            target: Some(host.clone()),
            request: context.request(),
            risk,
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

impl Dimensions for SessionOpenParams {
    fn columns(&self) -> u16 {
        self.columns
    }

    fn rows(&self) -> u16 {
        self.rows
    }
}
