use std::path::PathBuf;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct HostSettings {
    pub(crate) host_name: Option<String>,
    pub(crate) user: Option<String>,
    pub(crate) port: Option<u16>,
    pub(crate) identity_files: Vec<PathBuf>,
    pub(crate) proxy_jump: Vec<String>,
}
