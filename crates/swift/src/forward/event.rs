use crate::CoreListener;

pub(crate) enum Event {
    Notification { method: String, params: String },
    StreamData { stream: u32, data: Vec<u8> },
    StreamEnded { stream: u32, ended: String },
    Disconnected,
}

impl Event {
    pub(crate) fn deliver(self, listener: &dyn CoreListener) {
        match self {
            Self::Notification { method, params } => listener.notification(method, params),
            Self::StreamData { stream, data } => listener.stream_data(stream, data),
            Self::StreamEnded { stream, ended } => listener.stream_ended(stream, ended),
            Self::Disconnected => listener.disconnected(),
        }
    }
}
