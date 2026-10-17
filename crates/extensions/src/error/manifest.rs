use synt_proto::ProtoError;
use thiserror::Error;

use crate::manifest::{AssetPath, ExtensionId, HttpHost, LocalId, OneLine, SecretName, Suffix};

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error(
        "id {0:?} must start with a lowercase letter and hold only lowercase letters, digits \
         and dashes, at most {most} of them",
        most = ExtensionId::LONGEST
    )]
    ExtensionId(String),

    #[error(
        "{0:?} must start with a lowercase letter and hold only lowercase letters, digits, \
         '-' and '_', at most {most} of them",
        most = LocalId::LONGEST
    )]
    LocalId(String),

    #[error("{0:?} must be one line of at most {most} bytes", most = OneLine::LONGEST)]
    NotOneLine(String),

    #[error("description is too long or holds control characters")]
    Description,

    #[error(
        "{0:?} is not a plain path of at most {most} bytes inside the bundle",
        most = AssetPath::LONGEST
    )]
    AssetPath(String),

    #[error(
        "http host {0:?} must be a lowercase host name or IPv4 address, with an optional \
         :port; patterns and schemes are not allowed"
    )]
    HttpHost(String),

    #[error(
        "secret {0:?} must be 1 to {most} letters, digits, '-' and '_', with '/' between parts",
        most = SecretName::LONGEST
    )]
    SecretName(String),

    #[error(
        "document suffix {0:?} must be 1 to {most} lowercase letters and digits",
        most = Suffix::LONGEST
    )]
    Suffix(String),

    #[error("action {action:?}: {source}")]
    ActionName { action: String, source: ProtoError },

    #[error("action {0:?}: name one action, not a pattern")]
    ActionPattern(String),

    #[error("api {api} is not supported; this core runs extensions for api {supported}")]
    UnsupportedApi { api: String, supported: String },

    #[error(
        "contributes and the capabilities http, secrets, documents, events, timers and files \
         need api 0.2"
    )]
    NeedsNewerApi,

    #[error("call {0:?}: an extension runs its own actions without calling them")]
    CallsItself(String),

    #[error("secret {0} names no host it may be sent to")]
    SecretWithoutHost(SecretName),

    #[error("secret {secret} may go to {host}, which is not listed in capabilities.http")]
    SecretHostNotListed { secret: SecretName, host: HttpHost },

    #[error("capabilities.documents needs at least one contributes.document type")]
    DocumentsWithoutType,

    #[error("contributes.document needs documents = true in capabilities")]
    TypeWithoutDocuments,

    #[error(
        "contributes names the action {0}; commands and menus run the extension's own actions \
         only"
    )]
    ForeignAction(String),

    #[error("contributes.{point} has {size} entries; the most is {most}")]
    TooManyEntries {
        point: &'static str,
        size: usize,
        most: usize,
    },

    #[error("{field} {value:?} is used twice")]
    Duplicate { field: &'static str, value: String },

    #[error("a document type opens in panel {0}, which is not contributed")]
    MissingPanel(LocalId),

    #[error("panel {0} needs either a native view or a web entry, not both")]
    PanelKind(LocalId),

    #[error("settings.schema must be a JSON Schema of type object with properties")]
    SettingsSchema,
}
