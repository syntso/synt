use crate::ExtensionError;

pub(crate) const LONGEST_TEXT: usize = 200;
const LONGEST_LOCAL_ID: usize = 40;
const LONGEST_ASSET_PATH: usize = 200;

pub(crate) fn check_text(field: &str, text: &str) -> Result<(), ExtensionError> {
    if text.trim().is_empty() || text.len() > LONGEST_TEXT || text.contains(char::is_control) {
        return Err(refused(format!(
            "{field} must be one line of at most {LONGEST_TEXT} bytes"
        )));
    }

    Ok(())
}

pub(crate) fn check_local_id(field: &str, id: &str) -> Result<(), ExtensionError> {
    let starts_with_letter = id.starts_with(|first: char| first.is_ascii_lowercase());
    let plain = id.chars().all(|character| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || "-_".contains(character)
    });

    if !starts_with_letter || !plain || id.len() > LONGEST_LOCAL_ID {
        return Err(refused(format!(
            "{field} {id:?} must start with a lowercase letter and hold only lowercase letters, \
             digits, '-' and '_', at most {LONGEST_LOCAL_ID} of them"
        )));
    }

    Ok(())
}

pub(crate) fn is_asset_path(path: &str) -> bool {
    let plain_segment = |segment: &str| {
        !segment.is_empty()
            && !segment.starts_with('.')
            && segment
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "-_.".contains(character))
    };

    path.len() <= LONGEST_ASSET_PATH && path.split('/').all(plain_segment)
}

pub(crate) fn refused(reason: impl Into<String>) -> ExtensionError {
    ExtensionError::Manifest(reason.into())
}

#[cfg(test)]
mod tests {
    use super::{check_local_id, is_asset_path};

    #[test]
    fn local_ids_are_plain() {
        assert!(check_local_id("view", "load-chart_2").is_ok());

        for id in ["", "Load", "2load", "a.b", "a b"] {
            assert!(check_local_id("view", id).is_err(), "{id:?}");
        }
    }

    #[test]
    fn asset_paths_stay_inside_the_bundle() {
        assert!(is_asset_path("index.html"));
        assert!(is_asset_path("assets/app-1.2.js"));

        for path in [
            "",
            "/index.html",
            "../x",
            "a//b",
            ".env",
            "a/.hidden",
            "a\\b",
            "a b",
        ] {
            assert!(!is_asset_path(path), "{path:?}");
        }
    }
}
