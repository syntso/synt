use synt_proto::Notification;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{broadcast, mpsc};

use super::Event;

pub(crate) async fn notifications(
    mut receiver: broadcast::Receiver<Notification>,
    events: mpsc::Sender<Event>,
) {
    loop {
        let notification = match receiver.recv().await {
            Ok(notification) => notification,
            Err(RecvError::Lagged(_)) => continue,
            Err(RecvError::Closed) => break,
        };

        let event = Event::Notification {
            method: notification.method,
            params: notification.params.to_string(),
        };

        if events.send(event).await.is_err() {
            return;
        }
    }

    events.send(Event::Disconnected).await.ok();
}
