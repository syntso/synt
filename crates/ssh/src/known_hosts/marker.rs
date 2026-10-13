#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Marker {
    None,
    Revoked,
    CertificateAuthority,
}

impl Marker {
    pub(super) fn parse(token: &str) -> Option<Self> {
        match token {
            "@revoked" => Some(Self::Revoked),
            "@cert-authority" => Some(Self::CertificateAuthority),
            _ => None,
        }
    }
}
