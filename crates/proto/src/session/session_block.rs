use serde::{Deserialize, Serialize};

use crate::{Notice, StreamId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionBlock {
    pub session: u64,
    pub stream: StreamId,
    pub block: u64,

    #[serde(default)]
    pub command: Option<String>,

    #[serde(default)]
    pub exit: Option<i32>,

    pub duration_ms: u64,
    pub output_start: u64,
    pub output_end: u64,
}

impl Notice for SessionBlock {
    const METHOD: &'static str = "session.block";
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::SessionBlock;
    use crate::{Notice, StreamId};

    #[test]
    fn a_block_travels_as_a_notification() {
        let block = SessionBlock {
            session: 57,
            stream: StreamId(1),
            block: 3,
            command: Some("false".to_owned()),
            exit: Some(1),
            duration_ms: 4,
            output_start: 120,
            output_end: 120,
        };

        let notification = block.to_notification().unwrap();

        assert_eq!(notification.method, "session.block");
        assert_eq!(notification.params::<SessionBlock>().unwrap(), block);
        assert_eq!(
            serde_json::to_value(&block).unwrap()["command"],
            json!("false")
        );
    }
}
