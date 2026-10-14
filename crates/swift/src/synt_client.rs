use std::sync::{Arc, Mutex, PoisonError};

use bytes::Bytes;
use serde_json::Value;
use synt_client::{Autostart, Client};
use synt_platform::Endpoint;
use synt_proto::{ActionCall, ApprovalId, ClientKind, Hello, Requester, StreamId};
use tokio::runtime::Handle;
use tokio::sync::mpsc;
use tokio::task::AbortHandle;

use crate::forward::{self, Event};
use crate::{ActionInfo, ApprovalDecision, CallOutcome, CoreListener, SyntError};

#[derive(uniffi::Object)]
pub struct SyntClient {
    client: Client,
    events: mpsc::Sender<Event>,
    runtime: Handle,
    forwarders: Mutex<Vec<AbortHandle>>,
}

#[uniffi::export(async_runtime = "tokio")]
impl SyntClient {
    #[uniffi::constructor]
    pub async fn connect(
        endpoint: String,
        software: String,
        core_program: Option<String>,
        listener: Arc<dyn CoreListener>,
    ) -> Result<Arc<Self>, SyntError> {
        let endpoint = Endpoint::new(endpoint);
        let hello = Hello::new(ClientKind::App, software);
        let autostart = autostart(core_program);

        let client = Client::connect(&endpoint, hello, &autostart).await?;
        let runtime = Handle::current();
        let events = forward::start_dispatcher(listener)?;

        let notifications = forward::notifications(client.notifications(), events.clone());
        let forwarder = runtime.spawn(notifications).abort_handle();

        Ok(Arc::new(Self {
            client,
            events,
            runtime,
            forwarders: Mutex::new(vec![forwarder]),
        }))
    }

    pub fn core_version(&self) -> String {
        self.client.welcome().core.clone()
    }

    pub fn acts_for_person(&self) -> bool {
        self.client.welcome().actor == Requester::Person
    }

    pub async fn describe(&self) -> Result<Vec<ActionInfo>, SyntError> {
        let descriptions = self.client.describe().await?;

        Ok(descriptions.into_iter().map(ActionInfo::from).collect())
    }

    pub async fn call(
        &self,
        action: String,
        target: Option<String>,
        params: String,
    ) -> Result<CallOutcome, SyntError> {
        let params = parse_params(&params)?;

        let mut call = ActionCall::new(action).params(params);
        call.target = target;

        let outcome = self.client.call(call).await?;

        Ok(outcome.into())
    }

    pub async fn respond(
        &self,
        approval: u64,
        decision: ApprovalDecision,
    ) -> Result<(), SyntError> {
        self.client
            .respond(ApprovalId(approval), decision.into())
            .await?;

        Ok(())
    }

    pub fn watch(&self, stream: u32) {
        let receiver = self.client.stream(StreamId(stream));

        let forwarding = forward::stream(receiver, self.events.clone());
        let forwarder = self.runtime.spawn(forwarding).abort_handle();

        self.remember(forwarder);
    }

    pub async fn send(&self, stream: u32, data: Vec<u8>) -> Result<(), SyntError> {
        self.client
            .send_stream(StreamId(stream), Bytes::from(data))
            .await?;

        Ok(())
    }
}

impl SyntClient {
    fn remember(&self, forwarder: AbortHandle) {
        let mut forwarders = self
            .forwarders
            .lock()
            .unwrap_or_else(PoisonError::into_inner);

        forwarders.retain(|forwarder| !forwarder.is_finished());
        forwarders.push(forwarder);
    }
}

impl Drop for SyntClient {
    fn drop(&mut self) {
        let forwarders = self
            .forwarders
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner);

        for forwarder in forwarders.drain(..) {
            forwarder.abort();
        }
    }
}

fn autostart(core_program: Option<String>) -> Autostart {
    match core_program {
        Some(program) => Autostart::core().program(program),
        None => Autostart::off(),
    }
}

fn parse_params(params: &str) -> Result<Value, SyntError> {
    if params.trim().is_empty() {
        return Ok(Value::Null);
    }

    Ok(serde_json::from_str(params)?)
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::parse_params;
    use crate::SyntError;

    #[test]
    fn empty_params_are_null() {
        assert_eq!(parse_params("").unwrap(), Value::Null);
        assert_eq!(parse_params("  ").unwrap(), Value::Null);
    }

    #[test]
    fn params_are_parsed_as_json() {
        let params = parse_params(r#"{"cols": 80}"#).unwrap();

        assert_eq!(params, json!({ "cols": 80 }));
    }

    #[test]
    fn broken_params_are_invalid_json() {
        let broken = parse_params("{cols");

        assert!(matches!(broken, Err(SyntError::InvalidJson { .. })));
    }
}
