use serde::Serialize;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub(crate) struct Mime(&'static str);

impl Mime {
    pub(crate) fn of(path: &str) -> Option<Self> {
        let (_, suffix) = path.rsplit_once('.')?;
        let suffix = suffix.to_ascii_lowercase();

        TYPES
            .iter()
            .find(|(known, _)| *known == suffix)
            .map(|(_, mime)| Self(mime))
    }
}

#[cfg(test)]
mod tests {
    use super::Mime;

    #[test]
    fn knows_the_files_a_web_panel_is_made_of() {
        assert_eq!(
            Mime::of("index.html").map(|mime| mime.0),
            Some("text/html; charset=utf-8")
        );
        assert_eq!(
            Mime::of("assets/app.JS").map(|mime| mime.0),
            Some("text/javascript; charset=utf-8")
        );
        assert_eq!(Mime::of("Makefile").map(|mime| mime.0), None);
        assert_eq!(Mime::of("run.sh").map(|mime| mime.0), None);
    }
}
