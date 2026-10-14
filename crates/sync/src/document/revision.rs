use super::{DocumentPath, Draft, Payload};
use crate::SyncError;
use crate::canonical::Canonical;
use crate::encoding::Base64;
use crate::id::{DocumentId, Generation, MemberId, ProjectId, RevisionId};
use crate::provider::SealedRevision;
use crate::seal::{Keyring, Naming, cipher};

pub(crate) const LARGEST_BODY: usize = 1024 * 1024;
const LONGEST_CONFLICT_LIST: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Revision {
    pub(crate) document: DocumentId,
    pub(crate) id: RevisionId,
    pub(crate) parents: Vec<RevisionId>,
    pub(crate) at: u64,
    pub(crate) path: DocumentPath,
    pub(crate) body: Option<Vec<u8>>,
    pub(crate) conflicts: Vec<String>,
    pub(crate) author: Option<MemberId>,
}

impl Revision {
    pub(crate) fn new(naming: &Naming, mut draft: Draft) -> Self {
        draft.parents.sort();
        draft.parents.dedup();

        let document = naming.document_id(&draft.path);
        let id = naming.revision_id(&content(&document, &draft));

        Self {
            document,
            id,
            parents: draft.parents,
            at: draft.at,
            path: draft.path,
            body: draft.body,
            conflicts: draft.conflicts,
            author: draft.author,
        }
    }

    pub(crate) fn open(
        project: &ProjectId,
        keyring: &Keyring,
        sealed: &SealedRevision,
    ) -> Result<Self, SyncError> {
        if &sealed.project != project {
            return Err(SyncError::refused("it belongs to another project"));
        }

        let key = keyring.key(sealed.generation)?;
        let plain = cipher::open(key, &aad(sealed), sealed.sealed.get())?;
        let payload: Payload = serde_json::from_slice(&plain)
            .map_err(|error| SyncError::refused(format!("its payload is malformed: {error}")))?;

        let revision = Self::new(&keyring.naming(project)?, draft(sealed, payload)?);

        if revision.document != sealed.document || revision.id != sealed.revision {
            return Err(SyncError::refused(format!(
                "revision {} does not match its content",
                sealed.revision
            )));
        }

        Ok(revision)
    }

    pub(crate) fn seal(
        &self,
        project: &ProjectId,
        keyring: &Keyring,
    ) -> Result<SealedRevision, SyncError> {
        let (generation, key) = keyring.current()?;
        let mut sealed = self.unsealed(project, generation);

        let payload = serde_json::to_vec(&self.payload())
            .map_err(|error| SyncError::invalid(format!("cannot encode a revision: {error}")))?;
        sealed.sealed = Base64::new(cipher::seal(key, &aad(&sealed), &payload)?);

        Ok(sealed)
    }

    pub(crate) fn is_removal(&self) -> bool {
        self.body.is_none()
    }

    fn unsealed(&self, project: &ProjectId, generation: Generation) -> SealedRevision {
        SealedRevision {
            project: project.clone(),
            document: self.document.clone(),
            revision: self.id.clone(),
            parents: self.parents.clone(),
            generation,
            at: self.at,
            sealed: Base64::default(),
        }
    }

    fn payload(&self) -> Payload {
        Payload {
            path: self.path.get().to_owned(),
            kind: self.path.kind().to_string(),
            body: self.body.clone().map(Base64::new),
            conflicts: self.conflicts.clone(),
            author: self.author.clone(),
        }
    }
}

fn draft(sealed: &SealedRevision, payload: Payload) -> Result<Draft, SyncError> {
    let path = DocumentPath::parse(&payload.path)?;

    if path.kind().to_string() != payload.kind {
        return Err(SyncError::refused(format!(
            "{path} is not a document of kind {}",
            payload.kind
        )));
    }

    let body = payload.body.map(Base64::into_bytes);
    if body.as_ref().is_some_and(|body| body.len() > LARGEST_BODY) {
        return Err(SyncError::refused(format!(
            "{path} is larger than {LARGEST_BODY} bytes"
        )));
    }

    if payload.conflicts.len() > LONGEST_CONFLICT_LIST {
        return Err(SyncError::refused(format!(
            "{path} lists too many conflicts"
        )));
    }

    Ok(Draft {
        path,
        parents: sealed.parents.clone(),
        at: sealed.at,
        body,
        conflicts: payload.conflicts,
        author: payload.author,
    })
}

