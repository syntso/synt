use tokio::sync::mpsc;

use crate::StreamEvent;

pub(super) enum StreamEntry {
    Unclaimed {
        sender: Option<mpsc::Sender<StreamEvent>>,
        receiver: mpsc::Receiver<StreamEvent>,
    },
    Claimed(mpsc::Sender<StreamEvent>),
    Released,
}
