mod home;
mod host_key;

use std::path::PathBuf;

pub use home::SshHome;
pub use host_key::HostKey;

use crate::run_folder::RunFolder;

const SWITCH: &str = "SYNT_SSH_TESTS";
const RUN_FOLDER: &str = "../../tests/ssh/run";
const START: &str = "tests/ssh/up.sh";
const DEFAULT_PORT: u16 = 22;

#[derive(Debug, Clone)]
pub struct SshServers {
    run: RunFolder,
    bastion_port: u16,
    bastion_key: HostKey,
    target_key: HostKey,
}

impl SshServers {
    pub const USER: &'static str = "synt";
    pub const BASTION_ADDRESS: &'static str = "127.0.0.1";
    pub const TARGET_NAME: &'static str = "target";
    pub const TARGET_PORT: u16 = 22;
    pub const ECHO_PORT: u16 = 7000;

    pub fn running() -> Option<Self> {
        let run = RunFolder::switched_on(SWITCH, START, RUN_FOLDER, "ssh servers")?;

        Some(Self {
            bastion_port: run.bastion_port(),
            bastion_key: run.host_key("bastion"),
            target_key: run.host_key("target"),
            run,
        })
    }

    pub fn bastion_port(&self) -> u16 {
        self.bastion_port
    }

    pub fn bastion_address(&self) -> String {
        if self.bastion_port == DEFAULT_PORT {
            return Self::BASTION_ADDRESS.to_owned();
        }

        format!("{}:{}", Self::BASTION_ADDRESS, self.bastion_port)
    }

    pub fn bastion_pattern(&self) -> String {
        if self.bastion_port == DEFAULT_PORT {
            return Self::BASTION_ADDRESS.to_owned();
        }

        format!("[{}]:{}", Self::BASTION_ADDRESS, self.bastion_port)
    }

    pub fn bastion_key(&self) -> &HostKey {
        &self.bastion_key
    }

    pub fn target_key(&self) -> &HostKey {
        &self.target_key
    }

    pub fn client_key(&self) -> PathBuf {
        self.run.file("client_ed25519")
    }

    pub fn vault_key(&self) -> PathBuf {
        self.run.file("vault_ed25519")
    }

    pub fn trusting_home(&self) -> SshHome {
        SshHome::empty()
            .trusting(&self.bastion_pattern(), &self.bastion_key)
            .trusting(Self::TARGET_NAME, &self.target_key)
    }
}
