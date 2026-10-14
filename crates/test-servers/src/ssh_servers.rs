use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::{HostKey, SshHome};

const SWITCH: &str = "SYNT_SSH_TESTS";
const RUN_FOLDER: &str = "../../tests/ssh/run";
const START: &str = "tests/ssh/up.sh";
const DEFAULT_PORT: u16 = 22;

#[derive(Debug, Clone)]
pub struct SshServers {
    run: PathBuf,
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
        if env::var(SWITCH).as_deref() != Ok("1") {
            eprintln!("skipped: set {SWITCH}=1 after {START} to run tests against ssh servers");
            return None;
        }

        let run = Path::new(env!("CARGO_MANIFEST_DIR")).join(RUN_FOLDER);

        Some(Self {
            bastion_port: read(&run, "bastion_port")
                .trim()
                .parse()
                .unwrap_or_else(|_| panic!("{START} wrote a bastion port that is not a number")),
            bastion_key: host_key(&run, "bastion"),
            target_key: host_key(&run, "target"),
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
        self.run.join("client_ed25519")
    }

    pub fn vault_key(&self) -> PathBuf {
        self.run.join("vault_ed25519")
    }

    pub fn trusting_home(&self) -> SshHome {
        SshHome::empty()
            .trusting(&self.bastion_pattern(), &self.bastion_key)
            .trusting(Self::TARGET_NAME, &self.target_key)
    }
}

fn host_key(run: &Path, host: &str) -> HostKey {
    let line = read(run, &format!("{host}/host_ed25519_key.pub"));

    HostKey::parse(&line).unwrap_or_else(|| panic!("the {host} host key from {START} is damaged"))
}

fn read(run: &Path, name: &str) -> String {
    let path = run.join(name);

    fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "{SWITCH}=1 but {} cannot be read ({error}); run {START} first",
            path.display()
        )
    })
}
