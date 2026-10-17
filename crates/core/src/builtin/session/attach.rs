use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use super::AttachedSession;
use super::open::CARRIED;
use crate::link::Link;
use crate::terminals::{SessionId, Terminals};

#[derive(Debug, Clone)]
pub struct SessionAttach {
    terminals: Terminals,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct SessionAttachParams {
    pub session: SessionId,
}

impl SessionAttach {
    pub fn new(terminals: Terminals) -> Self {
        Self { terminals }
    }
}

impl Action for SessionAttach {
    const ID: &'static str = "session.attach";
    const TITLE: &'static str = "Attach to an open terminal session";
    const RISK: Risk = Risk::Write;
    const PERSON_ONLY: bool = true;

    type Params = SessionAttachParams;
    type Output = AttachedSession;

    fn risk(&self, _target: Option<&ResourceRef>, params: &SessionAttachParams) -> Risk {
        self.terminals
            .get(params.session)
            .map_or(Risk::Write, |session| session.origin().risk)
    }

    fn summary(&self, output: &AttachedSession) -> Value {
        json!({
            "session": output.session,
            "stream": output.stream,
            "replayed_bytes": output.replayed_bytes,
        })
    }

    async fn run(
        &self,
        context: &Context,
        params: SessionAttachParams,
    ) -> Result<AttachedSession, ActionError> {
        let link = Link::of_call(context, CARRIED)?;
        let session = self.terminals.find(params.session)?;
        let attached = session.attach(&link).await;

        Ok(attached)
    }
}
