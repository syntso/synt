use std::fmt;

use super::DocumentKind;
use crate::SyncError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExtensionDocumentPath {
    relative: String,
}

impl ExtensionDocumentPath {
    pub const MOST_PARTS: usize = 8;
    pub const LONGEST: usize = 200;

    pub fn as_str(&self) -> &str {
        &self.relative
    }

    pub fn suffix(&self) -> &str {
        self.relative
            .rsplit_once('.')
            .map_or("", |(_, suffix)| suffix)
    }

    pub fn segments(&self) -> impl Iterator<Item = &str> {
        self.relative.split('/')
    }

    fn has_suffix(segments: &[&str]) -> bool {
        let Some((name, suffix)) = segments.last().and_then(|file| file.rsplit_once('.')) else {
            return false;
        };

        DocumentKind::is_name(name) && DocumentKind::is_suffix(suffix)
    }
}

impl TryFrom<String> for ExtensionDocumentPath {
    type Error = SyncError;

    fn try_from(relative: String) -> Result<Self, SyncError> {
        let segments: Vec<&str> = relative.split('/').collect();

        if relative.len() > Self::LONGEST || segments.len() > Self::MOST_PARTS {
            return Err(SyncError::LongExtensionPath {
                most_parts: Self::MOST_PARTS,
                longest: Self::LONGEST,
            });
        }

        if !segments
            .iter()
            .all(|segment| DocumentKind::is_name(segment))
        {
            return Err(SyncError::BadExtensionPath(relative));
        }

        if !Self::has_suffix(&segments) {
            return Err(SyncError::NoSuffix(relative));
        }

        Ok(Self { relative })
    }
}

impl fmt::Display for ExtensionDocumentPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.relative)
    }
}

#[cfg(test)]
mod tests {
    use super::ExtensionDocumentPath;

    fn path(text: &str) -> Result<ExtensionDocumentPath, crate::SyncError> {
        ExtensionDocumentPath::try_from(text.to_owned())
    }

    #[test]
    fn a_path_names_its_type_by_suffix() {
        let path = path("diagrams/orders.uml").unwrap();

        assert_eq!(path.suffix(), "uml");
        assert_eq!(
            path.segments().collect::<Vec<_>>(),
            ["diagrams", "orders.uml"]
        );
    }

    #[test]
    fn paths_that_could_escape_the_folder_are_refused() {
        for text in [
            "",
            "../secrets.uml",
            "/etc/passwd.uml",
            "a//b.uml",
            ".hidden.uml",
            "a/./b.uml",
            "noext",
            "trailing.",
            "a\\b.uml",
            "a b.uml",
            "a/b/c/d/e/f/g/h/i.uml",
        ] {
            assert!(path(text).is_err(), "{text:?}");
        }
    }
}
