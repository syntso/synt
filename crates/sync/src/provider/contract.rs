use super::{Cursor, MemberKey, SealedRevision, WrappedKey};
use crate::SyncError;
use crate::id::{MemberId, ProjectId};

pub trait SyncProvider: Send + Sync {
    fn push(&self, project: &ProjectId, revisions: &[SealedRevision]) -> Result<(), SyncError>;

    fn pull(
        &self,
        project: &ProjectId,
        since: &Cursor,
    ) -> Result<(Vec<SealedRevision>, Cursor), SyncError>;

    fn members(&self, project: &ProjectId) -> Result<Vec<MemberKey>, SyncError>;

    fn join(&self, project: &ProjectId, member: &MemberKey) -> Result<(), SyncError>;

    fn leave(&self, project: &ProjectId, member: &MemberId) -> Result<(), SyncError>;

    fn share(&self, project: &ProjectId, wrapped: &[WrappedKey]) -> Result<(), SyncError>;

    fn key(&self, project: &ProjectId, member: &MemberId) -> Result<Option<WrappedKey>, SyncError>;
}
