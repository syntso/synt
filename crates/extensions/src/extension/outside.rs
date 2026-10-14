use std::sync::Arc;

use synt_actions::Context;
use tokio::runtime::Handle;

use crate::call::Caller;

pub(crate) struct Outside {
    pub(crate) caller: Arc<dyn Caller>,
    pub(crate) origin: Option<Context>,
    pub(crate) runtime: Handle,
}
