mod http;
mod manifest;
mod settings;
mod view;

use std::io;
use std::path::PathBuf;
use std::time::Duration;

use synt_actions::ActionError;
use thiserror::Error;

pub use http::HttpError;
pub use manifest::ManifestError;
pub use settings::SettingsError;
pub use view::ViewError;

use crate::files::FileAccess;
use crate::interface::GuestError;
use crate::manifest::Api;
use crate::runtime::QuotaSpent;

#[derive(Debug, Error)]
pub enum ExtensionError {
    #[error("extension files: {0}")]
    Io(#[from] io::Error),

    #[error("extension state: {0}")]
    State(#[from] rusqlite::Error),

    #[error("extension.toml: {0}")]
    Manifest(#[from] ManifestError),

    #[error("extension.toml: {0}")]
    ManifestText(#[from] toml::de::Error),

    #[error("extension.toml: {0}")]
    ManifestWrite(#[from] toml::ser::Error),

    #[error("the component cannot be used: {0}")]
    Component(String),

    #[error(
        "the component cannot be used: action {action:?} must be named {extension}.<name>, with a lowercase name"
    )]
    ActionOutsideNamespace { action: String, extension: String },

    #[error("the component cannot be used: action {action:?}: {source}")]
    ActionId {
        action: String,
        source: synt_proto::ProtoError,
    },

    #[error(
        "the component cannot be used: action {action} needs a one-line title of at most {longest} bytes"
    )]
    ActionTitle { action: String, longest: usize },

    #[error("the component cannot be used: the params schema of {action} is over {largest} bytes")]
    SchemaTooLarge { action: String, largest: usize },

    #[error("the component cannot be used: the params schema of {action} is not JSON: {source}")]
    SchemaNotJson {
        action: String,
        source: serde_json::Error,
    },

    #[error("the component cannot be used: the params schema of {0} must be a JSON object")]
    SchemaNotObject(String),

    #[error(
        "the component cannot be used: an extension adds 1 to {most} actions, this one describes {found}"
    )]
    ActionCount { most: usize, found: usize },

    #[error("the component cannot be used: action {0} is described twice")]
    DuplicateAction(String),

    #[error("the component cannot be used: {0} described its actions twice")]
    DescribedTwice(String),

    #[error(
        "the component cannot be used: the manifest contributes {0}, which the extension does not describe"
    )]
    Undescribed(String),

    #[error("a timer name holds 1 to {longest} letters, digits, '-', '_' and '.'")]
    TimerName { longest: usize },

    #[error("a timer fires every {shortest} to {longest} seconds, not {asked}")]
    TimerInterval {
        shortest: u32,
        longest: u32,
        asked: u32,
    },

    #[error("an extension keeps at most {0} timers")]
    TooManyTimers(usize),

    #[error("the core is not ready to run extensions yet")]
    NotConnected,

    #[error("the extension failed: {0}")]
    Trapped(String),

    #[error("the extension ran out of fuel")]
    OutOfFuel,

    #[error("the extension ran past its time limit of {0:?}")]
    TimedOut(Duration),

    #[error("the extension made {0}")]
    OverQuota(QuotaSpent),

    #[error("{0}")]
    Guest(#[from] GuestError),

    #[error("{0}")]
    Http(#[from] HttpError),

    #[error(
        "the component cannot be used: the component is not an extension for api {api}: {reason}"
    )]
    WrongApi { api: Api, reason: String },

    #[error("the extension was stopped")]
    Stopped,

    #[error("the registry cannot be reached: {0}")]
    Fetch(String),

    #[error("the registry index is damaged: {0}")]
    Index(serde_json::Error),

    #[error("the registry index names the file {0:?}")]
    ReleaseFile(String),

    #[error("the signature does not check out: {0}")]
    Signature(String),

    #[error("{extension} owns no document type with the suffix .{suffix}")]
    NoDocumentType { extension: String, suffix: String },

    #[error("the core has no projects to keep documents in")]
    NoProjects,

    #[error("no extension {0} is installed")]
    NotInstalled(String),

    #[error("the registry has no extension {0}")]
    NotListed(String),

    #[error("{0:?} is not a sha256 digest")]
    NotSha256(String),

    #[error(
        "the signature does not check out: the registry index is older than one already seen \
         (serial {serial} < {seen})"
    )]
    IndexRolledBack { serial: u64, seen: u64 },

    #[error("the listed version {version} is not semver: {source}")]
    NotSemver {
        version: String,
        source: semver::Error,
    },

    #[error("the registry offers {id} {offered}, older than the installed {current}")]
    Downgrade {
        id: String,
        offered: String,
        current: String,
    },

    #[error("the storage of {extension} would hold {total} bytes; the limit is {limit}")]
    StorageFull {
        extension: String,
        total: u64,
        limit: u64,
    },

    #[error("{extension} already holds {most} picked files; forget some first")]
    TooManyPicks { extension: String, most: u64 },

    #[error("{0}")]
    Settings(#[from] SettingsError),

    #[error("{0} contributes no settings")]
    NoSettings(String),

    #[error("stored settings: {0}")]
    StoredSettings(serde_json::Error),

    #[error("the id {0} is taken by the core's own actions")]
    IdTaken(String),

    #[error("capabilities.call names {0}, which is not a core action an extension may call")]
    NotCallable(String),

    #[error("the installed files of {0} changed since they were installed; install it again")]
    FilesChanged(String),

    #[error("{}: {source}", .path.display())]
    BuildManifest { path: PathBuf, source: io::Error },

    #[error(
        "no extension.wasm in {} and no build in {}; run cargo build --release --target \
         wasm32-wasip2 first",
        .folder.display(),
        .output.display()
    )]
    NoComponent { folder: PathBuf, output: PathBuf },

    #[error(
        "{} holds several .wasm files; put the one to install at {}",
        .output.display(),
        .beside.display()
    )]
    SeveralComponents { output: PathBuf, beside: PathBuf },

    #[error("{} is {size} bytes; the limit is {limit}", .path.display())]
    ComponentTooLarge {
        path: PathBuf,
        size: u64,
        limit: u64,
    },

    #[error("{extension} may not {} files the person picks", .access.as_str())]
    FilesNotGranted {
        extension: String,
        access: FileAccess,
    },

    #[error("a picked file is named by its absolute path")]
    PickRelative,

    #[error("{} is not a regular file", .0.display())]
    PickNotFile(PathBuf),

    #[error("{}: {source}", .path.display())]
    PickUnreadable { path: PathBuf, source: io::Error },

    #[error("the folder of {} does not exist", .0.display())]
    PickFolderMissing(PathBuf),

    #[error("system random source: {0}")]
    Random(getrandom::Error),
}

impl From<wasmtime::Error> for ExtensionError {
    fn from(error: wasmtime::Error) -> Self {
        Self::Component(format!("{error:#}"))
    }
}

impl From<minisign_verify::Error> for ExtensionError {
    fn from(error: minisign_verify::Error) -> Self {
        Self::Signature(error.to_string())
    }
}

impl From<ExtensionError> for ActionError {
    fn from(error: ExtensionError) -> Self {
        Self::failed(error.to_string())
    }
}
