use std::fmt;

use super::ExtensionDocumentPath;

pub(crate) const PROJECT_FILE: &str = "synt.toml";
pub(crate) const RESOURCES: &str = "resources";
pub(crate) const RUNBOOKS: &str = "runbooks";
pub(crate) const EXTENSIONS: &str = "extensions";

const TOML_SUFFIX: &str = ".toml";
const LONGEST_NAME: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DocumentKind {
    Project,
    Resource,
    Runbook,
    Extension {
        extension: String,
        path: ExtensionDocumentPath,
    },
}

impl DocumentKind {
    pub(crate) fn of(path: &str) -> Option<Self> {
        let parts: Vec<&str> = path.split('/').collect();

        match parts.as_slice() {
            [PROJECT_FILE] => Some(Self::Project),
            [RESOURCES, file] => Self::is_toml_name(file).then_some(Self::Resource),
            [RUNBOOKS, file] => Self::is_toml_name(file).then_some(Self::Runbook),
            [EXTENSIONS, extension, inside @ ..] => Self::extension(extension, inside),
            _ => None,
        }
    }

    pub fn is_core(&self) -> bool {
        !matches!(self, Self::Extension { .. })
    }

    pub(super) fn is_name(name: &str) -> bool {
        let allowed =
            |character: char| character.is_ascii_alphanumeric() || "-_.".contains(character);

        !name.is_empty()
            && name.len() <= LONGEST_NAME
            && !name.starts_with('.')
            && name.chars().all(allowed)
    }

    pub(super) fn is_suffix(suffix: &str) -> bool {
        let allowed =
            |character: char| character.is_ascii_lowercase() || character.is_ascii_digit();

        !suffix.is_empty() && suffix.len() <= LONGEST_NAME && suffix.chars().all(allowed)
    }

    fn extension(extension: &str, inside: &[&str]) -> Option<Self> {
        if !Self::is_extension_id(extension) {
            return None;
        }

        let path = ExtensionDocumentPath::try_from(inside.join("/")).ok()?;

        Some(Self::Extension {
            extension: extension.to_owned(),
            path,
        })
    }

    fn is_toml_name(file: &str) -> bool {
        file.strip_suffix(TOML_SUFFIX).is_some_and(Self::is_name)
    }

    fn is_extension_id(id: &str) -> bool {
        let allowed = |character: char| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        };

        id.starts_with(|first: char| first.is_ascii_lowercase())
            && id.len() <= LONGEST_NAME
            && id.chars().all(allowed)
    }
}

impl fmt::Display for DocumentKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Project => formatter.write_str("core.project"),
            Self::Resource => formatter.write_str("core.resource"),
            Self::Runbook => formatter.write_str("core.runbook"),
            Self::Extension { extension, path } => {
                write!(formatter, "ext.{extension}.{}", path.suffix())
            }
        }
    }
}
