use serde_json::json;
use synt_client::{StreamEvent, StreamReceiver};
use tokio::sync::mpsc;

use super::Event;

const DROPPED: &str = "dropped";

pub(crate) async fn stream(mut receiver: StreamReceiver, events: mpsc::Sender<Event>) {
    let stream = receiver.stream().0;

    let ended = loop {
        let data = match receiver.next().await {
            Some(StreamEvent::Data(data)) => data,
            Some(StreamEvent::Ended(ended)) => {
                break serde_json::to_string(&ended).unwrap_or_default();
            }
            None => break dropped(stream),
        };

        let event = Event::StreamData {
            stream,
            data: data.to_vec(),
        };

        if events.send(event).await.is_err() {
            return;
        }
    };

    events.send(Event::StreamEnded { stream, ended }).await.ok();
}

fn dropped(stream: u32) -> String {
    json!({ "stream": stream, "reason": DROPPED }).to_string()
}

#[cfg(test)]
mod tests {
    use super::dropped;

    #[test]
    fn a_dropped_stream_says_so() {
        assert_eq!(dropped(7), r#"{"reason":"dropped","stream":7}"#);
    }
}
