use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use synt_journal::JournalWriter;
use synt_platform::Vault;

use super::{Notes, OpenFolder, ProjectFolders, Reports, Round};
use crate::SyncError;
use crate::clock::unix_millis;
use crate::document::{DocumentPath, ProjectFiles};
use crate::history::{History, SyncedProject};
use crate::id::{Generation, MemberId, ProjectId, RevisionId};
use crate::membership::{Membership, Standing};
use crate::merge::DocumentMerger;
use crate::provider::Place;
use crate::replica::Replica;
use crate::report::{Access, Disabled, MembershipChange, RevisionInfo, RoundReport, SyncStatus};
use crate::seal::{DeviceKey, Keyring};

pub(crate) struct Engine {
    history: History,
    vault: Arc<dyn Vault>,
    folders: Arc<dyn ProjectFolders>,
    merger: Option<Arc<dyn DocumentMerger>>,
    notes: Notes,
    reports: Reports,
    work: Mutex<()>,
}

impl Engine {
    pub(crate) fn new(
        history: History,
        vault: Arc<dyn Vault>,
        folders: Arc<dyn ProjectFolders>,
        journal: Option<JournalWriter>,
        merger: Option<Arc<dyn DocumentMerger>>,
    ) -> Self {
        Self {
            history,
            vault,
            folders,
            merger,
            notes: Notes::new(journal),
            reports: Reports::default(),
            work: Mutex::default(),
        }
    }

    pub(crate) fn enable(&self, project: &str, place: &Place) -> Result<SyncStatus, SyncError> {
        let _work = self.work();
        let folder = self.folder(project)?;

        if let Some(synced) = self.history.synced(&folder.id)?
            && &synced.place != place
        {
            return Err(SyncError::invalid(format!(
                "project {} already syncs through {}; run sync.disable first",
                folder.id, synced.place
            )));
        }

        let membership =
            Membership::new(Arc::clone(&self.vault), place.connect()?, folder.id.clone())?;
        membership.join()?;
        self.history.enable(&folder.id, place)?;

        let synced = self.synced(&folder.id)?;
        self.round_of(&synced);

        self.status_of(&synced)
    }

    pub(crate) fn disable(&self, project: &str) -> Result<Disabled, SyncError> {
        let _work = self.work();
        let synced = self.synced_by_name(project)?;
        let id = synced.project.clone();

        self.history.forget(&id)?;
        Membership::forget_keyring(self.vault.as_ref(), &id)?;
        self.reports.forget(&id);

        let left = self
            .membership(&synced)
            .and_then(|membership| membership.leave())
            .is_ok();

        Ok(Disabled {
            project: id,
            device: DeviceKey::load_or_create(self.vault.as_ref())?.member(),
            left,
        })
    }

    pub(crate) fn status(&self, project: &str) -> Result<SyncStatus, SyncError> {
        let _work = self.work();
        let synced = self.synced_by_name(project)?;

        self.status_of(&synced)
    }

    pub(crate) fn share(
        &self,
        project: &str,
        member: &MemberId,
    ) -> Result<MembershipChange, SyncError> {
        self.change_member(project, member, Membership::share)
    }

    pub(crate) fn unshare(
        &self,
        project: &str,
        member: &MemberId,
    ) -> Result<MembershipChange, SyncError> {
        self.change_member(project, member, Membership::unshare)
    }

    pub(crate) fn history_of(
        &self,
        project: &str,
        path: &DocumentPath,
    ) -> Result<Vec<RevisionInfo>, SyncError> {
        let _work = self.work();

        self.working_replica(project)?.history_of(path)
    }

    pub(crate) fn resolve(
        &self,
        project: &str,
        path: &DocumentPath,
        revision: &RevisionId,
    ) -> Result<RevisionInfo, SyncError> {
        let _work = self.work();
        let replica = self.working_replica(project)?;

        let resolution = replica.resolve(path, revision)?;
        self.round_of(&self.synced(replica.project())?);

        replica
            .history_of(path)?
            .into_iter()
            .find(|info| info.revision == resolution.id)
            .ok_or_else(|| SyncError::invalid("the resolution vanished from the history"))
    }

    pub(crate) fn round_all(&self) -> Vec<RoundReport> {
        let _work = self.work();

        match self.history.synced_projects() {
            Ok(projects) => projects
                .iter()
                .map(|synced| self.round_of(synced))
                .collect(),
            Err(error) => vec![RoundReport {
                at: unix_millis(),
                error: Some(error.to_string()),
                ..RoundReport::default()
            }],
        }
    }

