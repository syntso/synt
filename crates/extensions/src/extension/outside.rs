use std::sync::Arc;

use synt_actions::Context;
use tokio::runtime::Handle;

use crate::caller::Caller;

pub(crate) struct Outside {
    pub(crate) caller: Arc<dyn Caller>,
    pub(crate) context: Context,
    pub(crate) runtime: Handle,
}
