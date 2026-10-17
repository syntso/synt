mod cursor;
mod folder;
mod member_key;
mod place;
mod pulled;
mod sealed_revision;
mod wrapped_key;

pub use cursor::Cursor;
pub use folder::FolderProvider;
pub use member_key::MemberKey;
pub use place::Place;
pub use pulled::Pulled;
pub use sealed_revision::SealedRevision;
pub use wrapped_key::WrappedKey;

use crate::SyncError;
use crate::id::{MemberId, ProjectId};

pub trait SyncProvider: Send + Sync {
    fn push(&self, project: &ProjectId, revisions: &[SealedRevision]) -> Result<(), SyncError>;

    fn pull(&self, project: &ProjectId, since: &Cursor) -> Result<Pulled, SyncError>;

    fn members(&self, project: &ProjectId) -> Result<Vec<MemberKey>, SyncError>;

    fn join(&self, project: &ProjectId, member: &MemberKey) -> Result<(), SyncError>;

    fn leave(&self, project: &ProjectId, member: &MemberId) -> Result<(), SyncError>;

    fn share(&self, project: &ProjectId, wrapped: &[WrappedKey]) -> Result<(), SyncError>;

    fn key(&self, project: &ProjectId, member: &MemberId) -> Result<Option<WrappedKey>, SyncError>;
}
