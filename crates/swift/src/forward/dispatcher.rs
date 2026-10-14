use std::sync::Arc;
use std::thread;

use tokio::sync::mpsc;

use super::Event;
use crate::{CoreListener, SyntError};

const WAITING_EVENTS: usize = 1024;
const THREAD_NAME: &str = "synt-listener";

pub(crate) fn start_dispatcher(
    listener: Arc<dyn CoreListener>,
) -> Result<mpsc::Sender<Event>, SyntError> {
    let (sender, receiver) = mpsc::channel(WAITING_EVENTS);

    thread::Builder::new()
        .name(THREAD_NAME.to_owned())
        .spawn(move || deliver_all(receiver, listener.as_ref()))
        .map_err(|error| SyntError::Failed {
            reason: format!("cannot start the listener thread: {error}"),
        })?;

    Ok(sender)
}

fn deliver_all(mut receiver: mpsc::Receiver<Event>, listener: &dyn CoreListener) {
    while let Some(event) = receiver.blocking_recv() {
        event.deliver(listener);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex, mpsc as std_mpsc};
    use std::time::Duration;

    use super::start_dispatcher;
    use crate::CoreListener;
    use crate::forward::Event;

    struct BlockingListener {
        release: Mutex<std_mpsc::Receiver<()>>,
        seen: Mutex<Vec<String>>,
        done: Mutex<std_mpsc::Sender<()>>,
    }

    impl CoreListener for BlockingListener {
        fn notification(&self, method: String, _params: String) {
            let thread = std::thread::current().name().unwrap_or_default().to_owned();
            self.release.lock().unwrap().recv().unwrap();
            self.seen
                .lock()
                .unwrap()
                .push(format!("{method} on {thread}"));
        }

        fn stream_data(&self, stream: u32, data: Vec<u8>) {
            self.seen
                .lock()
                .unwrap()
                .push(format!("{stream}: {data:?}"));
        }

        fn stream_ended(&self, stream: u32, ended: String) {
            self.seen
                .lock()
                .unwrap()
                .push(format!("{stream} ended {ended}"));
        }

        fn disconnected(&self) {
            self.seen.lock().unwrap().push("disconnected".to_owned());
            self.done.lock().unwrap().send(()).unwrap();
        }
    }

    #[tokio::test]
    async fn a_blocked_listener_does_not_block_the_runtime() {
        let (release, released) = std_mpsc::channel();
        let (finished, done) = std_mpsc::channel();
        let listener = Arc::new(BlockingListener {
            release: Mutex::new(released),
            seen: Mutex::new(Vec::new()),
            done: Mutex::new(finished),
        });
        let events = start_dispatcher(Arc::clone(&listener) as Arc<dyn CoreListener>).unwrap();

        events
            .send(Event::Notification {
                method: "session.block".to_owned(),
                params: "{}".to_owned(),
            })
            .await
            .unwrap();
        events
            .send(Event::StreamData {
                stream: 3,
                data: vec![1],
            })
            .await
            .unwrap();
        let (answer, reply) = tokio::sync::oneshot::channel();
        tokio::spawn(async move { answer.send(()).unwrap() });
        reply.await.unwrap();

        release.send(()).unwrap();
        events.send(Event::Disconnected).await.unwrap();
        done.recv_timeout(Duration::from_secs(5)).unwrap();

        assert_eq!(
            *listener.seen.lock().unwrap(),
            vec![
                "session.block on synt-listener".to_owned(),
                "3: [1]".to_owned(),
                "disconnected".to_owned(),
            ]
        );
    }
}
