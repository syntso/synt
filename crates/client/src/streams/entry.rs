use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;

use super::{STREAM_BACKLOG, StreamEvent};

pub(super) enum StreamEntry {
    Unclaimed {
        sender: Option<mpsc::Sender<StreamEvent>>,
        receiver: mpsc::Receiver<StreamEvent>,
    },
    Claimed(mpsc::Sender<StreamEvent>),
    Released,
}

impl StreamEntry {
    pub(super) fn unclaimed() -> Self {
        let (sender, receiver) = mpsc::channel(STREAM_BACKLOG);

        Self::Unclaimed {
            sender: Some(sender),
            receiver,
        }
    }

    pub(super) fn send(&self, event: StreamEvent) -> Option<TrySendError<StreamEvent>> {
        let sender = match self {
            Self::Unclaimed {
                sender: Some(sender),
                ..
            }
            | Self::Claimed(sender) => sender,
            Self::Unclaimed { sender: None, .. } | Self::Released => return None,
        };

        sender.try_send(event).err()
    }
}
