use serde::{Deserialize, Serialize};
use synt_actions::{Action, ActionError, Context, Risk};
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
    const RISK: Risk = Risk::Read;

    type Params = ();
    type Output = Pong;

    async fn run(&self, context: &Context, (): ()) -> Result<Pong, ActionError> {
        Ok(Pong {
            core: VERSION.to_owned(),
            request: RequestSeq(context.request().get()),
        })
    }
}
