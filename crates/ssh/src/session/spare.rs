use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use russh::Channel;
use russh::client::{Handle, Msg};
use synt_platform::Lock;
use tokio::runtime;

use super::ClientHandler;

#[derive(Default)]
pub(super) struct Spare {
    channel: Mutex<Option<Channel<Msg>>>,
    filling: AtomicBool,
}

impl Spare {
    pub(super) fn take(&self) -> Option<Channel<Msg>> {
        self.channel.locked().take()
    }

    pub(super) async fn fill_now(&self, handle: &Handle<ClientHandler>) {
        if self.channel.locked().is_some() {
            return;
        }

        if let Ok(channel) = handle.channel_open_session().await {
            self.keep(channel).await;
        }
    }

    pub(super) fn fill(self: &Arc<Self>, handle: &Arc<Handle<ClientHandler>>) {
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
            let mut slot = self.channel.locked();

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
