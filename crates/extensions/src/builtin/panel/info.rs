use serde::Serialize;

const CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self' \
     'unsafe-inline'; img-src 'self' data: blob:; font-src 'self'; connect-src 'none'; \
     frame-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'; \
     frame-ancestors 'self'";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct PanelInfo {
    pub extension: String,
    pub panel: String,
    pub title: String,
    pub entry: String,
    pub csp: &'static str,
}

impl PanelInfo {
    pub(crate) fn new(extension: String, panel: String, title: String, entry: String) -> Self {
        Self {
            extension,
            panel,
            title,
            entry,
            csp: CSP,
        }
    }
}
