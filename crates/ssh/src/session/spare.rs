use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use russh::Channel;
use russh::client::{Handle, Msg};
use tokio::runtime;

use super::ClientHandler;
use crate::pool::locked;

#[derive(Default)]
pub(crate) struct Spare {
    channel: Mutex<Option<Channel<Msg>>>,
    filling: AtomicBool,
}

impl Spare {
    pub(crate) fn take(&self) -> Option<Channel<Msg>> {
        locked(&self.channel).take()
    }

    pub(crate) async fn fill_now(&self, handle: &Handle<ClientHandler>) {
        if locked(&self.channel).is_some() {
            return;
        }

        if let Ok(channel) = handle.channel_open_session().await {
            self.keep(channel).await;
        }
    }

    pub(crate) fn fill(self: &Arc<Self>, handle: &Arc<Handle<ClientHandler>>) {
        if handle.is_closed() || self.filling.swap(true, Ordering::AcqRel) {
            return;
        }

        let Ok(runtime) = runtime::Handle::try_current() else {
            self.filling.store(false, Ordering::Release);
            return;
        };

        let spare = Arc::clone(self);
        let handle = Arc::clone(handle);

        runtime.spawn(async move {
            spare.fill_now(&handle).await;
            spare.filling.store(false, Ordering::Release);
        });
    }

    async fn keep(&self, channel: Channel<Msg>) {
        let extra = {
            let mut slot = locked(&self.channel);

            if slot.is_some() {
                Some(channel)
            } else {
                *slot = Some(channel);
                None
            }
        };

        if let Some(extra) = extra {
            let _ = extra.close().await;
        }
    }
}
