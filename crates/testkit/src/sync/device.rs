use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use synt_sync::{
    DocumentMerger, DocumentPath, MemberId, Place, RevisionInfo, RoundReport, StatusReport, SyncError, Syncer,
};
use tempfile::TempDir;

use super::{OneProject, PROJECT_ID, PROJECT_SLUG};
use crate::{MemoryVault, TestError};

const STATE_FILE: &str = "state.sqlite";

pub struct Device {
    home: TempDir,
    shared: PathBuf,
    vault: Arc<MemoryVault>,
    syncer: Syncer,
}

impl Device {
    pub fn new(shared: &Path) -> Result<Self, TestError> {
        Self::build(shared, None)
    }

    pub fn with_merger(shared: &Path, merger: Arc<dyn DocumentMerger>) -> Result<Self, TestError> {
        Self::build(shared, Some(merger))
    }

    fn build(shared: &Path, merger: Option<Arc<dyn DocumentMerger>>) -> Result<Self, TestError> {
        let home = tempfile::tempdir()?;
        let project = home.path().join(PROJECT_SLUG);
        fs::create_dir_all(project.join("resources"))?;
        fs::write(
            project.join("synt.toml"),
            format!("id = \"{PROJECT_ID}\"\nname = \"Orders\"\n"),
        )?;

        let vault = Arc::new(MemoryVault::default());
        let mut builder = Syncer::builder(
            &home.path().join(STATE_FILE),
            vault.clone(),
            Arc::new(OneProject::new(&project)),
        );
        if let Some(merger) = merger {
            builder = builder.merger(merger);
        }

        Ok(Self {
            home,
            shared: shared.to_owned(),
            vault,
            syncer: builder.open()?,
        })
    }

    pub fn project(&self) -> PathBuf {
        self.home.path().join(PROJECT_SLUG)
    }

    pub fn write(&self, path: &str, text: &str) -> Result<(), TestError> {
        let file = self.project().join(path);
        let folder = file.parent().ok_or("the file has no folder")?;

        fs::create_dir_all(folder)?;
        fs::write(file, text)?;

        Ok(())
    }

    pub fn read(&self, path: &str) -> Option<String> {
        fs::read_to_string(self.project().join(path)).ok()
    }

    pub fn remove(&self, path: &str) -> Result<(), TestError> {
        fs::remove_file(self.project().join(path))?;

        Ok(())
    }

    pub fn vault_items(&self) -> Vec<String> {
        self.vault.names()
    }

    pub fn syncer(&self) -> &Syncer {
        &self.syncer
    }

    pub async fn enable(&self) -> Result<StatusReport, SyncError> {
        self.syncer
            .enable(PROJECT_SLUG, Place::Folder(self.shared.clone()))
            .await
    }

    pub async fn status(&self) -> Result<StatusReport, SyncError> {
        self.syncer.status(PROJECT_SLUG).await
    }

    pub async fn member(&self) -> Result<MemberId, SyncError> {
        Ok(self.status().await?.device)
    }

    pub async fn share_with(&self, other: &Device) -> Result<(), SyncError> {
        let member = other.member().await?;

        self.syncer.share(PROJECT_SLUG, member).await?;

        Ok(())
    }

    pub async fn sync(&self) -> RoundReport {
        let report = self.round().await;
        assert_eq!(report.error, None, "the round failed");

        report
    }

    pub async fn round(&self) -> RoundReport {
        let mut reports = self.syncer.sync_now().await;
        assert_eq!(reports.len(), 1, "one project syncs");

        reports.remove(0)
    }

    pub async fn history(&self, path: &str) -> Result<Vec<RevisionInfo>, SyncError> {
        let path = DocumentPath::parse(path)?;

        self.syncer.history(PROJECT_SLUG, path).await
    }
}
