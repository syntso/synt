use std::sync::Arc;

use serde_json::Value;
use synt_journal::ResourceRef;

use super::DynamicAction;
use crate::prepared::{Runnable, Running};
use crate::{Completed, Context};

pub(super) struct DynamicBound<D> {
    action: Arc<D>,
    target: Option<ResourceRef>,
    params: Value,
}

impl<D: DynamicAction> DynamicBound<D> {
    pub(super) fn new(action: Arc<D>, target: Option<ResourceRef>, params: Value) -> Self {
        Self {
            action,
            target,
            params,
        }
    }
}

impl<D: DynamicAction> Runnable for DynamicBound<D> {
    fn run(self: Box<Self>, context: &Context) -> Running<'_> {
        let Self {
            action,
            target,
            params,
        } = *self;

        Box::pin(async move {
            let value = action.run(context, target, params).await?;

            Ok(Completed {
                value,
                summary: Value::Null,
            })
        })
    }
}
