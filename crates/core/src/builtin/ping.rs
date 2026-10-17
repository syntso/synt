use serde::{Deserialize, Serialize};
use synt_actions::{Action, ActionError, ActionGroup, Context, Registry, Risk};
use synt_proto::RequestSeq;

use crate::VERSION;

#[derive(Debug, Clone, Copy, Default)]
pub struct Ping;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pong {
    pub core: String,
    pub request: RequestSeq,
}

impl Action for Ping {
    const ID: &'static str = "core.ping";
    const TITLE: &'static str = "Check that the core answers";
    const RISK: Risk = Risk::Read;

    type Params = ();
    type Output = Pong;

    async fn run(&self, context: &Context, (): ()) -> Result<Pong, ActionError> {
        Ok(Pong {
            core: VERSION.to_owned(),
            request: RequestSeq::new(context.request().get()),
        })
    }
}

impl ActionGroup for Ping {
    fn register(&self, registry: &mut Registry) -> Result<(), ActionError> {
        registry.register(*self)
    }
}
