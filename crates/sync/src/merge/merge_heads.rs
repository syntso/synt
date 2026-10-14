use super::toml_merge::merge_toml;
use super::{DocumentMerger, Merged};
use crate::document::{DocumentKind, Revision};

pub(crate) const REMOVED_AND_CHANGED: &str = "removed on one device and changed on another";

pub(crate) fn merge_heads(
    kind: &DocumentKind,
    base: Option<&[u8]>,
    heads: &[&Revision],
    merger: Option<&dyn DocumentMerger>,
) -> Option<Merged> {
    let (first, rest) = heads.split_first()?;
    let mut result = Merged::clean(first.body.as_deref());

    for newer in rest {
        let step = merge_two(
            kind,
            base,
            result.body.as_deref(),
            newer.body.as_deref(),
            merger,
        )?;

        result.body = step.body;
        result.conflicts.extend(step.conflicts);
    }

    result.conflicts.sort();
    result.conflicts.dedup();

    Some(result)
}

fn merge_two(
    kind: &DocumentKind,
    base: Option<&[u8]>,
    older: Option<&[u8]>,
    newer: Option<&[u8]>,
    merger: Option<&dyn DocumentMerger>,
) -> Option<Merged> {
    if older == newer || base == newer {
        return Some(Merged::clean(older));
    }

    if base == older {
        return Some(Merged::clean(newer));
    }

    match (older, newer) {
        (Some(older), Some(newer)) if kind.is_core() => Some(merge_toml(base, older, newer)),
        (Some(older), Some(newer)) => merger?
            .merge(kind, base, older, newer)
            .map(|body| Merged::clean(Some(&body))),
        (kept, None) | (None, kept) => Some(Merged::conflicted(kept, REMOVED_AND_CHANGED)),
    }
}

#[cfg(test)]
mod tests {
    use super::{REMOVED_AND_CHANGED, merge_heads};
    use crate::document::{DocumentKind, DocumentPath, Draft, Revision};
    use crate::id::ProjectId;
    use crate::merge::DocumentMerger;
    use crate::seal::{Keyring, Naming};

    fn naming() -> Naming {
        let project = ProjectId::parse(&"4f".repeat(16)).unwrap();

        Keyring::generate().unwrap().naming(&project).unwrap()
    }

    fn revision(naming: &Naming, path: &str, body: Option<&str>, at: u64) -> Revision {
        Revision::new(
            naming,
            Draft {
                path: DocumentPath::parse(path).unwrap(),
                parents: Vec::new(),
                at,
                body: body.map(|body| body.as_bytes().to_vec()),
                conflicts: Vec::new(),
                author: None,
            },
        )
    }

    fn uml() -> DocumentKind {
        DocumentKind::Extension {
            extension: "uml".to_owned(),
            kind: "uml".to_owned(),
        }
    }

    struct Concatenate;

    impl DocumentMerger for Concatenate {
        fn merge(
            &self,
            _kind: &DocumentKind,
            _base: Option<&[u8]>,
            older: &[u8],
            newer: &[u8],
        ) -> Option<Vec<u8>> {
            Some([older, b"+", newer].concat())
        }
    }

    #[test]
    fn a_removal_against_an_edit_keeps_the_edit_and_says_so() {
        let naming = naming();
        let edited = revision(&naming, "resources/a.toml", Some("a = 2"), 1);
        let removed = revision(&naming, "resources/a.toml", None, 2);

        let merged = merge_heads(
            &DocumentKind::Resource,
            Some(b"a = 1"),
            &[&edited, &removed],
            None,
        )
        .unwrap();

        assert_eq!(merged.body.as_deref(), Some(&b"a = 2"[..]));
        assert_eq!(merged.conflicts, vec![REMOVED_AND_CHANGED]);
    }

    #[test]
    fn a_removal_against_an_untouched_side_removes() {
        let naming = naming();
        let untouched = revision(&naming, "resources/a.toml", Some("a = 1"), 1);
        let removed = revision(&naming, "resources/a.toml", None, 2);

        let merged = merge_heads(
            &DocumentKind::Resource,
            Some(b"a = 1"),
            &[&untouched, &removed],
            None,
        )
        .unwrap();

        assert_eq!(merged.body, None);
        assert!(merged.conflicts.is_empty());
    }

    #[test]
    fn three_heads_fold_from_oldest_to_newest() {
        let naming = naming();
        let heads = [
            revision(&naming, "resources/a.toml", Some("a = 2\nb = 1\nc = 1"), 1),
            revision(&naming, "resources/a.toml", Some("a = 1\nb = 2\nc = 1"), 2),
            revision(&naming, "resources/a.toml", Some("a = 1\nb = 1\nc = 2"), 3),
        ];
        let heads: Vec<&Revision> = heads.iter().collect();

        let merged = merge_heads(
            &DocumentKind::Resource,
            Some(b"a = 1\nb = 1\nc = 1"),
            &heads,
            None,
        )
        .unwrap();
        let table: toml::Table = std::str::from_utf8(&merged.body.unwrap())
            .unwrap()
            .parse()
            .unwrap();

        assert_eq!(table, "a = 2\nb = 2\nc = 2".parse::<toml::Table>().unwrap());
        assert!(merged.conflicts.is_empty());
    }

    #[test]
    fn an_extension_document_goes_to_its_merger() {
        let naming = naming();
        let older = revision(&naming, "documents/uml/a.uml", Some("left"), 1);
        let newer = revision(&naming, "documents/uml/a.uml", Some("right"), 2);

        let merged =
            merge_heads(&uml(), Some(b"base"), &[&older, &newer], Some(&Concatenate)).unwrap();

        assert_eq!(merged.body.as_deref(), Some(&b"left+right"[..]));
    }

    #[test]
    fn an_extension_document_without_a_merger_keeps_both_heads() {
        let naming = naming();
        let older = revision(&naming, "documents/uml/a.uml", Some("left"), 1);
        let newer = revision(&naming, "documents/uml/a.uml", Some("right"), 2);

        assert!(merge_heads(&uml(), Some(b"base"), &[&older, &newer], None).is_none());
    }

    #[test]
    fn an_extension_document_changed_on_one_side_needs_no_merger() {
        let naming = naming();
        let older = revision(&naming, "documents/uml/a.uml", Some("base"), 1);
        let newer = revision(&naming, "documents/uml/a.uml", Some("right"), 2);

        let merged = merge_heads(&uml(), Some(b"base"), &[&older, &newer], None).unwrap();

        assert_eq!(merged.body.as_deref(), Some(&b"right"[..]));
    }
}
