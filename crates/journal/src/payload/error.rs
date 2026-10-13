use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DetailsError {
    #[error("details could not be serialized: {0}")]
    Serialize(String),

    #[error("details contain a floating point number")]
    Float,

    #[error("details nest deeper than {limit} levels")]
    TooDeep { limit: usize },

    #[error("details take {size} bytes, the limit is {limit}")]
    TooLarge { size: usize, limit: usize },

    #[error("details contain what looks like private key material")]
    KeyMaterial,
}
