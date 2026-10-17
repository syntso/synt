use std::sync::Arc;

use serde::Serialize;
use serde_json::{Value, json};
use synt_journal::Actor;
use synt_proto::{ActionCall, ApprovalAnswer, ErrorCode, Method, Request, RpcError};
use tokio_util::sync::CancellationToken;

use crate::core::Core;
use crate::link::Link;
use crate::log::log;

pub(super) struct Responder {
    pub(super) core: Arc<Core>,
    pub(super) actor: Arc<Actor>,
    pub(super) departed: CancellationToken,
    pub(super) link: Arc<Link>,
}

impl Responder {
    pub(super) async fn answer(&self, request: Request) -> Result<Value, RpcError> {
        match request.method() {
            Some(Method::ActionCall) => {
                let call: ActionCall = request.into_params()?;
                let outcome = self
                    .core
                    .dispatch(&self.actor, call, &self.departed, &self.link)
                    .await?;

                Self::encoded(&outcome, "outcome", "the outcome cannot be encoded")
            }
            Some(Method::ActionsDescribe) => {
                let descriptions = self.core.describe(&self.actor);

                Self::encoded(
                    &descriptions,
                    "descriptions",
                    "the action descriptions cannot be encoded",
                )
            }
            Some(Method::ApprovalRespond) => {
                let answer: ApprovalAnswer = request.into_params()?;
                self.core.respond(&self.actor, answer)?;

                Ok(json!({}))
            }
            Some(Method::Hello) => Err(RpcError::invalid_request("hello was already received")),
            None => Err(RpcError::new(
                ErrorCode::METHOD_NOT_FOUND,
                format!("no method named {}", request.method),
            )),
        }
    }

    fn encoded(value: &impl Serialize, what: &str, refusal: &str) -> Result<Value, RpcError> {
        serde_json::to_value(value).map_err(|error| {
            log!("{what}: {error}");
            RpcError::internal(refusal)
        })
    }
}
