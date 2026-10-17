use std::error::Error as StdError;
use std::io;
use std::path::PathBuf;
use std::str::Utf8Error;
use std::time::Duration;

use synt_actions::ActionError;
use synt_platform::{PlatformError, SecretName};
use synt_proto::ProtoError;
use thiserror::Error;
use tokio::task::JoinError;

use crate::files::FileAccess;
use crate::interface::GuestError;
use crate::manifest::{Api, AssetPath, ExtensionId, HttpHost, LocalId, Suffix};
use crate::runtime::Spend;
use crate::sha256::Sha256;

#[derive(Debug, Error)]
pub enum ExtensionError {
    #[error("extension files: {0}")]
    Io(#[from] io::Error),

    #[error("extension state: {0}")]
    State(#[from] rusqlite::Error),

    #[error("extension.toml: {0}")]
    ManifestText(#[from] toml::de::Error),

    #[error("extension.toml: {0}")]
    ManifestWrite(#[from] toml::ser::Error),

    #[error("the component cannot be used: {0:#}")]
    Component(wasmtime::Error),

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

    #[error("the extension made more than {} {} in one call", .0.most(), .0.what())]
    OverQuota(Spend),

    #[error("{0}")]
    Guest(GuestError),

    #[error("{0}")]
    Secret(#[source] Box<dyn StdError + Send + Sync>),

    #[error(
        "the component cannot be used: the component is not an extension for api {api}: {reason}"
    )]
    WrongApi { api: Api, reason: String },

    #[error("the extension was stopped")]
    Stopped,

    #[error("the work stopped unexpectedly")]
    Interrupted(#[from] JoinError),

    #[error("the registry cannot be reached: {url}: {source}")]
    Fetch { url: String, source: ureq::Error },

    #[error("the registry index is damaged: {0}")]
    Index(serde_json::Error),

    #[error("the registry index names the file {0:?}")]
    ReleaseFile(String),

    #[error("the signature does not check out: {0}")]
    Signature(#[from] minisign_verify::Error),

    #[error("the signature does not check out: {0}")]
    SignatureText(#[source] Utf8Error),

    #[error("the signature does not check out: the public key: {0}")]
    PublicKey(#[source] minisign_verify::Error),

    #[error("the signature does not check out: the trusted comment of the index: {0}")]
    IndexComment(#[source] serde_json::Error),

    #[error(
        "the signature does not check out: the registry index has expired; the registry has \
         not been signed lately"
    )]
    IndexExpired,

    #[error(
        "the signature does not check out: the download has sha256 {found}, the index says {listed}"
    )]
    DownloadDigest { found: Sha256, listed: Sha256 },

    #[error("the signature does not check out: the trusted comment: {0}")]
    ListingComment(#[source] serde_json::Error),

    #[error(
        "the signature does not check out: the signed id, name, version, description, \
         repository, sha256 or capabilities differ from the index"
    )]
    ListingDiffers,

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

    #[error("{method} is not one of {allowed}")]
    Method { method: String, allowed: String },

    #[error("a request carries at most {0} headers")]
    TooManyHeaders(usize),

    #[error("the core sets the {0} header itself")]
    ReservedHeader(String),

    #[error("the {0} header is filled from a secret and cannot be set as well")]
    SecretClash(String),

    #[error("timeout_ms is 1 to {longest}, not {asked}")]
    Timeout { longest: u32, asked: u32 },

    #[error("the body is not base64: {0}")]
    BodyNotBase64(data_encoding::DecodeError),

    #[error("{0:?} is not a header name")]
    HeaderName(String),

    #[error("the {0} header has a value that cannot be sent")]
    HeaderValue(String),

    #[error("the {0} header cannot carry this secret")]
    SecretHeaderValue(String),

    #[error("{url:?} is not a URL: {source}")]
    NotUrl {
        url: String,
        source: ureq::http::uri::InvalidUri,
    },

    #[error("{0:?} must start with https:// or http://")]
    Scheme(String),

    #[error("{0:?} names no host this core may reach")]
    NoHost(String),

    #[error("{0:?} carries credentials; attach a secret instead")]
    Credentials(String),

    #[error("{0} is not listed in capabilities.http")]
    HostNotListed(String),

    #[error("the core keeps no secrets to attach")]
    NoSecrets,

    #[error("secrets go only over https or to this computer, not to http://{0}")]
    SecretOverPlainHttp(String),

    #[error("capabilities.secrets does not let secret://{secret} go to {destination}")]
    SecretNotAllowed { secret: String, destination: String },

    #[error("there is no secret secret://{0}")]
    NoSuchSecret(String),

    #[error("{0} is not text and cannot go in a header")]
    SecretNotText(String),

    #[error("{0} is empty or holds control characters and cannot go in a header")]
    SecretNotHeader(String),

    #[error("{url}: {source}")]
    Request {
        url: String,
        source: ureq::http::Error,
    },

    #[error("{url}: {source}")]
    Transport { url: String, source: ureq::Error },

    #[error("the response body: {0}")]
    ResponseBody(ureq::Error),

    #[error("the response body is not base64: {0}")]
    ResponseNotBase64(data_encoding::DecodeError),

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

    #[error(transparent)]
    NotOneLine(#[from] ProtoError),

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

    #[error(transparent)]
    SecretName(#[from] PlatformError),

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
    ForeignCommand(String),

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

    #[error("settings hold at most {0} bytes")]
    SettingsTooLarge(usize),

    #[error("settings are an object")]
    SettingsNotObject,

    #[error("the setting {0} is required")]
    RequiredSetting(String),

    #[error("{0} is not a setting of this extension")]
    UnknownSetting(String),

    #[error("the setting {name} must be of type {kind}")]
    SettingType { name: String, kind: String },

    #[error("the setting {0} must be one of its listed values")]
    SettingNotListed(String),

    #[error("{0}")]
    MalformedView(#[source] serde_json::Error),

    #[error("a view holds at most {most} nodes, {deepest} deep")]
    ViewTooLarge { most: usize, deepest: usize },

    #[error("a log holds at most {0} lines of text")]
    LongLog(usize),

    #[error("every table row holds one plain value per column")]
    RaggedTable,

    #[error("a key-value item needs a plain value")]
    KeyValue,

    #[error("a view may bind only the extension's own actions, not {0}")]
    ForeignBinding(String),

    #[error("a view patch holds 1 to {0} operations")]
    PatchSize(usize),

    #[error("every view patch operation needs a JSON pointer path")]
    PatchPath,
}

impl From<wasmtime::Error> for ExtensionError {
    fn from(error: wasmtime::Error) -> Self {
        Self::Component(error)
    }
}

impl From<GuestError> for ExtensionError {
    fn from(error: GuestError) -> Self {
        Self::Guest(error)
    }
}

impl From<ExtensionError> for ActionError {
    fn from(error: ExtensionError) -> Self {
        Self::failed(error.to_string())
    }
}
