use std::path::PathBuf;

use crate::run_folder::RunFolder;
use crate::{HostKey, SshHome};

const SWITCH: &str = "SYNT_KUBE_TESTS";
const RUN_FOLDER: &str = "../../tests/kube/run";
const START: &str = "tests/kube/up.sh";

#[derive(Debug, Clone)]
pub struct KubeServers {
    run: RunFolder,
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
        let run = RunFolder::switched_on(SWITCH, START, RUN_FOLDER, "a cluster")?;

        Some(Self {
            bastion_port: run.bastion_port(),
            bastion_key: run.host_key("bastion"),
            run,
        })
    }

    pub fn kubeconfig(&self) -> PathBuf {
        self.run.file("kubeconfig")
    }

    pub fn kubeconfig_text(&self) -> String {
        self.run.read("kubeconfig")
    }

    pub fn bastion_port(&self) -> u16 {
        self.bastion_port
    }

    pub fn client_key(&self) -> PathBuf {
        self.run.file("client_ed25519")
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
            .unwrap_or_else(|| {
                panic!(
                    "{} wrote a kubeconfig without the direct server",
                    self.run.start()
                )
            })
    }
}
