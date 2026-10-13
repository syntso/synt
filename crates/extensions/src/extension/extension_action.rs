use std::sync::Arc;

use serde_json::Value;
use synt_actions::{ActionError, Context, DynamicAction, Risk};
use synt_journal::{Actor, ResourceRef};
use tokio::runtime::Handle;
use tokio::task;

use super::{ActionSpec, Guest};
use crate::runtime::GuestCall;

pub(crate) struct ExtensionAction {
    spec: ActionSpec,
    guest: Arc<Guest>,
}

impl ExtensionAction {
    pub(crate) fn new(spec: ActionSpec, guest: Arc<Guest>) -> Self {
        Self { spec, guest }
    }
}

impl DynamicAction for ExtensionAction {
    fn title(&self) -> &str {
        &self.spec.title
    }

    fn risk(&self) -> Risk {
        self.spec.risk
    }

    fn takes_target(&self) -> bool {
        self.spec.takes_target
    }

    fn params_schema(&self) -> Value {
        self.spec.params_schema.clone()
    }

    fn destination(&self, target: Option<&ResourceRef>) -> Option<String> {
        target.map(|target| target.as_str().to_owned())
    }

    async fn run(
        &self,
        context: &Context,
        target: Option<ResourceRef>,
        params: Value,
    ) -> Result<Value, ActionError> {
        if matches!(context.actor(), Actor::Plugin(_)) {
            return Err(ActionError::failed(
                "an extension cannot run the actions of an extension",
            ));
        }

        let call = GuestCall {
            action: self.spec.id.as_str().to_owned(),
            target: target.map(|target| target.as_str().to_owned()),
            params: params.to_string(),
        };

        let guest = Arc::clone(&self.guest);
        let context = context.clone();
        let runtime = Handle::current();

        let ran = task::spawn_blocking(move || guest.run(&call, context, runtime))
            .await
            .map_err(|_| ActionError::failed("the extension stopped unexpectedly"))?;

        match ran {
            Ok(Ok(text)) => Ok(output(text)),
            Ok(Err(error)) => Err(ActionError::failed(format!(
                "{}: {}",
                error.code, error.message
            ))),
            Err(error) => Err(ActionError::failed(error.to_string())),
        }
    }
}

fn output(text: String) -> Value {
    serde_json::from_str(&text).unwrap_or(Value::String(text))
}
