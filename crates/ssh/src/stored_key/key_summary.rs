use russh::keys::{HashAlg, PrivateKey};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeySummary {
    pub kind: String,
    pub public_key: String,
    pub fingerprint: String,
}

impl KeySummary {
    pub(crate) fn of(key: &PrivateKey) -> Self {
        let public = key.public_key();

        Self {
            kind: public.algorithm().as_str().to_owned(),
            public_key: public.to_openssh().unwrap_or_default(),
            fingerprint: public.fingerprint(HashAlg::Sha256).to_string(),
        }
    }
}
