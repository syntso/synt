use crate::id::RevisionId;

const AT_DIGITS: usize = 13;
const SUFFIX: &str = ".json";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct RevisionFile {
    name: String,
    at: u64,
}

impl RevisionFile {
    pub(super) fn new(at: u64, revision: &RevisionId) -> Self {
        Self {
            name: format!("{at:0AT_DIGITS$}-{revision}{SUFFIX}"),
            at,
        }
    }

    pub(super) fn parse(name: &str) -> Option<Self> {
        let stem = name.strip_suffix(SUFFIX)?;
        let (at, revision) = stem.split_once('-')?;

        if at.len() != AT_DIGITS || !at.chars().all(|character| character.is_ascii_digit()) {
            return None;
        }
        RevisionId::parse(revision).ok()?;

        Some(Self {
            name: name.to_owned(),
            at: at.parse().ok()?,
        })
    }

    pub(super) fn floor(at: u64) -> String {
        format!("{at:0AT_DIGITS$}")
    }

    pub(super) fn name(&self) -> &str {
        &self.name
    }

    pub(super) fn at(&self) -> u64 {
        self.at
    }
}

#[cfg(test)]
mod tests {
    use super::RevisionFile;
    use crate::id::RevisionId;

    #[test]
    fn names_sort_by_time() {
        let revision = RevisionId::parse(&"ab".repeat(16)).unwrap();

        let early = RevisionFile::new(999, &revision);
        let late = RevisionFile::new(1_700_000_000_000, &revision);

        assert!(early.name() < late.name());
        assert_eq!(RevisionFile::parse(late.name()), Some(late));
    }

    #[test]
    fn other_files_are_not_revisions() {
        let names = [
            "0000000000999-x.json",
            "999-abababababababababababababababab.json",
            ".0001700000000000-abababababababababababababababab.json.pending",
            "0001700000000000-abababababababababababababababab.txt",
        ];

        for name in names {
            assert!(RevisionFile::parse(name).is_none(), "{name}");
        }
    }
}
