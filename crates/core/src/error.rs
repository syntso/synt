use std::io;
use std::path::PathBuf;
use std::time::Duration;

use serde_json::Value;
use synt_actions::ActionError;
use synt_db::{DbError, Engine, StatementKind};
use synt_extensions::ExtensionError;
use synt_journal::{ActionId, JournalError, JournalId, ResourceRef, Seq};
use synt_kube::KubeError;
use synt_platform::{KeyPlace, PlatformError, SecretName};
use synt_proto::{ProtoError, RunPhase};
use synt_ssh::SshError;
use synt_sync::SyncError;
use thiserror::Error;
use tokio::task::JoinError;

use crate::builtin::ForwardId;
use crate::runbooks::{ParamName, RunbookName};
use crate::ssh_keys::KeyName;
use crate::terminals::SessionId;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("data folder: {0}")]
    Io(#[from] io::Error),

    #[error("key storage: {0}")]
    Platform(#[source] PlatformError),

    #[error(transparent)]
    VaultName(PlatformError),

    #[error(transparent)]
    Journal(#[from] JournalError),

    #[error("the revocation was not journaled: {0}")]
    NotJournaled(#[source] JournalError),

    #[error(transparent)]
    Proto(#[from] ProtoError),

    #[error("{0}")]
    Json(#[from] serde_json::Error),

    #[error("state database: {0}")]
    State(#[from] rusqlite::Error),

    #[error("{}", Self::ssh_reason(.0))]
    Ssh(#[from] SshError),

    #[error("{0}")]
    KeyImport(#[source] SshError),

    #[error("system random source: {0}")]
    Random(#[from] getrandom::Error),

    #[error("the work stopped unexpectedly")]
    Stopped(#[from] JoinError),

    #[error("{0}")]
    Shell(#[source] SshError),

    #[error("{0}")]
    Pty(#[source] PlatformError),

    #[error("could not start the shell: {0}")]
    PtyStart(#[source] PlatformError),

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

    #[error("term must be 1 to {longest} letters, digits or -_.+")]
    BadTerm { longest: usize },

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

    #[error(transparent)]
    Extension(#[from] ExtensionError),

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
    ResourceFields(#[source] serde_json::Error),

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

    #[error("{0:?} is not a host name or address")]
    NotAnAddress(String),

    #[error("local_port must be between 1 and 65535; leave it out to pick a free port")]
    LocalPortZero,

    #[error("remote_host and remote_port go together")]
    RemoteApart,

    #[error("remote_host must be a host name or address, got {0:?}")]
    NotARemoteHost(String),

    #[error("remote_port must be between 1 and 65535")]
    RemotePortZero,

    #[error("user must be a login name, got {0:?}")]
    NotALogin(String),

    #[error("no user given and none found in the environment")]
    NoLocalUser,

    #[error("{0} is missing")]
    MissingField(&'static str),

    #[error("context must be a context name, got {0:?}")]
    NotAContext(String),

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

    #[error(transparent)]
    NotOneLine(ProtoError),

    #[error(transparent)]
    BadName(Box<CoreError>),

    #[error("param {0:?}: a name starts with a letter or _ and holds only letters, digits and _")]
    BadParamName(String),

    #[error("expect field {0:?} must be a JSON pointer such as /state")]
    BadPointer(String),

    #[error("action: {0}")]
    BadAction(#[source] ProtoError),

    #[error("{0}")]
    BadReference(#[source] ProtoError),

    #[error("a runbook has at most {0} params")]
    TooManyParams(usize),

    #[error("param {0} is declared twice")]
    DeclaredTwice(ParamName),

    #[error("a runbook has 1 to {0} steps")]
    StepCount(usize),

    #[error("a runbook has at most {0} rollback steps")]
    RollbackCount(usize),

    #[error("a step rolls back, but the runbook has no rollback steps")]
    NoRollback,

    #[error("rollback steps always go on after a failure, so on_failure does not apply")]
    RollbackOnFailure,

    #[error("{action} cannot be a step of a runbook")]
    NestedRunbook { action: ActionId },

    #[error("{{{0}}} names no param; declare it, or write {{{{ and }}}} for literal braces")]
    UndeclaredParam(ParamName),

    #[error("{} {number}: {source}", phase_noun(*.phase))]
    Step {
        phase: RunPhase,
        number: usize,
        source: Box<CoreError>,
    },

    #[error("retry attempts must be 1 to {most}, got {attempts}")]
    Attempts { attempts: u32, most: u32 },

    #[error("retry every_seconds must be at most {longest}, got {pause}")]
    Pause { pause: u64, longest: u64 },

    #[error("param {0}: a choice needs choices")]
    NoChoices(ParamName),

    #[error("param {0}: only a choice has choices")]
    StrayChoices(ParamName),

    #[error("param {0}: only an integer has min and max")]
    StrayBounds(ParamName),

    #[error("param {name}: min {min} is above max {max}")]
    UpsideDown { name: ParamName, min: i64, max: i64 },

    #[error("expected text, got {0}")]
    NotText(Value),

    #[error("text is longer than {0} bytes")]
    TextTooLong(usize),

    #[error("text cannot hold control characters such as new lines")]
    ControlInText,

    #[error("{word:?} is not a word: use 1 to {longest} letters, digits or {symbols}")]
    NotWord {
        word: String,
        longest: usize,
        symbols: &'static str,
    },

    #[error("expected a whole number, got {0}")]
    NotInteger(Value),

    #[error("expected true or false, got {0}")]
    NotBoolean(Value),

    #[error("{0:?} is not a target: name one host, such as @project/api-1 or deploy@10.0.1.12")]
    NotTarget(String),

    #[error("a path cannot be empty")]
    EmptyPathParam,

    #[error("{chosen:?} is not one of {}", .choices.join(", "))]
    NotAChoice {
        chosen: String,
        choices: Vec<String>,
    },

    #[error("{number} is outside {}..{}", bound(*.min), bound(*.max))]
    OutOfBounds {
        number: i64,
        min: Option<i64>,
        max: Option<i64>,
    },

    #[error("the runbook has no param {0}")]
    UnknownParam(String),

    #[error("param {0} is required")]
    MissingParam(ParamName),

    #[error("param {name}: {source}")]
    Param {
        name: ParamName,
        source: Box<CoreError>,
    },

    #[error("no value for param {0}")]
    NoValue(String),

    #[error("expected exit {0}, but the result has no exit")]
    NoExitCarried(i64),

    #[error("exit {exit}, expected {wanted}")]
    WrongExit { exit: i64, wanted: i64 },

    #[error("no exit code, expected {wanted}{}", signal_note(.signal.as_deref()))]
    NoExitCode { wanted: i64, signal: Option<String> },

    #[error("the output does not contain {0:?}")]
    OutputLacks(String),

    #[error("{pointer} is {found}, expected {wanted}")]
    WrongField {
        pointer: String,
        found: Value,
        wanted: Value,
    },

    #[error("the result has no {pointer}, expected {wanted}")]
    ResultLacks { pointer: String, wanted: Value },

    #[error("there is no action {0}")]
    NoAction(ActionId),

    #[error("{0} is for the person only, so a runbook cannot run it")]
    PersonOnly(ActionId),

    #[error("{0} needs a target")]
    NeedsTarget(ActionId),

    #[error("{0} takes no target")]
    TakesNoTarget(ActionId),

    #[error("{0} does not name a host this core can reach")]
    Unreachable(ResourceRef),

    #[error(transparent)]
    Refused(ActionError),

    #[error("runbook {name}: {source}")]
    Runbook {
        name: RunbookName,
        source: Box<CoreError>,
    },

    #[error("there is no such runbook")]
    NoSuchRunbook,

    #[error("the file is not UTF-8 text")]
    NotUtf8,

    #[error("{0}")]
    UnreadableRunbook(#[source] io::Error),

    #[error("{0}")]
    Malformed(String),

    #[error("cannot write {name}: {source}")]
    Unwritable {
        name: RunbookName,
        source: toml::ser::Error,
    },

    #[error("runbook {name} of project {project}: {source}")]
    InProject {
        name: RunbookName,
        project: String,
        source: Box<CoreError>,
    },

    #[error(
        "runbook {name} of project {project} is now {actual}, which is not what the call was \
         admitted for; run it again"
    )]
    Moved {
        name: RunbookName,
        project: String,
        actual: String,
    },

    #[error("the runbook cannot be written as TOML: {0}")]
    Unprintable(#[source] toml::ser::Error),

    #[error("cannot write runbook {name}: {source}")]
    NotSaved {
        name: RunbookName,
        source: io::Error,
    },

    #[error("the journal has no event {0}")]
    NoEvent(Seq),

    #[error("journal event {0} belongs to no call")]
    NoCall(Seq),

    #[error("journal event {0} is not a request")]
    NotARequest(Seq),

    #[error("{what} must list 1 to {most} {numbers}")]
    Picks {
        what: &'static str,
        most: usize,
        numbers: &'static str,
    },

    #[error("there is no open session {0}")]
    NoSessionNamed(String),

    #[error("session {0} is a local shell, so its commands have no host to run on")]
    LocalSession(String),

    #[error("the session has no finished block {0}")]
    NoBlock(u64),

    #[error("block {0} has no command line; its shell did not report one")]
    NoCommand(u64),

    #[error("runbooks are not connected to a running core")]
    NotConnected,

    #[error("runbooks are already connected to a core")]
    AlreadyConnected,

    #[error("this call has no connection to report progress on")]
    NoProgressConnection,

    #[error("the runbook was not planned before it ran")]
    NotPlanned,

    #[error("there is no running run {0}")]
    NotRunning(u64),

    #[error(transparent)]
    Database(#[from] DbError),

    #[error(transparent)]
    Action(#[from] ActionError),

    #[error("{0}")]
    DbTarget(Box<CoreError>),

    #[error("the target is no longer a database")]
    NoLongerDatabase,

    #[error("the database actions are shutting down")]
    ShuttingDown,

    #[error("no answer within {} s, so the call was stopped", .0.as_secs())]
    DbTimeout(Duration),

    #[error(
        "{target} is now reached through a root login, which the call was not admitted for; \
         run it again so it is asked as privileged"
    )]
    RootLogin { target: String },

    #[error(
        "{target} now leads to {reached}, which is not what the call was admitted for; \
         run it again so it is checked for the new place"
    )]
    DbMoved { target: String, reached: String },

    #[error("give either sql or command")]
    NoStatement,

    #[error("sql is empty")]
    EmptySql,

    #[error("command is empty")]
    EmptyCommand,

    #[error("a redis command carries its values in the command itself, not in params")]
    CommandParams,

    #[error("redis takes a command, such as [\"GET\", \"user:1\"], not sql")]
    SqlForRedis,

    #[error("this database takes sql, not a command")]
    CommandForSql,

    #[error("{field} must be between 1 and {most}")]
    OutOfRange { field: &'static str, most: u64 },

    #[error("redis has no query plans to explain")]
    NoPlans,

    #[error("db.explain takes one statement the parser understands, got {0}")]
    NotOneStatement(StatementKind),

    #[error("give the statement itself; db.explain adds EXPLAIN")]
    AlreadyExplained,

    #[error(transparent)]
    Kube(#[from] KubeError),

    #[error("{0} is not a cluster")]
    NotAClusterTarget(ResourceRef),

    #[error("{0}")]
    KubePlace(KubeError),

    #[error("the cluster can only be reached from inside the core")]
    OutsideCore,

    #[error("reading the kubeconfig stopped unexpectedly")]
    KubeconfigStopped,

    #[error(
        "{0} names secrets now, which was not known when the call was admitted; \
         run it again so it is asked as privileged"
    )]
    SecretsNow(String),

    #[error(
        "{0} is cluster-scoped, which was not known when the call was admitted; \
         run it again so it is asked as privileged"
    )]
    ClusterScopedNow(String),

    #[error("follow and stream go together: name the stream to follow the logs on")]
    FollowWithoutStream,

    #[error("no stream was started by request {0}")]
    NoStream(Seq),

    #[error("the stream of request {0} belongs to another caller")]
    StreamOfAnother(Seq),

    #[error("this call has no connection to stream over")]
    NoStreamConnection,

    #[error("there is no open port forward {0}")]
    NoForward(ForwardId),

    #[error(
        "{0} is now reached through a root login, which the call was not admitted for; \
         run it again so it is asked as privileged"
    )]
    RootLoginNow(ResourceRef),

    #[error(
        "{target} now leads to {destination}, which is not what the call was admitted for; \
         run it again so it is checked for the new place"
    )]
    DestinationChanged {
        target: ResourceRef,
        destination: String,
    },
}

impl CoreError {
    pub(crate) fn field(field: &'static str, reason: Self) -> Self {
        Self::Field {
            field,
            reason: Box::new(reason),
        }
    }

    pub(crate) fn in_step(self, phase: RunPhase, number: usize) -> Self {
        Self::Step {
            phase,
            number,
            source: Box::new(self),
        }
    }

    pub(crate) fn in_param(self, name: &ParamName) -> Self {
        Self::Param {
            name: name.clone(),
            source: Box::new(self),
        }
    }

    pub(crate) fn in_runbook(self, name: &RunbookName) -> Self {
        Self::Runbook {
            name: name.clone(),
            source: Box::new(self),
        }
    }

    pub(crate) fn in_project(self, name: &RunbookName, project: &str) -> Self {
        Self::InProject {
            name: name.clone(),
            project: project.to_owned(),
            source: Box::new(self),
        }
    }

    fn is_invalid_params(&self) -> bool {
        matches!(
            self,
            Self::NotOneLine(..)
                | Self::BadName(..)
                | Self::BadParamName(..)
                | Self::BadPointer(..)
                | Self::BadAction(..)
                | Self::BadReference(..)
                | Self::TooManyParams(..)
                | Self::DeclaredTwice(..)
                | Self::StepCount(..)
                | Self::RollbackCount(..)
                | Self::NoRollback
                | Self::RollbackOnFailure
                | Self::NestedRunbook { .. }
                | Self::UndeclaredParam(..)
                | Self::Step { .. }
                | Self::Attempts { .. }
                | Self::Pause { .. }
                | Self::NoChoices(..)
                | Self::StrayChoices(..)
                | Self::StrayBounds(..)
                | Self::UpsideDown { .. }
                | Self::NotText(..)
                | Self::TextTooLong(..)
                | Self::ControlInText
                | Self::NotWord { .. }
                | Self::NotInteger(..)
                | Self::NotBoolean(..)
                | Self::NotTarget(..)
                | Self::EmptyPathParam
                | Self::NotAChoice { .. }
                | Self::OutOfBounds { .. }
                | Self::UnknownParam(..)
                | Self::MissingParam(..)
                | Self::Param { .. }
                | Self::NoValue(..)
                | Self::NoExitCarried(..)
                | Self::WrongExit { .. }
                | Self::NoExitCode { .. }
                | Self::OutputLacks(..)
                | Self::WrongField { .. }
                | Self::ResultLacks { .. }
                | Self::NoAction(..)
                | Self::PersonOnly(..)
                | Self::NeedsTarget(..)
                | Self::TakesNoTarget(..)
                | Self::Unreachable(..)
                | Self::Refused(..)
                | Self::Runbook { .. }
                | Self::NoSuchRunbook
                | Self::NotUtf8
                | Self::UnreadableRunbook(..)
                | Self::Malformed(..)
                | Self::Unwritable { .. }
                | Self::InProject { .. }
                | Self::NoEvent(..)
                | Self::NoCall(..)
                | Self::NotARequest(..)
                | Self::Picks { .. }
                | Self::LocalSession(..)
                | Self::NoBlock(..)
                | Self::NoCommand(..)
                | Self::DbTarget(..)
                | Self::NoStatement
                | Self::EmptySql
                | Self::EmptyCommand
                | Self::CommandParams
                | Self::SqlForRedis
                | Self::CommandForSql
                | Self::OutOfRange { .. }
                | Self::NoPlans
                | Self::NotOneStatement(..)
                | Self::AlreadyExplained
                | Self::FollowWithoutStream
        )
    }

    fn ssh_reason(error: &SshError) -> String {
        match error {
            SshError::HostKeyUnknown { host, fingerprint } => format!(
                "the host key of {host} is not in known_hosts (fingerprint {fingerprint}); \
                 check the fingerprint and add the key to ~/.ssh/known_hosts before trying again"
            ),
            SshError::HostKeyChanged { host, fingerprint } => format!(
                "the host key of {host} does not match known_hosts (server sent {fingerprint}); \
                 someone may be intercepting the connection, so it was refused"
            ),
            SshError::HostKeyOtherType { host, fingerprint } => format!(
                "{host} presented a key of a type not pinned in known_hosts (fingerprint \
                 {fingerprint}), while a key of another type is pinned; do not replace the pinned \
                 key without checking the new one out of band"
            ),
            SshError::Timeout(limit) => format!(
                "the host did not answer within {} s, so nothing was started",
                limit.as_secs()
            ),
            SshError::CommandTimeout { .. }
            | SshError::Sftp { .. }
            | SshError::SftpLost { .. }
            | SshError::SftpClosed
            | SshError::Listen { .. } => error.to_string(),
            other => format!("ssh: {other}"),
        }
    }
}

impl From<PlatformError> for CoreError {
    fn from(error: PlatformError) -> Self {
        match error {
            PlatformError::VaultNameLength { .. } | PlatformError::InvalidVaultName { .. } => {
                Self::VaultName(error)
            }
            other => Self::Platform(other),
        }
    }
}

impl From<CoreError> for ActionError {
    fn from(error: CoreError) -> Self {
        if error.is_invalid_params() {
            return Self::InvalidParams(error.to_string());
        }

        match error {
            CoreError::Action(action) => action,
            CoreError::Database(DbError::InvalidInput(reason))
            | CoreError::Kube(KubeError::Invalid(reason)) => Self::InvalidParams(reason),
            CoreError::Database(DbError::Server {
                message,
                code: Some(code),
            }) => Self::Failed(format!("the server refused: {message} ({code})")),
            CoreError::Kube(error @ KubeError::UnknownKind(_)) => {
                Self::InvalidParams(error.to_string())
            }
            other => Self::Failed(other.to_string()),
        }
    }
}

impl From<CoreError> for JournalError {
    fn from(error: CoreError) -> Self {
        Self::key_store(error)
    }
}

impl From<CoreError> for SyncError {
    fn from(error: CoreError) -> Self {
        Self::Projects(Box::new(error))
    }
}

fn phase_noun(phase: RunPhase) -> &'static str {
    match phase {
        RunPhase::Steps => "step",
        RunPhase::Rollback => "rollback step",
    }
}

fn bound(limit: Option<i64>) -> String {
    limit.map(|limit| limit.to_string()).unwrap_or_default()
}

fn signal_note(signal: Option<&str>) -> String {
    signal
        .map(|signal| format!(" (ended by signal {signal})"))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use synt_actions::ActionError;
    use synt_db::DbError;

    use super::CoreError;

    fn action(error: CoreError) -> ActionError {
        error.into()
    }

    #[test]
    fn bad_input_to_the_database_is_a_parameter_error() {
        let error = DbError::InvalidInput("the command is empty".to_owned());

        assert_eq!(
            action(error.into()),
            ActionError::InvalidParams("the command is empty".to_owned())
        );
    }

    #[test]
    fn a_refusal_names_the_server_code() {
        let error = DbError::Server {
            message: "permission denied".to_owned(),
            code: Some("42501".to_owned()),
        };

        assert_eq!(
            action(error.into()),
            ActionError::Failed("the server refused: permission denied (42501)".to_owned())
        );
    }

    #[test]
    fn statement_mistakes_are_parameter_errors() {
        assert_eq!(
            action(CoreError::CommandForSql),
            ActionError::InvalidParams("this database takes sql, not a command".to_owned())
        );
    }

    #[test]
    fn a_failed_action_passes_through() {
        assert_eq!(
            action(ActionError::TargetRequired.into()),
            ActionError::TargetRequired
        );
    }
}
