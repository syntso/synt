use std::future;
use std::time::Duration;

use synt_ssh::{ExecLimits, ExecOutput, Pool, PoolBuilder, SshError, Target};
use synt_test_servers::{SshHome, SshServers};

pub struct Setup {
    pub servers: SshServers,
    pub home: SshHome,
    pub pool: Pool,
}

impl Setup {
    pub fn start() -> Option<Self> {
        Self::start_with(SshServers::trusting_home, |_, pool| pool)
    }

    pub fn knowing(home: impl FnOnce(&SshServers) -> SshHome) -> Option<Self> {
        Self::start_with(home, |_, pool| pool)
    }

    pub fn pooled(pool: impl FnOnce(&SshServers, PoolBuilder) -> PoolBuilder) -> Option<Self> {
        Self::start_with(SshServers::trusting_home, pool)
    }

    fn start_with(
        home: impl FnOnce(&SshServers) -> SshHome,
        pool: impl FnOnce(&SshServers, PoolBuilder) -> PoolBuilder,
    ) -> Option<Self> {
        let servers = SshServers::running()?;
        let home = home(&servers);
        let builder = Pool::builder().ssh_directory(home.path());
        let pool = pool(&servers, builder).build();

        Some(Self {
            servers,
            home,
            pool,
        })
    }

    pub fn bastion(&self) -> Target {
        Target::new(
            SshServers::BASTION_ADDRESS,
            self.servers.bastion_port(),
            SshServers::USER,
        )
        .with_identity_files(vec![self.servers.client_key()])
    }

    pub fn target(&self) -> Target {
        Target::new(
            SshServers::TARGET_NAME,
            SshServers::TARGET_PORT,
            SshServers::USER,
        )
        .with_identity_files(vec![self.servers.client_key()])
        .through(vec![self.bastion()])
    }

    pub async fn run(&self, target: &Target, command: &str) -> Result<ExecOutput, SshError> {
        self.run_within(target, command, ExecLimits::default())
            .await
    }

    pub async fn run_within(
        &self,
        target: &Target,
        command: &str,
        limits: ExecLimits,
    ) -> Result<ExecOutput, SshError> {
        self.pool
            .exec(target, command, limits, future::pending())
            .await
    }

    pub async fn client_port(&self, target: &Target) -> String {
        let output = self.run(target, "echo $SSH_CLIENT").await.unwrap();
        let words: Vec<&str> = output.stdout.split_whitespace().collect();

        words[1].to_owned()
    }
}

pub fn seconds(count: u64) -> ExecLimits {
    ExecLimits {
        timeout: Duration::from_secs(count),
        ..ExecLimits::default()
    }
}