    fn change_member(
        &self,
        project: &str,
        member: &MemberId,
        change: impl FnOnce(&Membership, &MemberId) -> Result<Generation, SyncError>,
    ) -> Result<MembershipChange, SyncError> {
        let _work = self.work();
        let synced = self.synced_by_name(project)?;

        let generation = change(&self.membership(&synced)?, member)?;

        Ok(MembershipChange {
            project: synced.project,
            member: member.clone(),
            generation,
        })
    }

    fn round_of(&self, synced: &SyncedProject) -> RoundReport {
        let mut report = RoundReport {
            at: unix_millis(),
            ..RoundReport::default()
        };

        if let Err(error) = self.round_steps(synced, &mut report) {
            report.error = Some(error.to_string());
        }

        self.notes.applied(&synced.project, &report.applied);
        self.reports.record(&synced.project, &report);

        report
    }

    fn round_steps(
        &self,
        synced: &SyncedProject,
        report: &mut RoundReport,
    ) -> Result<(), SyncError> {
        let folder = self.folder(synced.project.get())?;
        let membership = self.membership(synced)?;

        let Standing::Member(keyring) = membership.standing()? else {
            return Ok(());
        };
        let replica = self.replica(&folder, &keyring, &membership)?;

        let round = Round {
            history: &self.history,
            synced,
            provider: membership.provider().as_ref(),
            keyring: &keyring,
            replica: &replica,
        };

        round.run(report)
    }

    fn status_of(&self, synced: &SyncedProject) -> Result<SyncStatus, SyncError> {
        let project = &synced.project;
        let membership = self.membership(synced)?;

        let (access, generation) = match membership.standing()? {
            Standing::Member(keyring) => (Access::Member, Some(keyring.generation())),
            Standing::Waiting => (Access::Waiting, None),
            Standing::Removed => (Access::Removed, None),
        };

        Ok(SyncStatus {
            project: project.clone(),
            place: synced.place.clone(),
            device: membership.me(),
            access,
            generation,
            members: membership.members()?,
            documents: self.history.documents(project)?.len(),
            unpushed: self.history.unpushed(project)?.len(),
            held: self.history.held(project)?.len(),
            conflicts: self.history.conflicts(project)?,
            last_round: self.reports.last(project),
        })
    }

    fn working_replica(&self, project: &str) -> Result<Replica, SyncError> {
        let folder = self.folder(project)?;
        let synced = self.synced(&folder.id)?;

        let membership = self.membership(&synced)?;
        let keyring = membership.keyring_for_work()?;

        self.replica(&folder, &keyring, &membership)
    }

    fn membership(&self, synced: &SyncedProject) -> Result<Membership, SyncError> {
        Membership::new(
            Arc::clone(&self.vault),
            synced.place.connect()?,
            synced.project.clone(),
        )
    }

    fn replica(
        &self,
        folder: &OpenFolder,
        keyring: &Keyring,
        membership: &Membership,
    ) -> Result<Replica, SyncError> {
        Ok(Replica::new(
            self.history.clone(),
            folder.id.clone(),
            ProjectFiles::new(&folder.path),
            keyring.naming(&folder.id)?,
            membership.me(),
            self.merger.clone(),
        ))
    }

    fn synced(&self, project: &ProjectId) -> Result<SyncedProject, SyncError> {
        self.history
            .synced(project)?
            .ok_or_else(|| SyncError::NotEnabled(project.to_string()))
    }

    fn synced_by_name(&self, project: &str) -> Result<SyncedProject, SyncError> {
        self.synced(&self.project_id(project)?)
    }

    fn folder(&self, project: &str) -> Result<OpenFolder, SyncError> {
        let folder = self.folders.folder(project).map_err(SyncError::Project)?;

        Ok(OpenFolder {
            id: ProjectId::parse(&folder.id)?,
            path: folder.path,
        })
    }

    fn project_id(&self, project: &str) -> Result<ProjectId, SyncError> {
        match self.folder(project) {
            Ok(folder) => Ok(folder.id),
            Err(error) => ProjectId::parse(project).map_err(|_| error),
        }
    }

    fn work(&self) -> MutexGuard<'_, ()> {
        self.work.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
