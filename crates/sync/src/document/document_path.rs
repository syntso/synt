use std::fmt;
use std::path::{Path, PathBuf};

use super::DocumentKind;
use crate::SyncError;

pub(crate) const PROJECT_FILE: &str = "synt.toml";
pub(crate) const RESOURCES: &str = "resources";
pub(crate) const RUNBOOKS: &str = "runbooks";
pub(crate) const EXTENSIONS: &str = "extensions";
pub(crate) const MOST_EXTENSION_PARTS: usize = 8;

const TOML_SUFFIX: &str = ".toml";
const LONGEST_NAME: usize = 64;
const LONGEST_EXTENSION_PATH: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DocumentPath {
    text: String,
    kind: DocumentKind,
}

impl DocumentPath {
    pub fn parse(text: &str) -> Result<Self, SyncError> {
        let kind = kind_of(text).ok_or_else(|| {
            SyncError::invalid(format!(
                "{text:?} is not a document path: use {PROJECT_FILE}, \
                 {RESOURCES}/<name>.toml, {RUNBOOKS}/<name>.toml or \
                 {EXTENSIONS}/<extension>/<path>.<suffix>"
            ))
        })?;

        Ok(Self {
            text: text.to_owned(),
            kind,
        })
    }

    pub fn get(&self) -> &str {
        &self.text
    }

    pub fn kind(&self) -> &DocumentKind {
        &self.kind
    }

    pub(crate) fn in_folder(&self, root: &Path) -> PathBuf {
        self.text
            .split('/')
            .fold(root.to_owned(), |path, part| path.join(part))
    }

    pub(crate) fn folders(&self) -> Vec<String> {
        let parts: Vec<&str> = self.text.split('/').collect();

        (1..parts.len()).map(|end| parts[..end].join("/")).collect()
    }
}

impl fmt::Display for DocumentPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.text)
    }
}

fn kind_of(text: &str) -> Option<DocumentKind> {
    let parts: Vec<&str> = text.split('/').collect();

    match parts.as_slice() {
        [PROJECT_FILE] => Some(DocumentKind::Project),
        [RESOURCES, file] => toml_name(file).then_some(DocumentKind::Resource),
        [RUNBOOKS, file] => toml_name(file).then_some(DocumentKind::Runbook),
        [EXTENSIONS, extension, inside @ ..] => extension_kind(extension, inside),
        _ => None,
    }
}

fn toml_name(file: &str) -> bool {
    file.strip_suffix(TOML_SUFFIX).is_some_and(is_name)
}

fn extension_kind(extension: &str, inside: &[&str]) -> Option<DocumentKind> {
    let (file, folders) = inside.split_last()?;
    let (name, suffix) = file.rsplit_once('.')?;

    if !is_extension_id(extension)
        || !fits_extension_folder(inside)
        || !folders.iter().all(|folder| is_name(folder))
        || !is_name(name)
        || !is_suffix(suffix)
    {
        return None;
    }

    Some(DocumentKind::Extension {
        extension: extension.to_owned(),
        suffix: suffix.to_owned(),
    })
}

fn fits_extension_folder(inside: &[&str]) -> bool {
    let length = inside.iter().map(|part| part.len() + 1).sum::<usize>();

    inside.len() <= MOST_EXTENSION_PARTS && length <= LONGEST_EXTENSION_PATH + 1
}

fn is_name(name: &str) -> bool {
    let allowed = |character: char| character.is_ascii_alphanumeric() || "-_.".contains(character);

    !name.is_empty()
        && name.len() <= LONGEST_NAME
        && !name.starts_with('.')
        && name.chars().all(allowed)
}

fn is_extension_id(id: &str) -> bool {
    id.starts_with(|first: char| first.is_ascii_lowercase())
        && id.len() <= LONGEST_NAME
        && id.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
}

fn is_suffix(suffix: &str) -> bool {
    !suffix.is_empty()
        && suffix.len() <= LONGEST_NAME
        && suffix
            .chars()
            .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::DocumentPath;
    use crate::document::DocumentKind;

    fn kind(text: &str) -> String {
        DocumentPath::parse(text).unwrap().kind().to_string()
    }

    #[test]
    fn each_kind_of_document_has_its_place() {
        assert_eq!(kind("synt.toml"), "core.project");
        assert_eq!(kind("resources/api-1.toml"), "core.resource");
        assert_eq!(kind("runbooks/restart_api.toml"), "core.runbook");
        assert_eq!(kind("extensions/uml/orders.flow.uml"), "ext.uml.uml");
        assert_eq!(kind("extensions/uml/flows/2026/orders.uml"), "ext.uml.uml");
    }

    #[test]
    fn extension_documents_name_their_owner_and_suffix() {
        let path = DocumentPath::parse("extensions/k8s-board/main.board").unwrap();

        assert_eq!(
            path.kind(),
            &DocumentKind::Extension {
                extension: "k8s-board".to_owned(),
                suffix: "board".to_owned()
            }
        );
    }

    #[test]
    fn paths_that_could_leave_the_project_or_hide_are_refused() {
        let refused = [
            "",
            "/synt.toml",
            "../synt.toml",
            "resources/../synt.toml",
            "resources/../../etc/passwd.toml",
            "resources/.hidden.toml",
            "resources/a/b.toml",
            "resources/api.txt",
            "resources/.toml",
            "resources/api.toml/",
            "resources\\api.toml",
            "runbooks/a b.toml",
            "extensions/uml/x",
            "extensions/Uml/x.uml",
            "extensions/uml/x.UML",
            "extensions/uml/../x.uml",
            "extensions/uml/.drafts/x.uml",
            "extensions/uml/a/b/c/d/e/f/g/h/x.uml",
            "documents/uml/x.uml",
            "keys/device.toml",
            "synt.toml.bak",
        ];

        for text in refused {
            assert!(DocumentPath::parse(text).is_err(), "{text:?}");
        }
    }

    #[test]
    fn a_path_lands_inside_the_project_folder() {
        let path = DocumentPath::parse("extensions/uml/orders.uml").unwrap();

        assert_eq!(
            path.in_folder(Path::new("/p")),
            Path::new("/p")
                .join("extensions")
                .join("uml")
                .join("orders.uml")
        );
        assert_eq!(path.folders(), vec!["extensions", "extensions/uml"]);
    }
}
