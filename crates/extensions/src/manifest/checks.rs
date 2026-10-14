use crate::ExtensionError;

pub(crate) const LONGEST_TEXT: usize = 200;
const LONGEST_LOCAL_ID: usize = 40;
const LONGEST_ASSET_PATH: usize = 200;
const LONGEST_HOST: usize = 253;
const LONGEST_SECRET_NAME: usize = 64;

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

pub(crate) fn check_http_host(entry: &str) -> Result<(), String> {
    let (host, port) = match entry.rsplit_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (entry, None),
    };

    let plain_host = !host.is_empty()
        && host.len() <= LONGEST_HOST
        && !host.starts_with(['.', '-'])
        && !host.ends_with(['.', '-'])
        && !host.contains("..")
        && host.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || ".-".contains(character)
        });
    let plain_port = port.is_none_or(|port| port.parse::<u16>().is_ok_and(|port| port > 0));

    if plain_host && plain_port {
        return Ok(());
    }

    Err(format!(
        "http host {entry:?} must be a lowercase host name or IPv4 address, with an optional \
         :port; patterns and schemes are not allowed"
    ))
}

pub(crate) fn check_secret_name(name: &str) -> Result<(), String> {
    let plain_segment = |segment: &str| {
        !segment.is_empty()
            && segment
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "-_".contains(character))
    };

    if name.is_empty() || name.len() > LONGEST_SECRET_NAME || !name.split('/').all(plain_segment) {
        return Err(format!(
            "secret {name:?} must be 1 to {LONGEST_SECRET_NAME} letters, digits, '-' and '_', \
             with '/' between parts"
        ));
    }

    Ok(())
}

pub(crate) fn refused(reason: impl Into<String>) -> ExtensionError {
    ExtensionError::Manifest(reason.into())
}

#[cfg(test)]
mod tests {
    use super::{check_http_host, check_local_id, check_secret_name, is_asset_path};

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

    #[test]
    fn hosts_are_plain_names_with_an_optional_port() {
        for host in [
            "api.example.com",
            "127.0.0.1:8080",
            "localhost",
            "grafana-1.lan:3000",
        ] {
            assert!(check_http_host(host).is_ok(), "{host}");
        }

        for host in [
            "",
            "*.example.com",
            "https://example.com",
            "Example.com",
            "example.com:0",
            "example.com:99999",
            "example..com",
            "example.com/path",
            ".example.com",
        ] {
            assert!(check_http_host(host).is_err(), "{host}");
        }
    }

    #[test]
    fn secret_names_match_the_vault_rules() {
        assert!(check_secret_name("grafana/token").is_ok());

        for name in ["", "a//b", "../x", "a.b", "a b", "/a"] {
            assert!(check_secret_name(name).is_err(), "{name:?}");
        }
    }
}
