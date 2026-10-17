mod spec;

use std::sync::Arc;

use serde_json::Value;
use synt_actions::{ActionError, Context, DynamicAction, Handle, Risk, blocking};
use synt_journal::ResourceRef;
use tokio::runtime::Handle as RuntimeHandle;

pub(crate) use spec::ActionSpec;

use super::Guest;
use crate::builtin::CallingActor;
use crate::runtime::GuestCall;

pub(crate) struct ExtensionAction {
    spec: ActionSpec,
    guest: Arc<Guest>,
}

impl ExtensionAction {
    pub(crate) fn new(spec: ActionSpec, guest: Arc<Guest>) -> Self {
        Self { spec, guest }
    }

    pub(crate) fn into_handle(self) -> Handle {
        let id = self.spec.id().clone();

        Handle::dynamic(id, self)
    }
}

impl DynamicAction for ExtensionAction {
    fn title(&self) -> &str {
        self.spec.title()
    }

    fn risk(&self) -> Risk {
        self.spec.risk()
    }

    fn takes_target(&self) -> bool {
        self.spec.takes_target()
    }

    fn params_schema(&self) -> Value {
        self.spec.params_schema().clone()
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
        context.refuse_extensions("run the actions of an extension")?;

        let call = GuestCall::new(
            self.spec.id().as_str().to_owned(),
            target.map(|target| target.as_str().to_owned()),
            params.to_string(),
        );
        let guest = Arc::clone(&self.guest);
        let context = context.clone();
        let runtime = RuntimeHandle::current();

        let reply =
            blocking::<_, _, ActionError>(move || guest.run(&call, context, runtime)).await?;

        Ok(reply.into_value())
    }
}
