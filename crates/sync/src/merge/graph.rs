use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use crate::document::{DocumentPath, Revision};
use crate::id::RevisionId;
use crate::report::ConflictInfo;

#[derive(Debug, Clone, Default)]
pub(crate) struct DocumentGraph {
    revisions: BTreeMap<RevisionId, Revision>,
}

impl DocumentGraph {
    pub(crate) fn new(revisions: impl IntoIterator<Item = Revision>) -> Self {
        let revisions = revisions
            .into_iter()
            .map(|revision| (revision.id.clone(), revision))
            .collect();

        Self { revisions }
    }

    pub(crate) fn get(&self, id: &RevisionId) -> Option<&Revision> {
        self.revisions.get(id)
    }

    pub(crate) fn revisions(&self) -> impl DoubleEndedIterator<Item = &Revision> {
        let mut revisions: Vec<&Revision> = self.revisions.values().collect();
        revisions.sort_by(|one, other| order(one, other));

        revisions.into_iter()
    }

    pub(crate) fn heads(&self) -> Vec<&Revision> {
        let replaced: BTreeSet<&RevisionId> = self
            .revisions
            .values()
            .flat_map(|revision| &revision.parents)
            .collect();

        let mut heads: Vec<&Revision> = self
            .revisions
            .values()
            .filter(|revision| !replaced.contains(&revision.id))
            .collect();
        heads.sort_by(|one, other| order(one, other));

        heads
    }

    pub(crate) fn merge_base(&self, heads: &[&Revision]) -> Option<&Revision> {
        let common = heads
            .iter()
            .map(|head| self.ancestors(&head.id))
            .reduce(|common, ancestors| common.intersection(&ancestors).cloned().collect())?;

        let below_others: BTreeSet<RevisionId> = common
            .iter()
            .flat_map(|id| {
                let mut ancestors = self.ancestors(id);
                ancestors.remove(id);
                ancestors
            })
            .collect();

        common
            .iter()
            .filter(|id| !below_others.contains(*id))
            .filter_map(|id| self.revisions.get(id))
            .max_by(|one, other| order(one, other))
    }

    pub(crate) fn has_conflict(&self) -> bool {
        match self.heads().as_slice() {
            [head] => !head.conflicts.is_empty(),
            heads => heads.len() > 1,
        }
    }

    pub(crate) fn conflict(&self, path: &DocumentPath) -> Option<ConflictInfo> {
        if !self.has_conflict() {
            return None;
        }

        let path = path.clone();
        let heads = self.heads();

        if let [merge] = heads.as_slice() {
            return Some(ConflictInfo {
                path,
                merged: Some(merge.id.clone()),
                candidates: merge.parents.clone(),
                fields: merge.conflicts.clone(),
            });
        }

        Some(ConflictInfo {
            path,
            merged: None,
            candidates: heads.iter().map(|head| head.id.clone()).collect(),
            fields: Vec::new(),
        })
    }

    fn ancestors(&self, id: &RevisionId) -> BTreeSet<RevisionId> {
        let mut seen = BTreeSet::new();
        let mut waiting = vec![id.clone()];

        while let Some(next) = waiting.pop() {
            let Some(revision) = self.revisions.get(&next) else {
                continue;
            };

            if seen.insert(next) {
                waiting.extend(revision.parents.iter().cloned());
            }
        }

        seen
    }
}

fn order(one: &Revision, other: &Revision) -> Ordering {
    (one.at, &one.id).cmp(&(other.at, &other.id))
}

#[cfg(test)]
mod tests {
    use super::DocumentGraph;
    use crate::document::{DocumentPath, Draft, Revision};
    use crate::id::ProjectId;
    use crate::seal::{Keyring, Naming};

    struct Builder {
        naming: Naming,
    }

    impl Builder {
        fn new() -> Self {
            let project = ProjectId::parse(&"4f".repeat(16)).unwrap();
            let naming = Keyring::generate().unwrap().naming(&project).unwrap();

            Self { naming }
        }

        fn revision(&self, body: &str, at: u64, parents: &[&Revision]) -> Revision {
            Revision::new(
                &self.naming,
                Draft {
                    path: DocumentPath::parse("synt.toml").unwrap(),
                    parents: parents.iter().map(|parent| parent.id.clone()).collect(),
                    at,
                    body: Some(body.as_bytes().to_vec()),
                    conflicts: Vec::new(),
                    author: None,
                },
            )
        }
    }

    fn ids(revisions: &[&Revision]) -> Vec<String> {
        revisions
            .iter()
            .map(|revision| revision.id.to_string())
            .collect()
    }

    #[test]
    fn a_straight_line_has_one_head() {
        let builder = Builder::new();
        let first = builder.revision("a = 1", 1, &[]);
        let second = builder.revision("a = 2", 2, &[&first]);

        let graph = DocumentGraph::new([first, second.clone()]);

        assert_eq!(ids(&graph.heads()), ids(&[&second]));
    }

    #[test]
    fn two_edits_of_the_same_revision_are_two_heads_with_it_as_base() {
        let builder = Builder::new();
        let first = builder.revision("a = 1", 1, &[]);
        let left = builder.revision("a = 2", 3, &[&first]);
        let right = builder.revision("a = 3", 2, &[&first]);

        let graph = DocumentGraph::new([first.clone(), left.clone(), right.clone()]);
        let heads = graph.heads();

        assert_eq!(ids(&heads), ids(&[&right, &left]));
        assert_eq!(
            graph.merge_base(&heads).map(|base| &base.id),
            Some(&first.id)
        );
    }

    #[test]
    fn the_base_is_the_nearest_common_ancestor() {
        let builder = Builder::new();
        let first = builder.revision("a = 1", 1, &[]);
        let shared = builder.revision("a = 2", 2, &[&first]);
        let left = builder.revision("a = 3", 3, &[&shared]);
        let right = builder.revision("a = 4", 4, &[&shared]);

        let graph = DocumentGraph::new([first, shared.clone(), left, right]);
        let heads = graph.heads();

        assert_eq!(
            graph.merge_base(&heads).map(|base| &base.id),
            Some(&shared.id)
        );
    }

    #[test]
    fn unrelated_roots_have_no_base() {
        let builder = Builder::new();
        let one = builder.revision("a = 1", 1, &[]);
        let other = builder.revision("a = 2", 2, &[]);

        let graph = DocumentGraph::new([one, other]);

        assert!(graph.merge_base(&graph.heads()).is_none());
    }

    #[test]
    fn two_heads_are_a_conflict_between_both_candidates() {
        let builder = Builder::new();
        let first = builder.revision("a = 1", 1, &[]);
        let left = builder.revision("a = 2", 2, &[&first]);
        let right = builder.revision("a = 3", 3, &[&first]);

        let graph = DocumentGraph::new([first, left.clone(), right.clone()]);
        let conflict = graph.conflict(&DocumentPath::parse("synt.toml").unwrap());

        assert!(graph.has_conflict());
        assert_eq!(
            conflict.map(|conflict| (conflict.merged, conflict.candidates)),
            Some((None, vec![left.id, right.id]))
        );
    }

    #[test]
    fn a_clean_line_has_no_conflict() {
        let builder = Builder::new();
        let first = builder.revision("a = 1", 1, &[]);
        let second = builder.revision("a = 2", 2, &[&first]);

        let graph = DocumentGraph::new([first, second]);

        assert!(!graph.has_conflict());
        assert!(
            graph
                .conflict(&DocumentPath::parse("synt.toml").unwrap())
                .is_none()
        );
    }
}
