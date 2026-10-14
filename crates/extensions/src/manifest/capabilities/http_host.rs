const LONGEST_HOST: usize = 253;
const LONGEST_SECRET_NAME: usize = 64;

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

#[cfg(test)]
mod tests {
    use super::{check_http_host, check_secret_name};

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
