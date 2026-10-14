use std::future::Future;
use std::pin::Pin;

use crate::{ActionError, Completed, Context};

pub(crate) type Running<'a> =
    Pin<Box<dyn Future<Output = Result<Completed, ActionError>> + Send + 'a>>;

pub(crate) trait Runnable: Send {
    fn run(self: Box<Self>, context: &Context) -> Running<'_>;
}
