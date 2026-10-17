use std::io;
use std::path::PathBuf;

use synt_actions::ActionError;
use synt_db::Engine;
use synt_journal::{JournalError, JournalId, KeyStoreError};
use synt_platform::{KeyPlace, PlatformError};
use synt_sync::SyncError;
use thiserror::Error;
use tokio::task::JoinError;

use crate::runbooks::RunbookError;
use crate::secrets::SecretName;
use crate::ssh_keys::KeyName;
use crate::terminals::SessionId;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("data folder: {0}")]
    Io(#[from] io::Error),

    #[error("key storage: {0}")]
    Platform(#[from] PlatformError),

    #[error(transparent)]
    Journal(#[from] JournalError),

    #[error("state database: {0}")]
    State(#[from] rusqlite::Error),

    #[error("ssh: {0}")]
    Ssh(#[from] synt_ssh::SshError),

    #[error("system random source: {0}")]
    Random(#[from] getrandom::Error),

    #[error("the work stopped unexpectedly")]
    Stopped(#[from] JoinError),

    #[error("{0}")]
    Shell(#[source] synt_ssh::SshError),

    #[error("{0}")]
    Pty(#[source] PlatformError),

    #[error("could not start the shell: {0}")]
    PtyStart(#[source] PlatformError),

    #[error(transparent)]
    Runbook(#[from] RunbookError),

    #[error("{} needs one of {}", .command, .choices.join(", "))]
    CommandUsage {
        command: &'static str,
        choices: &'static [&'static str],
    },

    #[error("endpoint: {0}")]
    Endpoint(#[source] io::Error),

    #[error("this call has no connection to carry {0}")]
    NoConnection(&'static str),

    #[error("there is no open session {0}")]
    NoSession(SessionId),

    #[error("unknown argument {0}")]
    UnknownArgument(String),

    #[error("{0} needs a value")]
    MissingValue(String),

    #[error("{0} needs a whole number of minutes")]
    NotMinutes(&'static str),

    #[error("login start: {0}")]
    LoginStart(#[source] PlatformError),

    #[error("key storage item {0} is damaged")]
    DamagedKey(String),

    #[error(
        "the device key is missing from {0}; the journal cannot be checked without it, \
         and the core will not make a new key for an existing journal"
    )]
    DeviceKeyMissing(KeyPlace),

    #[error(
        "journal missing: {place} remembers journal {id}, but {journal} is gone; \
         the core will not start a new journal in its place"
    )]
    JournalMissing {
        id: JournalId,
        journal: PathBuf,
        place: KeyPlace,
    },

    #[error(
        "{journal} exists but {place} remembers no journal for it, so it cannot be checked; \
         the core will not start a new journal over it"
    )]
    UnknownJournal { journal: PathBuf, place: KeyPlace },

    #[error("a {noun} name must be 1 to {longest} characters")]
    VaultNameLength { noun: &'static str, longest: usize },

    #[error(
        "{text:?} is not a valid {noun} name: use letters, digits, '-' and '_', \
         with '/' between parts"
    )]
    InvalidVaultName { noun: &'static str, text: String },

    #[error("there is already a key named {0}; remove it first")]
    KeyExists(KeyName),

    #[error("secret://{0} already names a secret; pick another name")]
    KeyNamesSecret(KeyName),

    #[error("there is no key named {0}")]
    NoKey(KeyName),

    #[error("a secret holds 1 to {0} bytes")]
    SecretSize(usize),

    #[error("secret://{0} already names an SSH key; pick another name")]
    SecretNamesKey(SecretName),

    #[error("there is no secret {0}")]
    NoSecret(SecretName),

    #[error("secret secret://{0}: there is no such secret in the vault; store it with secret.put")]
    SecretNotInVault(SecretName),

    #[error("secret secret://{0} is not UTF-8 text")]
    SecretNotText(SecretName),

    #[error(
        "secret secret://{name} is kept for {bound}, so it is not sent to {used_for}; if it \
         should go there now, put it again with secret.put"
    )]
    SecretKeptFor {
        name: SecretName,
        bound: String,
        used_for: String,
    },

    #[error("cannot open {path}: {source}", path = .path.display())]
    UnreadableFolder {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("{} is not a folder", .0.display())]
    NotAFolder(PathBuf),

    #[error("the folder name of {} cannot name a project", .0.display())]
    UnnamedFolder(PathBuf),

    #[error("a project folder cannot be called {0}: the name belongs to the Personal project")]
    PersonalSlug(String),

    #[error("a project folder cannot be called {0}: names of that shape are project ids")]
    IdShapedSlug(String),

    #[error("a project named {slug} is already open from {path}", path = .path.display())]
    SlugTaken { slug: String, path: PathBuf },

    #[error("project {other} is already open with the same id {id}")]
    IdTaken { other: String, id: String },

    #[error("no open project is called {0}")]
    ProjectNotOpen(String),

    #[error("the Personal project is built in and cannot be closed")]
    PersonalCannotClose,

    #[error("the Personal project is read-only; edit {0} or the kubeconfig instead")]
    PersonalReadOnly(String),

    #[error(
        "the Personal project is ~/.ssh/config and holds no files of its own, so it does not \
         sync; open a project folder"
    )]
    PersonalHasNoFiles,

    #[error("the Personal project has no databases; put them in a project")]
    PersonalHasNoDatabases,

    #[error("there is no Host {name} in {config} and no kubeconfig context of that name")]
    NoPersonalResource { name: String, config: String },

    #[error("{0:?} is not a project target; write @<project>/<resource>")]
    NotATarget(String),

    #[error("{0:?} is not a project resource; write @<project>/<resource>")]
    NotAProjectResource(String),

    #[error("{0:?} is not a database resource; write @<project>/<database>")]
    NotADatabaseTarget(String),

    #[error("{text} is a {kind}, not a cluster")]
    NotACluster { text: String, kind: &'static str },

    #[error(
        "the kubeconfig {kubeconfig} lies inside the project folder {folder}; files there can \
         arrive from other members through sync, so keep kubeconfigs outside every project \
         folder",
        kubeconfig = .kubeconfig.display(),
        folder = .folder.display()
    )]
    KubeconfigInsideProject {
        kubeconfig: PathBuf,
        folder: PathBuf,
    },

    #[error(
        "@{slug}/{name} is now {actual}, which is not what the call was admitted for; \
         run it again"
    )]
    AdmittedElsewhere {
        slug: String,
        name: String,
        actual: String,
    },

    #[error("cannot watch {path}: {source}", path = .path.display())]
    Watch {
        path: PathBuf,
        #[source]
        source: notify::Error,
    },

    #[error("{path}: {reason}", path = .path.display())]
    ProjectFile {
        path: PathBuf,
        reason: Box<CoreError>,
    },

    #[error("the file vanished after it was written")]
    Vanished,

    #[error(
        "{path} needs an id of {length} lowercase hex characters",
        path = .0.display(),
        length = crate::projects::ProjectId::LENGTH
    )]
    ProjectIdMissing(PathBuf),

    #[error(transparent)]
    Sync(#[from] SyncError),

    #[error("{} must be a plain folder, not a link", .0.display())]
    LinkedFolder(PathBuf),

    #[error("project {slug} has no resource {name}")]
    NoResource { slug: String, name: String },

    #[error("project {slug} has no resource {name}: its file is broken: {problem}")]
    BrokenFile {
        slug: String,
        name: String,
        problem: String,
    },

    #[error("@{slug}/{name}: {reason}")]
    BrokenResource {
        slug: String,
        name: String,
        reason: Box<CoreError>,
    },

    #[error("its via chain loops through {0}")]
    LoopingChain(String),

    #[error("its via chain is longer than {0} hops")]
    LongChain(usize),

    #[error("it is empty")]
    EmptyChain,

    #[error("@{slug}/{name} is a database; reach it with the db.* actions")]
    IsADatabase { slug: String, name: String },

    #[error("@{slug}/{name} is a {kind}, not a database")]
    NotADatabase {
        slug: String,
        name: String,
        kind: &'static str,
    },

    #[error("{name} in project {slug} is a {kind}, not a host")]
    NotAHost {
        slug: String,
        name: String,
        kind: &'static str,
    },

    #[error("cannot write {name}: {reason}")]
    ResourceWrite {
        name: String,
        reason: Box<CoreError>,
    },

    #[error("{}", .0.message())]
    TomlSyntax(#[from] toml::de::Error),

    #[error("{0}")]
    TomlWrite(#[from] toml::ser::Error),

    #[error("{0}")]
    Unreadable(#[source] io::Error),

    #[error("a name must be 1 to {0} characters")]
    NameLength(usize),

    #[error("{0:?} is not a valid name: use letters, digits, '-', '_' and '.', not first")]
    InvalidName(String),

    #[error("{field}: {reason}")]
    Field {
        field: &'static str,
        reason: Box<CoreError>,
    },

    #[error("identity {identity:?}: {reason}")]
    IdentityName {
        identity: String,
        reason: Box<CoreError>,
    },

    #[error(
        "identity {0:?} must be \"agent\", \"secret://<name>\", an absolute path, \
         or a path starting with ~/"
    )]
    NotAnIdentity(String),

    #[error("{field} must be a host name or address, got {value:?}")]
    NotAnAddress { field: &'static str, value: String },

    #[error("{0} must be between 1 and 65535")]
    PortZero(&'static str),

    #[error("user must be a login name, got {0:?}")]
    NotALogin(String),

    #[error("no user given and none found in the environment")]
    NoLocalUser,

    #[error("{0} is missing")]
    MissingField(&'static str),

    #[error("context must be a context name, got {0:?}")]
    NotAContext(String),

    #[error(
        "namespace must be lowercase letters, digits and '-', at most {longest} characters, \
         got {namespace:?}"
    )]
    NotANamespace { namespace: String, longest: usize },

    #[error("remote must be host:port, such as orders-db:5432, got {0:?}")]
    NotARemote(String),

    #[error("there is no home folder to expand ~ in: {0}")]
    NoHome(#[source] io::Error),

    #[error("{0:?} must be an absolute path or a path starting with ~/")]
    NotAbsolute(String),

    #[error("the path is empty")]
    EmptyPath,

    #[error("the path contains a NUL character")]
    NulInPath,

    #[error("a sqlite database is a file: give its path, not {0}")]
    SqliteTakesPath(&'static str),

    #[error("a {0} database is reached over the network: give host, not path")]
    NetworkTakesHost(Engine),

    #[error("redis does not negotiate tls; choose disable, require or verify-full")]
    RedisPrefer,

    #[error("a redis database is a number such as 0")]
    RedisDatabase,

    #[error(
        "password must name a secret kept by the core, such as secret://orders-db; \
         store it with secret.put"
    )]
    PasswordNotSecret,
}

impl CoreError {
    pub(crate) fn field(field: &'static str, reason: Self) -> Self {
        Self::Field {
            field,
            reason: Box::new(reason),
        }
    }
}

impl From<CoreError> for ActionError {
    fn from(error: CoreError) -> Self {
        match error {
            CoreError::Runbook(error) => error.into(),
            other => Self::Failed(other.to_string()),
        }
    }
}

impl From<CoreError> for KeyStoreError {
    fn from(error: CoreError) -> Self {
        Self::new(error)
    }
}

impl From<CoreError> for SyncError {
    fn from(error: CoreError) -> Self {
        Self::Projects(Box::new(error))
    }
}
