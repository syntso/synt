use super::{ExtensionEvent, TimerName};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Wake {
    Start,
    Event(ExtensionEvent),
    Timer(TimerName),
}

impl Wake {
    pub(crate) fn describe(&self) -> String {
        match self {
            Self::Start => "start".to_owned(),
            Self::Event(event) => format!("event {}", event.topic().as_str()),
            Self::Timer(name) => format!("timer {name}"),
        }
    }
}
