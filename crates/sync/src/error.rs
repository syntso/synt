use std::error::Error as StdError;
use std::io;
use std::path::PathBuf;

use synt_actions::ActionError;
use synt_journal::JournalError;
use synt_platform::PlatformError;
use synt_proto::ProtoError;
use thiserror::Error;

use crate::document::DocumentPath;
use crate::id::{MemberId, ProjectId, RevisionId};
use crate::provider::Place;

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("{text:?} is not a {what}: it needs 32 lowercase hex characters")]
    BadId { what: &'static str, text: String },

    #[error(
        "{0:?} is not a document path: use synt.toml, resources/<name>.toml, \
         runbooks/<name>.toml or extensions/<extension>/<path>.<suffix>"
    )]
    BadPath(String),

    #[error("project {project} already syncs through {place}; run sync.disable first")]
    AlreadySyncs { project: ProjectId, place: Place },

    #[error("{member} has not joined project {project}; run sync.enable on that device first")]
    NotJoined {
        member: MemberId,
        project: ProjectId,
    },

    #[error("a device cannot remove itself; use sync.disable on it")]
    RemovesItself,

    #[error("revision {revision} is not in the history of {path}")]
    NotInHistory {
        revision: RevisionId,
        path: DocumentPath,
    },

    #[error("{0} has no conflict to resolve")]
    NoConflict(DocumentPath),

    #[error("the resolution vanished from the history")]
    ResolutionLost,

    #[error(transparent)]
    Projects(Box<dyn StdError + Send + Sync>),

    #[error("project {0} does not sync; run sync.enable first")]
    NotEnabled(ProjectId),

    #[error("this device does not hold the key of project {0} yet; a member has to share it")]
    NoKey(ProjectId),

    #[error("this device was removed from project {0}")]
    Removed(ProjectId),

    #[error("{0} is not a plain folder any more; the round is skipped until the project is back")]
    FolderGone(PathBuf),

    #[error(
        "{folder} does not hold the project file of project {project} any more; \
         the round is skipped until it is back"
    )]
    ProjectFileGone { folder: PathBuf, project: ProjectId },

    #[error(
        "every one of the {0} documents of the project is missing from its folder; \
         the round is skipped so the other members keep them. Put the files back, remove them \
         a few at a time, or run sync.disable to stop syncing this project"
    )]
    EverythingMissing(usize),

    #[error("{0} must be a plain folder, not a link or a file")]
    NotPlainFolder(PathBuf),

    #[error("the sync folder {0} does not exist")]
    NoSyncFolder(PathBuf),

    #[error("the sync provider failed: {path}: {source}")]
    Provider { path: PathBuf, source: io::Error },

    #[error("the sync provider failed: {path} is damaged: {source}")]
    Damaged {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("a revision was refused: it belongs to another project")]
    OtherProject,

    #[error("a revision was refused: its payload is malformed: {0}")]
    MalformedPayload(serde_json::Error),

    #[error("a revision was refused: revision {0} does not match its content")]
    Mismatch(RevisionId),

    #[error("a revision was refused: {path} is not a document of kind {kind}")]
    WrongKind { path: DocumentPath, kind: String },

    #[error("a revision was refused: {path} is larger than {limit} bytes")]
    TooLarge { path: DocumentPath, limit: usize },

    #[error("a revision was refused: {0} lists too many conflicts")]
    TooManyConflicts(DocumentPath),

    #[error("the project key was shared by {0}, who is not a member")]
    SharedByStranger(MemberId),

    #[error("the new project key drops keys this device already holds")]
    KeysDropped,

    #[error("member {0} does not match its public key")]
    ForgedMember(MemberId),

    #[error("the shared key names other members than the ones it came from and to")]
    Misaddressed,

    #[error("the shared key is not of the generation it claims")]
    WrongGeneration,

    #[error("cannot encode: {0}")]
    Encode(#[from] serde_json::Error),

    #[error("cryptography failed: {0}")]
    Crypto(&'static str),

    #[error("the system random source failed: {0}")]
    Random(#[from] getrandom::Error),

    #[error("the key vault failed: {0}")]
    Vault(#[from] PlatformError),

    #[error("the sync database failed: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("the journal failed: {0}")]
    Journal(#[from] JournalError),

    #[error("a journal action is misnamed: {0}")]
    NoteAction(#[from] ProtoError),

    #[error("{0}")]
    Io(#[from] io::Error),

    #[error("the sync work stopped unexpectedly")]
    Stopped,
}

impl SyncError {
    fn is_bad_request(&self) -> bool {
        matches!(
            self,
            Self::BadId { .. }
                | Self::BadPath(_)
                | Self::AlreadySyncs { .. }
                | Self::NotJoined { .. }
                | Self::RemovesItself
                | Self::NotInHistory { .. }
                | Self::NoConflict(_)
        )
    }
}

impl From<SyncError> for ActionError {
    fn from(error: SyncError) -> Self {
        if error.is_bad_request() {
            return Self::InvalidParams(error.to_string());
        }

        Self::failed(error.to_string())
    }
}
