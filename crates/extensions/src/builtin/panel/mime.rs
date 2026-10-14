const TYPES: [(&str, &str); 13] = [
    ("html", "text/html; charset=utf-8"),
    ("js", "text/javascript; charset=utf-8"),
    ("mjs", "text/javascript; charset=utf-8"),
    ("css", "text/css; charset=utf-8"),
    ("json", "application/json"),
    ("svg", "image/svg+xml"),
    ("png", "image/png"),
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("webp", "image/webp"),
    ("woff2", "font/woff2"),
    ("wasm", "application/wasm"),
    ("txt", "text/plain; charset=utf-8"),
];

pub(super) fn mime_of(path: &str) -> Option<&'static str> {
    let (_, suffix) = path.rsplit_once('.')?;
    let suffix = suffix.to_ascii_lowercase();

    TYPES
        .iter()
        .find(|(known, _)| *known == suffix)
        .map(|(_, mime)| *mime)
}

#[cfg(test)]
mod tests {
    use super::mime_of;

    #[test]
    fn knows_the_files_a_web_panel_is_made_of() {
        assert_eq!(mime_of("index.html"), Some("text/html; charset=utf-8"));
        assert_eq!(
            mime_of("assets/app.JS"),
            Some("text/javascript; charset=utf-8")
        );
        assert_eq!(mime_of("Makefile"), None);
        assert_eq!(mime_of("run.sh"), None);
    }
}
