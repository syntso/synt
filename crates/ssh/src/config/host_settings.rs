use std::path::PathBuf;

use ssh2_config::HostParams;

const NO_JUMP: &str = "none";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct HostSettings {
    pub(crate) host_name: Option<String>,
    pub(crate) user: Option<String>,
    pub(crate) port: Option<u16>,
    pub(crate) identity_files: Vec<PathBuf>,
    pub(crate) proxy_jump: Vec<String>,
}

impl HostSettings {
    fn proxy_jump(params: &HostParams) -> Vec<String> {
        let hops: Vec<String> = params
            .proxy_jump
            .iter()
            .flatten()
            .flat_map(|entry| entry.split(','))
            .map(str::trim)
            .filter(|hop| !hop.is_empty())
            .map(str::to_owned)
            .collect();

        let disabled = hops
            .first()
            .is_some_and(|hop| hop.eq_ignore_ascii_case(NO_JUMP));

        if disabled {
            return Vec::new();
        }

        hops
    }
}

impl From<&HostParams> for HostSettings {
    fn from(params: &HostParams) -> Self {
        Self {
            host_name: params.host_name.clone(),
            user: params.user.clone(),
            port: params.port,
            identity_files: params.identity_file.clone().unwrap_or_default(),
            proxy_jump: Self::proxy_jump(params),
        }
    }
}
