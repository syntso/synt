use hkdf::Hkdf;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use zeroize::Zeroizing;

use super::SecretKey;
use super::secret_key::SECRET_KEY_BYTES;
use crate::canonical::Canonical;
use crate::document::DocumentPath;
use crate::id::{DocumentId, ProjectId, RevisionId};

const SALT: &[u8] = b"synt.sync.naming.v1";

#[derive(Clone)]
pub(crate) struct Naming {
    key: Zeroizing<[u8; SECRET_KEY_BYTES]>,
}

impl Naming {
    pub(crate) fn derive(first: &SecretKey, project: &ProjectId) -> Self {
        let mut key = Zeroizing::new([0_u8; SECRET_KEY_BYTES]);

        Hkdf::<Sha256>::new(Some(SALT), first.get())
            .expand(project.get().as_bytes(), key.as_mut())
            .expect("32 bytes is a valid HKDF-SHA256 output length");

        Self { key }
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
        let mut mac = Hmac::<Sha256>::new_from_slice(self.key.as_ref())
            .expect("HMAC accepts keys of any length");

        mac.update(input);
        mac.finalize().into_bytes().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::Naming;
    use crate::document::DocumentPath;
    use crate::id::ProjectId;
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

        let one = Naming::derive(&key, &project()).document_id(&path("resources/api-1.toml"));
        let other = Naming::derive(&key, &project()).document_id(&path("resources/api-1.toml"));

        assert_eq!(one, other);
    }

    #[test]
    fn ids_do_not_show_the_path_and_depend_on_the_key() {
        let key = SecretKey::generate().unwrap();
        let naming = Naming::derive(&key, &project());

        let api = naming.document_id(&path("resources/api-1.toml"));
        let db = naming.document_id(&path("resources/db.toml"));
        let elsewhere = Naming::derive(&SecretKey::generate().unwrap(), &project())
            .document_id(&path("resources/api-1.toml"));

        assert_ne!(api, db);
        assert_ne!(api, elsewhere);
        assert!(!api.get().contains("api"));
    }
}
