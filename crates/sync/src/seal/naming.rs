use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

use super::SecretKey;
use crate::SyncError;
use crate::canonical::Canonical;
use crate::document::DocumentPath;
use crate::id::{DocumentId, ProjectId, RevisionId};

const SALT: &[u8] = b"synt.sync.naming.v1";

#[derive(Clone)]
pub(crate) struct Naming {
    mac: Hmac<Sha256>,
}

impl Naming {
    pub(crate) fn derive(first: &SecretKey, project: &ProjectId) -> Result<Self, SyncError> {
        let key = SecretKey::derive(SALT, first.get(), project.get().as_bytes())?;
        let mac = Hmac::<Sha256>::new_from_slice(key.get())
            .map_err(|_| SyncError::Crypto("cannot key the naming"))?;

        Ok(Self { mac })
    }

    pub(crate) fn document_id(&self, path: &DocumentPath) -> DocumentId {
        let input = Canonical::new("synt.sync.document")
            .text(path.get())
            .finish();

        DocumentId::from_digest(&self.digest(&input))
    }

    pub(crate) fn revision_id(&self, content: &[u8]) -> RevisionId {
        RevisionId::from_digest(&self.digest(content))
    }

    fn digest(&self, input: &[u8]) -> Vec<u8> {
        let mut mac = self.mac.clone();

        mac.update(input);
        mac.finalize().into_bytes().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;

    use super::{Naming, SALT};
    use crate::canonical::Canonical;
    use crate::document::DocumentPath;
    use crate::id::{DocumentId, ProjectId};
    use crate::seal::SecretKey;

    fn project() -> ProjectId {
        ProjectId::parse(&"4f".repeat(16)).unwrap()
    }

    fn path(text: &str) -> DocumentPath {
        DocumentPath::parse(text).unwrap()
    }

    #[test]
    fn the_same_path_gets_the_same_id_on_every_device() {
        let key = SecretKey::generate().unwrap();

        let one = Naming::derive(&key, &project())
            .unwrap()
            .document_id(&path("resources/api-1.toml"));
        let other = Naming::derive(&key, &project())
            .unwrap()
            .document_id(&path("resources/api-1.toml"));

        assert_eq!(one, other);
    }

    #[test]
    fn ids_do_not_show_the_path_and_depend_on_the_key() {
        let key = SecretKey::generate().unwrap();
        let naming = Naming::derive(&key, &project()).unwrap();

        let api = naming.document_id(&path("resources/api-1.toml"));
        let db = naming.document_id(&path("resources/db.toml"));
        let elsewhere = Naming::derive(&SecretKey::generate().unwrap(), &project())
            .unwrap()
            .document_id(&path("resources/api-1.toml"));

        assert_ne!(api, db);
        assert_ne!(api, elsewhere);
        assert!(!api.get().contains("api"));
    }

    #[test]
    fn a_document_id_is_the_hmac_of_the_path_under_the_derived_key() {
        let first = SecretKey::generate().unwrap();
        let naming = Naming::derive(&first, &project()).unwrap();

        let key = SecretKey::derive(SALT, first.get(), project().get().as_bytes()).unwrap();
        let mut mac = Hmac::<Sha256>::new_from_slice(key.get()).unwrap();
        mac.update(
            &Canonical::new("synt.sync.document")
                .text("synt.toml")
                .finish(),
        );
        let expected = DocumentId::from_digest(&mac.finalize().into_bytes());

        assert_eq!(naming.document_id(&path("synt.toml")), expected);
    }
}
