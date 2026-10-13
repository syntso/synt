use std::future::Future;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::{ActionError, Context, Risk};

pub trait Action: Send + Sync + 'static {
    const ID: &'static str;
    const RISK: Risk;

    type Params: DeserializeOwned + Send;
    type Output: Serialize;

    fn run(
        &self,
        context: &Context,
        params: Self::Params,
    ) -> impl Future<Output = Result<Self::Output, ActionError>> + Send;
}
