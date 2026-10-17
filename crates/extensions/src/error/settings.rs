use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SettingsError {
    #[error("settings hold at most {0} bytes")]
    TooLarge(usize),

    #[error("settings are an object")]
    NotObject,

    #[error("the setting {0} is required")]
    Required(String),

    #[error("{0} is not a setting of this extension")]
    Unknown(String),

    #[error("the setting {name} must be of type {kind}")]
    WrongType { name: String, kind: String },

    #[error("the setting {0} must be one of its listed values")]
    NotListed(String),
}
