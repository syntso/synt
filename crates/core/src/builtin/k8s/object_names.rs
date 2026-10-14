use synt_actions::ActionError;

use crate::projects::is_namespace;

const LONGEST_NAME: usize = 253;
const FORBIDDEN: [char; 5] = ['/', '?', '#', '%', '\\'];

pub(crate) fn check_object_name(what: &str, name: &str) -> Result<(), ActionError> {
    let unusable = name.is_empty()
        || name.len() > LONGEST_NAME
        || name == "."
        || name == ".."
        || name.contains(FORBIDDEN)
        || name.contains(|character: char| character.is_whitespace() || character.is_control());

    if unusable {
        return Err(ActionError::InvalidParams(format!(
            "{what} must be an object name of 1 to {LONGEST_NAME} characters without '/', '?', \
             '#', '%' or spaces, got {name:?}"
        )));
    }

    Ok(())
}

pub(crate) fn check_namespace(namespace: Option<&str>) -> Result<(), ActionError> {
    match namespace {
        Some(namespace) if !is_namespace(namespace) => Err(ActionError::InvalidParams(format!(
            "namespace must be lowercase letters, digits and '-', got {namespace:?}"
        ))),
        _ => Ok(()),
    }
}

pub(crate) fn check_kind(kind: &str) -> Result<(), ActionError> {
    let usable = !kind.trim().is_empty()
        && kind
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || ".-".contains(character));

    if !usable {
        return Err(ActionError::InvalidParams(format!(
            "kind must be a kind, its plural or a short name, such as pods or deploy, got {kind:?}"
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{check_kind, check_namespace, check_object_name};

    #[test]
    fn ordinary_names_pass() {
        assert!(check_object_name("name", "api-7d9f8b-x2x4q").is_ok());
        assert!(check_object_name("name", "system:controller:job").is_ok());
        assert!(check_kind("deployments.apps").is_ok());
        assert!(check_namespace(Some("orders")).is_ok());
        assert!(check_namespace(None).is_ok());
    }

    #[test]
    fn names_that_would_change_the_request_path_are_refused() {
        for name in [
            "",
            ".",
            "..",
            "a/b",
            "a?watch=1",
            "a#b",
            "a%2Fb",
            "a b",
            "a\\b",
        ] {
            assert!(check_object_name("name", name).is_err(), "{name:?}");
        }
        assert!(check_kind("pods/exec").is_err());
        assert!(check_namespace(Some("Orders")).is_err());
    }
}
