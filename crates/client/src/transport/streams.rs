use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use synt_proto::StreamId;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;

use super::StreamEntry;
use crate::{StreamEvent, StreamReceiver};

const STREAM_BACKLOG: usize = 512;
const ENDED_UNCLAIMED_LIMIT: usize = 16;

#[derive(Default)]
pub(crate) struct Streams {
    entries: Mutex<HashMap<StreamId, StreamEntry>>,
    ended_unclaimed: Mutex<VecDeque<StreamId>>,
}

impl Streams {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn claim(self: &Arc<Self>, stream: StreamId) -> StreamReceiver {
        let mut entries = self.lock();

        let receiver = match entries.remove(&stream) {
            Some(StreamEntry::Unclaimed { sender, receiver }) => {
                if let Some(sender) = sender {
                    entries.insert(stream, StreamEntry::Claimed(sender));
                }
                receiver
            }
            Some(StreamEntry::Claimed(_) | StreamEntry::Released) | None => {
                let (sender, receiver) = mpsc::channel(STREAM_BACKLOG);
                entries.insert(stream, StreamEntry::Claimed(sender));
                receiver
            }
        };
        drop(entries);

        StreamReceiver::new(stream, receiver, Arc::clone(self))
    }

    pub(crate) fn release(&self, stream: StreamId) {
        let mut entries = self.lock();

        if let Some(entry @ StreamEntry::Claimed(_)) = entries.get_mut(&stream) {
            *entry = StreamEntry::Released;
        }
    }

    pub(crate) fn deliver(&self, stream: StreamId, event: StreamEvent) {
        let mut entries = self.lock();

        let entry = entries.entry(stream).or_insert_with(|| {
            let (sender, receiver) = mpsc::channel(STREAM_BACKLOG);
            StreamEntry::Unclaimed {
                sender: Some(sender),
                receiver,
            }
        });

        let sent = match entry {
            StreamEntry::Unclaimed {
                sender: Some(sender),
                ..
            }
            | StreamEntry::Claimed(sender) => sender.try_send(event),
            StreamEntry::Unclaimed { sender: None, .. } | StreamEntry::Released => return,
        };

        if let Err(TrySendError::Full(_)) = sent {
            *entry = StreamEntry::Released;
        }
    }

    pub(crate) fn end(&self, stream: StreamId) {
        let mut entries = self.lock();

        match entries.get_mut(&stream) {
            Some(StreamEntry::Unclaimed { sender, .. }) => {
                *sender = None;
                self.keep_ended(&mut entries, stream);
            }
            Some(StreamEntry::Claimed(_) | StreamEntry::Released) => {
                entries.remove(&stream);
            }
            None => {}
        }
    }

    pub(crate) fn close(&self) {
        let closed: Vec<StreamEntry> = self.lock().drain().map(|(_, entry)| entry).collect();
        drop(closed);
    }

    fn keep_ended(&self, entries: &mut HashMap<StreamId, StreamEntry>, stream: StreamId) {
        let mut ended = self
            .ended_unclaimed
            .lock()
            .unwrap_or_else(PoisonError::into_inner);

        ended.push_back(stream);

        while ended.len() > ENDED_UNCLAIMED_LIMIT {
            let Some(oldest) = ended.pop_front() else {
                break;
            };

            if let Some(StreamEntry::Unclaimed { sender: None, .. }) = entries.get(&oldest) {
                entries.remove(&oldest);
            }
        }
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<StreamId, StreamEntry>> {
        self.entries.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
