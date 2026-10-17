use crate::sha256::Sha256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredRevision {
    revision: Sha256,
    parents: Vec<Sha256>,
    body: Vec<u8>,
}

impl StoredRevision {
    pub(crate) fn new(body: &[u8], parents: Vec<Sha256>) -> Self {
        Self {
            revision: Sha256::of(body),
            parents,
            body: body.to_vec(),
        }
    }

    pub(super) fn recorded(revision: Sha256, joined_parents: &str, body: Vec<u8>) -> Self {
        let parents = joined_parents
            .split(',')
            .filter(|parent| !parent.is_empty())
            .map(Sha256::recorded)
            .collect();

        Self {
            revision,
            parents,
            body,
        }
    }

    pub(crate) fn revision(&self) -> &Sha256 {
        &self.revision
    }

    #[cfg(test)]
    pub(crate) fn parents(&self) -> &[Sha256] {
        &self.parents
    }

    pub(crate) fn body(&self) -> &[u8] {
        &self.body
    }

    pub(crate) fn into_body(self) -> Vec<u8> {
        self.body
    }

    pub(super) fn joined_parents(&self) -> String {
        let parents: Vec<&str> = self.parents.iter().map(Sha256::as_str).collect();

        parents.join(",")
    }
}
