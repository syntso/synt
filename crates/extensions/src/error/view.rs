use thiserror::Error;

#[derive(Debug, Error)]
pub enum ViewError {
    #[error("{0}")]
    Malformed(#[from] serde_json::Error),

    #[error("a view holds at most {most} nodes, {deepest} deep")]
    TooLarge { most: usize, deepest: usize },

    #[error("a log holds at most {0} lines of text")]
    LongLog(usize),

    #[error("every table row holds one plain value per column")]
    RaggedTable,

    #[error("a key-value item needs a plain value")]
    KeyValue,

    #[error("a view may bind only the extension's own actions, not {0}")]
    ForeignAction(String),

    #[error("a view patch holds 1 to {0} operations")]
    PatchSize(usize),

    #[error("every view patch operation needs a JSON pointer path")]
    PatchPath,
}
