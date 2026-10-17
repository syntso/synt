use std::path::Path;
use std::sync::Arc;

use synt_sync::DocumentMerger;
use tempfile::TempDir;

use super::Device;
use crate::TestError;

pub struct Team {
    pub shared: TempDir,
    pub alice: Device,
    pub bob: Device,
}

impl Team {
    pub async fn new() -> Result<Self, TestError> {
        let shared = tempfile::tempdir()?;
        let alice = Device::new(shared.path())?;
        let bob = Device::new(shared.path())?;

        Self::join(shared, alice, bob).await
    }

    pub async fn with_merger(merger: Arc<dyn DocumentMerger>) -> Result<Self, TestError> {
        let shared = tempfile::tempdir()?;
        let alice = Device::with_merger(shared.path(), merger.clone())?;
        let bob = Device::with_merger(shared.path(), merger)?;

        Self::join(shared, alice, bob).await
    }

    async fn join(shared: TempDir, alice: Device, bob: Device) -> Result<Self, TestError> {
        alice.enable().await?;
        bob.enable().await?;
        alice.share_with(&bob).await?;

        let team = Self { shared, alice, bob };
        team.settle().await;

        Ok(team)
    }

    pub fn shared(&self) -> &Path {
        self.shared.path()
    }

    pub fn device(&self) -> Result<Device, TestError> {
        Device::new(self.shared())
    }

    pub async fn settle(&self) {
        self.alice.sync().await;
        self.bob.sync().await;
        self.alice.sync().await;
    }
}
