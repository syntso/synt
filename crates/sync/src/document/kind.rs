use std::fmt;

pub(crate) const PROJECT_FILE: &str = "synt.toml";
pub(crate) const RESOURCES: &str = "resources";
pub(crate) const RUNBOOKS: &str = "runbooks";
pub(crate) const EXTENSIONS: &str = "extensions";
pub(crate) const MOST_EXTENSION_PARTS: usize = 8;

const TOML_SUFFIX: &str = ".toml";
const LONGEST_NAME: usize = 64;
const LONGEST_EXTENSION_PATH: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DocumentKind {
    Project,
    Resource,
    Runbook,
    Extension { extension: String, suffix: String },
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

    fn extension(extension: &str, inside: &[&str]) -> Option<Self> {
        let (file, folders) = inside.split_last()?;
        let (name, suffix) = file.rsplit_once('.')?;

        let valid = Self::is_extension_id(extension)
            && Self::fits_extension_folder(inside)
            && folders.iter().all(|folder| Self::is_name(folder))
            && Self::is_name(name)
            && Self::is_suffix(suffix);

        valid.then(|| Self::Extension {
            extension: extension.to_owned(),
            suffix: suffix.to_owned(),
        })
    }

    fn is_toml_name(file: &str) -> bool {
        file.strip_suffix(TOML_SUFFIX).is_some_and(Self::is_name)
    }

    fn fits_extension_folder(inside: &[&str]) -> bool {
        let length = inside.iter().map(|part| part.len() + 1).sum::<usize>();

        inside.len() <= MOST_EXTENSION_PARTS && length <= LONGEST_EXTENSION_PATH + 1
    }

    fn is_name(name: &str) -> bool {
        let allowed =
            |character: char| character.is_ascii_alphanumeric() || "-_.".contains(character);

        !name.is_empty()
            && name.len() <= LONGEST_NAME
            && !name.starts_with('.')
            && name.chars().all(allowed)
    }

    fn is_extension_id(id: &str) -> bool {
        let allowed = |character: char| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        };

        id.starts_with(|first: char| first.is_ascii_lowercase())
            && id.len() <= LONGEST_NAME
            && id.chars().all(allowed)
    }

    fn is_suffix(suffix: &str) -> bool {
        let allowed =
            |character: char| character.is_ascii_lowercase() || character.is_ascii_digit();

        !suffix.is_empty() && suffix.len() <= LONGEST_NAME && suffix.chars().all(allowed)
    }
}

impl fmt::Display for DocumentKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Project => formatter.write_str("core.project"),
            Self::Resource => formatter.write_str("core.resource"),
            Self::Runbook => formatter.write_str("core.runbook"),
            Self::Extension { extension, suffix } => {
                write!(formatter, "ext.{extension}.{suffix}")
            }
        }
    }
}
