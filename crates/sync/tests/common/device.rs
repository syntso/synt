use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use synt_sync::{
    DocumentMerger, MemberId, Place, RevisionInfo, RoundReport, SyncError, SyncStatus, Syncer,
};
use tempfile::TempDir;

use super::{MemoryVault, OneProject, PROJECT_ID, PROJECT_SLUG};

const STATE_FILE: &str = "state.sqlite";

pub struct Device {
    home: TempDir,
    shared: PathBuf,
    vault: Arc<MemoryVault>,
    syncer: Syncer,
}

impl Device {
    pub fn new(shared: &Path) -> Self {
        Self::build(shared, None)
    }

    pub fn with_merger(shared: &Path, merger: Arc<dyn DocumentMerger>) -> Self {
        Self::build(shared, Some(merger))
    }

    fn build(shared: &Path, merger: Option<Arc<dyn DocumentMerger>>) -> Self {
        let home = tempfile::tempdir().unwrap();
        let project = home.path().join(PROJECT_SLUG);
        fs::create_dir_all(project.join("resources")).unwrap();
        fs::write(
            project.join("synt.toml"),
            format!("id = \"{PROJECT_ID}\"\nname = \"Orders\"\n"),
        )
        .unwrap();

        let vault = Arc::new(MemoryVault::default());
        let mut builder = Syncer::builder(
            &home.path().join(STATE_FILE),
            vault.clone(),
            Arc::new(OneProject::new(&project)),
        );
        if let Some(merger) = merger {
            builder = builder.merger(merger);
        }

        Self {
            home,
            shared: shared.to_owned(),
            vault,
            syncer: builder.open().unwrap(),
        }
    }

    pub fn project(&self) -> PathBuf {
        self.home.path().join(PROJECT_SLUG)
    }

    pub fn write(&self, path: &str, text: &str) {
        let file = self.project().join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, text).unwrap();
    }

    pub fn read(&self, path: &str) -> Option<String> {
        fs::read_to_string(self.project().join(path)).ok()
    }

    pub fn remove(&self, path: &str) {
        fs::remove_file(self.project().join(path)).unwrap();
    }

    pub fn vault_items(&self) -> Vec<String> {
        self.vault.names()
    }

    pub fn syncer(&self) -> &Syncer {
        &self.syncer
    }

    pub async fn enable(&self) -> SyncStatus {
        self.syncer
            .enable(PROJECT_SLUG, Place::Folder(self.shared.clone()))
            .await
            .unwrap()
    }

    pub async fn status(&self) -> SyncStatus {
        self.syncer.status(PROJECT_SLUG).await.unwrap()
    }

    pub async fn member(&self) -> MemberId {
        self.status().await.device
    }

    pub async fn share_with(&self, other: &Device) {
        let member = other.member().await;

        self.syncer.share(PROJECT_SLUG, member).await.unwrap();
    }

    pub async fn sync(&self) -> RoundReport {
        let mut reports = self.syncer.sync_now().await;
        assert_eq!(reports.len(), 1, "one project syncs");

        let report = reports.remove(0);
        assert_eq!(report.error, None, "the round failed");

        report
    }

    pub async fn history(&self, path: &str) -> Result<Vec<RevisionInfo>, SyncError> {
        self.syncer
            .history(PROJECT_SLUG, synt_sync::DocumentPath::parse(path).unwrap())
            .await
    }
}
