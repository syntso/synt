use std::sync::Arc;

use crate::runnable::{Runnable, Running};
use crate::{Action, ActionError, Completed, Context};

pub(crate) struct Bound<A: Action> {
    action: Arc<A>,
    params: A::Params,
}

impl<A: Action> Bound<A> {
    pub(crate) fn new(action: Arc<A>, params: A::Params) -> Self {
        Self { action, params }
    }
}

impl<A: Action> Runnable for Bound<A> {
    fn run(self: Box<Self>, context: &Context) -> Running<'_> {
        let Self { action, params } = *self;

        Box::pin(async move {
            let output = action.run(context, params).await?;
            let summary = action.summary(&output);

            let value = serde_json::to_value(output)
                .map_err(|error| ActionError::failed(error.to_string()))?;

            Ok(Completed { value, summary })
        })
    }
}
