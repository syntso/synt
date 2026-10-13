use std::future::Future;

use super::{AccessRequest, Decision};

pub trait Gate: Send + Sync + 'static {
    fn decide(&self, request: &AccessRequest<'_>) -> impl Future<Output = Decision> + Send;
}
