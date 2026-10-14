use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::{HostKey, SshHome};

const SWITCH: &str = "SYNT_KUBE_TESTS";
const RUN_FOLDER: &str = "../../tests/kube/run";
const START: &str = "tests/kube/up.sh";

#[derive(Debug, Clone)]
pub struct KubeServers {
    run: PathBuf,
    bastion_port: u16,
    bastion_key: HostKey,
}

impl KubeServers {
    pub const DIRECT: &'static str = "direct";
    pub const INSIDE: &'static str = "inside";
    pub const PLUGIN: &'static str = "plugin";
    pub const INSIDE_HOST: &'static str = "cluster";
    pub const INSIDE_PORT: u16 = 6443;
    pub const BASTION_ADDRESS: &'static str = "127.0.0.1";
    pub const USER: &'static str = "synt";

    pub fn running() -> Option<Self> {
        if env::var(SWITCH).as_deref() != Ok("1") {
            eprintln!("skipped: set {SWITCH}=1 after {START} to run tests against a cluster");
            return None;
        }

        let run = Path::new(env!("CARGO_MANIFEST_DIR")).join(RUN_FOLDER);
        let line = read(&run, "bastion/host_ed25519_key.pub");

        Some(Self {
            bastion_port: read(&run, "bastion_port")
                .trim()
                .parse()
                .unwrap_or_else(|_| panic!("{START} wrote a bastion port that is not a number")),
            bastion_key: HostKey::parse(&line)
                .unwrap_or_else(|| panic!("the bastion host key from {START} is damaged")),
            run,
        })
    }

    pub fn kubeconfig(&self) -> PathBuf {
        self.run.join("kubeconfig")
    }

    pub fn kubeconfig_text(&self) -> String {
        read(&self.run, "kubeconfig")
    }

    pub fn bastion_port(&self) -> u16 {
        self.bastion_port
    }

    pub fn client_key(&self) -> PathBuf {
        self.run.join("client_ed25519")
    }

    pub fn trusting_home(&self) -> SshHome {
        let pattern = format!("[{}]:{}", Self::BASTION_ADDRESS, self.bastion_port);

        SshHome::empty().trusting(&pattern, &self.bastion_key)
    }

    pub fn api_port(&self) -> u16 {
        let text = self.kubeconfig_text();

        text.lines()
            .find_map(|line| line.trim().strip_prefix("server: https://127.0.0.1:"))
            .and_then(|port| port.trim().parse().ok())
            .unwrap_or_else(|| panic!("{START} wrote a kubeconfig without the direct server"))
    }
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