fn content(document: &DocumentId, draft: &Draft) -> Vec<u8> {
    Canonical::new("synt.sync.revision")
        .text(document.get())
        .texts(draft.parents.iter().map(RevisionId::get))
        .number(draft.at)
        .text(draft.path.get())
        .text(&draft.path.kind().to_string())
        .optional(draft.body.as_deref())
        .texts(draft.conflicts.iter().map(String::as_str))
        .optional(draft.author.as_ref().map(|author| author.get().as_bytes()))
        .finish()
}

fn aad(sealed: &SealedRevision) -> Vec<u8> {
    Canonical::new("synt.sync.sealed")
        .text(sealed.project.get())
        .text(sealed.document.get())
        .text(sealed.revision.get())
        .texts(sealed.parents.iter().map(RevisionId::get))
        .number(u64::from(sealed.generation.get()))
        .number(sealed.at)
        .finish()
}

#[cfg(test)]
mod tests {
    use super::Revision;
    use crate::document::{DocumentPath, Draft};
    use crate::encoding::Base64;
    use crate::id::{Generation, ProjectId};
    use crate::seal::Keyring;

    fn project() -> ProjectId {
        ProjectId::parse(&"4f".repeat(16)).unwrap()
    }

    fn draft(body: &str) -> Draft {
        Draft {
            path: DocumentPath::parse("resources/api-1.toml").unwrap(),
            parents: Vec::new(),
            at: 1_700_000_000_000,
            body: Some(body.as_bytes().to_vec()),
            conflicts: Vec::new(),
            author: None,
        }
    }

    fn revision(keyring: &Keyring, body: &str) -> Revision {
        Revision::new(&keyring.naming(&project()).unwrap(), draft(body))
    }

    #[test]
    fn a_sealed_revision_opens_to_the_same_revision() {
        let keyring = Keyring::generate().unwrap();
        let revision = revision(&keyring, "address = \"10.0.1.12\"\n");

        let sealed = revision.seal(&project(), &keyring).unwrap();

        assert_eq!(
            Revision::open(&project(), &keyring, &sealed).unwrap(),
            revision
        );
    }

    #[test]
    fn the_sealed_form_shows_neither_path_nor_body() {
        let keyring = Keyring::generate().unwrap();
        let revision = revision(&keyring, "address = \"10.0.1.12\"\n");

        let sealed = revision.seal(&project(), &keyring).unwrap();
        let text = serde_json::to_string(&sealed).unwrap();

        assert!(!text.contains("api-1"));
        assert!(!text.contains("resources"));
        assert!(!text.contains("10.0.1.12"));
    }

    #[test]
    fn the_same_content_gets_the_same_id_and_different_content_another() {
        let keyring = Keyring::generate().unwrap();

        assert_eq!(
            revision(&keyring, "a = 1").id,
            revision(&keyring, "a = 1").id
        );
        assert_ne!(
            revision(&keyring, "a = 1").id,
            revision(&keyring, "a = 2").id
        );
    }

    #[test]
    fn a_revision_moved_to_other_metadata_is_refused() {
        let keyring = Keyring::generate().unwrap();
        let sealed = revision(&keyring, "a = 1")
            .seal(&project(), &keyring)
            .unwrap();
        let other = revision(&keyring, "a = 2");

        let mut moved = sealed.clone();
        moved.revision = other.id.clone();
        let mut shifted = sealed.clone();
        shifted.at += 1;
        let mut reparented = sealed.clone();
        reparented.parents = vec![other.id];

        for tampered in [moved, shifted, reparented] {
            assert!(Revision::open(&project(), &keyring, &tampered).is_err());
        }
    }

    #[test]
    fn a_revision_of_another_project_or_with_a_forged_body_is_refused() {
        let keyring = Keyring::generate().unwrap();
        let sealed = revision(&keyring, "a = 1")
            .seal(&project(), &keyring)
            .unwrap();
        let other = ProjectId::parse(&"7a".repeat(16)).unwrap();

        let mut forged = sealed.clone();
        forged.sealed = Base64::new(vec![0; 64]);

        assert!(Revision::open(&other, &keyring, &sealed).is_err());
        assert!(Revision::open(&project(), &keyring, &forged).is_err());
    }

    #[test]
    fn a_revision_sealed_with_a_generation_this_device_lacks_does_not_open() {
        let keyring = Keyring::generate().unwrap();
        let rotated = keyring.rotated().unwrap();
        let sealed = revision(&rotated, "a = 1")
            .seal(&project(), &rotated)
            .unwrap();

        assert_eq!(sealed.generation, Generation::new(2));
        assert!(Revision::open(&project(), &keyring, &sealed).is_err());
        assert!(Revision::open(&project(), &rotated, &sealed).is_ok());
    }
}
