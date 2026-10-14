use std::fmt;
use std::path::{Path, PathBuf};

use super::DocumentKind;
use crate::SyncError;

pub(crate) const PROJECT_FILE: &str = "synt.toml";
pub(crate) const RESOURCES: &str = "resources";
pub(crate) const RUNBOOKS: &str = "runbooks";
pub(crate) const DOCUMENTS: &str = "documents";

const TOML_SUFFIX: &str = ".toml";
const LONGEST_NAME: usize = 64;

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
                 {DOCUMENTS}/<extension>/<name>.<type>"
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
        [DOCUMENTS, extension, file] => extension_kind(extension, file),
        _ => None,
    }
}

fn toml_name(file: &str) -> bool {
    file.strip_suffix(TOML_SUFFIX).is_some_and(is_name)
}

fn extension_kind(extension: &str, file: &str) -> Option<DocumentKind> {
    let (name, kind) = file.rsplit_once('.')?;

    if !is_extension_id(extension) || !is_name(name) || !is_kind(kind) {
        return None;
    }

    Some(DocumentKind::Extension {
        extension: extension.to_owned(),
        kind: kind.to_owned(),
    })
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

fn is_kind(kind: &str) -> bool {
    !kind.is_empty()
        && kind.len() <= LONGEST_NAME
        && kind
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
        assert_eq!(kind("documents/uml/orders.flow.uml"), "ext.uml.uml");
    }

    #[test]
    fn extension_documents_name_their_owner_and_type() {
        let path = DocumentPath::parse("documents/k8s-board/main.board").unwrap();

        assert_eq!(
            path.kind(),
            &DocumentKind::Extension {
                extension: "k8s-board".to_owned(),
                kind: "board".to_owned()
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
            "documents/uml/x",
            "documents/Uml/x.uml",
            "documents/uml/x.UML",
            "documents/uml/../x.uml",
            "keys/device.toml",
            "synt.toml.bak",
        ];

        for text in refused {
            assert!(DocumentPath::parse(text).is_err(), "{text:?}");
        }
    }

    #[test]
    fn a_path_lands_inside_the_project_folder() {
        let path = DocumentPath::parse("documents/uml/orders.uml").unwrap();

        assert_eq!(
            path.in_folder(Path::new("/p")),
            Path::new("/p")
                .join("documents")
                .join("uml")
                .join("orders.uml")
        );
        assert_eq!(path.folders(), vec!["documents", "documents/uml"]);
    }
}
