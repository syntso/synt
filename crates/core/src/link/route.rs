use bytes::Bytes;
use tokio::sync::mpsc;

use super::Delivery;

pub(crate) struct Route {
    pub(crate) input: mpsc::Sender<Bytes>,
    pub(crate) delivery: Delivery,
}
