use std::fs;

use synt_sync::{
    Base64, Cursor, DocumentId, FolderProvider, Generation, MemberId, MemberKey, ProjectId,
    PublicKey, RevisionId, SealedRevision, SyncError, SyncProvider, WrappedKey,
};
use synt_testkit::TestError;

fn project() -> Result<ProjectId, SyncError> {
    ProjectId::parse(&"4f".repeat(16))
}

fn sealed(at: u64, revision: &str) -> Result<SealedRevision, SyncError> {
    Ok(SealedRevision {
        project: project()?,
        document: DocumentId::parse(&"d0".repeat(16))?,
        revision: RevisionId::parse(&revision.repeat(16))?,
        parents: Vec::new(),
        generation: Generation::FIRST,
        at,
        sealed: Base64::new(vec![1, 2, 3]),
    })
}

fn member(byte: u8) -> MemberKey {
    MemberKey::new(PublicKey::new([byte; 32]))
}

#[test]
fn pushed_revisions_are_pulled_in_time_order() -> Result<(), TestError> {
    let folder = tempfile::tempdir().unwrap();
    let provider = FolderProvider::open(folder.path()).unwrap();
    let late = sealed(1_700_000_002_000, "b2")?;
    let early = sealed(1_700_000_001_000, "a1")?;

    provider
        .push(&project()?, &[late.clone(), early.clone()])
        .unwrap();
    let pulled = provider.pull(&project()?, &Cursor::default()).unwrap();

    assert_eq!(pulled.revisions, vec![early, late]);
    assert_ne!(pulled.cursor, Cursor::default());

    Ok(())
}

#[test]
fn a_cursor_skips_what_is_long_past_and_keeps_the_last_minute() -> Result<(), TestError> {
    let folder = tempfile::tempdir().unwrap();
    let provider = FolderProvider::open(folder.path()).unwrap();
    let old = sealed(1_700_000_000_000, "a1")?;
    let recent = sealed(1_700_000_100_000, "b2")?;
    provider.push(&project()?, &[old, recent.clone()]).unwrap();
    let cursor = provider
        .pull(&project()?, &Cursor::default())
        .unwrap()
        .cursor;

    let newer = sealed(1_700_000_100_500, "c3")?;
    provider
        .push(&project()?, std::slice::from_ref(&newer))
        .unwrap();
    let pulled = provider.pull(&project()?, &cursor).unwrap();

    assert_eq!(pulled.revisions, vec![recent, newer]);
    assert!(pulled.cursor.get() > cursor.get());

    Ok(())
}

#[test]
fn an_empty_project_pulls_nothing() -> Result<(), TestError> {
    let folder = tempfile::tempdir().unwrap();
    let provider = FolderProvider::open(folder.path()).unwrap();

    let pulled = provider.pull(&project()?, &Cursor::default()).unwrap();

    assert!(pulled.revisions.is_empty());
    assert_eq!(pulled.cursor, Cursor::default());

    Ok(())
}

#[test]
fn members_join_get_keys_and_leave() -> Result<(), TestError> {
    let folder = tempfile::tempdir().unwrap();
    let provider = FolderProvider::open(folder.path()).unwrap();
    let alice = member(1);
    let bob = member(2);
    let wrapped = WrappedKey {
        member: bob.member.clone(),
        by: alice.member.clone(),
        generation: Generation::FIRST,
        ephemeral: PublicKey::new([3; 32]),
        sealed: Base64::new(vec![4; 48]),
    };

    provider.join(&project()?, &alice).unwrap();
    provider.join(&project()?, &bob).unwrap();
    provider
        .share(&project()?, std::slice::from_ref(&wrapped))
        .unwrap();

    let mut joined = vec![alice.clone(), bob.clone()];
    joined.sort_by(|one, other| one.member.cmp(&other.member));
    assert_eq!(provider.members(&project()?).unwrap(), joined);
    assert_eq!(
        provider.key(&project()?, &bob.member).unwrap(),
        Some(wrapped)
    );
    assert_eq!(provider.key(&project()?, &alice.member).unwrap(), None);

    provider.leave(&project()?, &bob.member).unwrap();

    assert_eq!(provider.members(&project()?).unwrap(), vec![alice]);
    assert_eq!(provider.key(&project()?, &bob.member).unwrap(), None);

    Ok(())
}

#[test]
fn a_member_file_with_someone_elses_key_is_ignored() -> Result<(), TestError> {
    let folder = tempfile::tempdir().unwrap();
    let provider = FolderProvider::open(folder.path()).unwrap();
    let alice = member(1);
    let mallory = member(9);
    provider.join(&project()?, &alice).unwrap();

    let forged = MemberKey {
        member: alice.member.clone(),
        public_key: mallory.public_key,
    };
    let path = folder
        .path()
        .join(project()?.get())
        .join("members")
        .join(format!("{}.json", alice.member));
    fs::write(path, serde_json::to_vec(&forged).unwrap()).unwrap();

    assert!(provider.members(&project()?).unwrap().is_empty());

    Ok(())
}

#[test]
fn a_damaged_revision_file_is_reported() -> Result<(), TestError> {
    let folder = tempfile::tempdir().unwrap();
    let provider = FolderProvider::open(folder.path()).unwrap();
    provider
        .push(&project()?, &[sealed(1_700_000_000_000, "a1")?])
        .unwrap();

    let revisions = folder.path().join(project()?.get()).join("revisions");
    for entry in fs::read_dir(&revisions).unwrap() {
        fs::write(entry.unwrap().path(), b"{ not json").unwrap();
    }

    let error = provider.pull(&project()?, &Cursor::default()).unwrap_err();

    assert!(error.to_string().contains("damaged"), "{error}");

    Ok(())
}

#[test]
fn the_folder_must_exist() {
    let folder = tempfile::tempdir().unwrap();

    assert!(FolderProvider::open(&folder.path().join("missing")).is_err());
    assert!(MemberId::parse("not an id").is_err());
}
