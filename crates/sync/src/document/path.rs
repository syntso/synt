use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::DocumentKind;
use crate::SyncError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DocumentPath {
    text: String,
    kind: DocumentKind,
}

impl DocumentPath {
    pub fn parse(text: &str) -> Result<Self, SyncError> {
        Self::try_from(text.to_owned())
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

impl TryFrom<String> for DocumentPath {
    type Error = SyncError;

    fn try_from(text: String) -> Result<Self, SyncError> {
        let Some(kind) = DocumentKind::of(&text) else {
            return Err(SyncError::BadPath(text));
        };

        Ok(Self { text, kind })
    }
}

impl From<DocumentPath> for String {
    fn from(path: DocumentPath) -> Self {
        path.text
    }
}

impl fmt::Display for DocumentPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.text)
    }
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
