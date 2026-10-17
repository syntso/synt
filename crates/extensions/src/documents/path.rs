use crate::call::FailureCode;
use crate::interface::GuestError;
use crate::manifest::AssetPath;

const LONGEST_PATH: usize = 200;
const MOST_SEGMENTS: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DocumentPath {
    relative: String,
    suffix: String,
}

impl DocumentPath {
    pub(crate) fn parse(text: &str) -> Result<Self, GuestError> {
        let segments: Vec<&str> = text.split('/').collect();

        if text.len() > LONGEST_PATH || segments.len() > MOST_SEGMENTS {
            return Err(Self::invalid(format!(
                "a document path holds at most {MOST_SEGMENTS} parts and {LONGEST_PATH} bytes"
            )));
        }

        if !segments
            .iter()
            .all(|segment| AssetPath::is_plain_segment(segment))
        {
            return Err(Self::invalid(format!(
                "{text:?} is not a document path: use letters, digits, '-', '_' and '.', \
                 with '/' between parts and no part starting with '.'"
            )));
        }

        let suffix = Self::suffix_of(&segments).ok_or_else(|| {
            Self::invalid(format!("{text:?} has no suffix naming its document type"))
        })?;

        Ok(Self {
            relative: text.to_owned(),
            suffix,
        })
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.relative
    }

    pub(crate) fn suffix(&self) -> &str {
        &self.suffix
    }

    pub(crate) fn segments(&self) -> impl Iterator<Item = &str> {
        self.relative.split('/')
    }

    fn suffix_of(segments: &[&str]) -> Option<String> {
        let (_, suffix) = segments.last()?.rsplit_once('.')?;

        (!suffix.is_empty()).then(|| suffix.to_owned())
    }

    fn invalid(reason: String) -> GuestError {
        GuestError::new(FailureCode::InvalidPath, reason)
    }
}

#[cfg(test)]
mod tests {
    use super::DocumentPath;

    #[test]
    fn a_path_names_its_type_by_suffix() {
        let path = DocumentPath::parse("diagrams/orders.uml").unwrap();

        assert_eq!(path.suffix(), "uml");
        assert_eq!(
            path.segments().collect::<Vec<_>>(),
            ["diagrams", "orders.uml"]
        );
    }

    #[test]
    fn paths_that_could_escape_the_folder_are_refused() {
        for path in [
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
        ] {
            assert!(DocumentPath::parse(path).is_err(), "{path:?}");
        }
    }
}
