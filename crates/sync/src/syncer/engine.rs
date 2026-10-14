use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use serde_json::json;
use synt_journal::{ActionId, Actor, Details, EventKind, JournalWriter, NewEvent};
use synt_platform::Vault;

use super::{OpenFolder, ProjectFolders, Round};
use crate::SyncError;
use crate::clock::unix_millis;
use crate::document::{DocumentPath, ProjectFiles};
use crate::history::{History, SyncedProject};
use crate::id::{MemberId, ProjectId, RevisionId};
use crate::membership::{Membership, Standing};
use crate::merge::DocumentMerger;
use crate::provider::Place;
use crate::replica::{Replica, conflicts};
use crate::report::{
    Access, Applied, Disabled, MembershipChange, RevisionInfo, RoundReport, SyncStatus,
};
use crate::seal::{DeviceKey, Keyring};

const APPLY_NOTE: &str = "sync.apply";

pub(crate) struct Engine {
    history: History,
    vault: Arc<dyn Vault>,
    folders: Arc<dyn ProjectFolders>,
    journal: Option<JournalWriter>,
    merger: Option<Arc<dyn DocumentMerger>>,
    reports: Mutex<BTreeMap<ProjectId, RoundReport>>,
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
            journal,
            merger,
            reports: Mutex::default(),
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
        let synced = self.synced(&self.project_id(project)?)?;
        let id = synced.project.clone();

        self.history.forget(&id)?;
        Membership::forget_keyring(self.vault.as_ref(), &id)?;
        self.lock_reports().remove(&id);

        let left = synced
            .place
            .connect()
            .and_then(|provider| Membership::new(Arc::clone(&self.vault), provider, id.clone()))
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
        let synced = self.synced(&self.project_id(project)?)?;

        self.status_of(&synced)
    }

    pub(crate) fn share(
        &self,
        project: &str,
        member: &MemberId,
    ) -> Result<MembershipChange, SyncError> {
        let _work = self.work();
        let synced = self.synced(&self.project_id(project)?)?;

        let generation = self.membership(&synced)?.share(member)?;

        Ok(MembershipChange {
            project: synced.project,
            member: member.clone(),
            generation,
        })
    }

    pub(crate) fn unshare(
        &self,
        project: &str,
        member: &MemberId,
    ) -> Result<MembershipChange, SyncError> {
        let _work = self.work();
        let synced = self.synced(&self.project_id(project)?)?;

        let generation = self.membership(&synced)?.unshare(member)?;

        Ok(MembershipChange {
            project: synced.project,
            member: member.clone(),
            generation,
        })
    }

    pub(crate) fn history_of(
        &self,
        project: &str,
        path: &DocumentPath,
    ) -> Result<Vec<RevisionInfo>, SyncError> {
        let _work = self.work();
        let folder = self.folder(project)?;
        let synced = self.synced(&folder.id)?;

        let membership = self.membership(&synced)?;
        let keyring = membership.keyring_for_work()?;

        self.replica(&folder, &keyring, &membership)?
            .history_of(path)
    }

    pub(crate) fn resolve(
        &self,
        project: &str,
        path: &DocumentPath,
        revision: &RevisionId,
    ) -> Result<RevisionInfo, SyncError> {
        let _work = self.work();
        let folder = self.folder(project)?;
        let synced = self.synced(&folder.id)?;

        let membership = self.membership(&synced)?;
        let keyring = membership.keyring_for_work()?;
        let replica = self.replica(&folder, &keyring, &membership)?;

        let resolution = replica.resolve(path, revision)?;
        self.round_of(&self.synced(&folder.id)?);

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

    fn round_of(&self, synced: &SyncedProject) -> RoundReport {
        let mut report = RoundReport {
            at: unix_millis(),
            ..RoundReport::default()
        };

        if let Err(error) = self.round_steps(synced, &mut report) {
            report.error = Some(error.to_string());
        }

        self.note_applied(&synced.project, &report.applied);
        self.lock_reports()
            .insert(synced.project.clone(), report.clone());

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
            conflicts: conflicts(&self.history, project)?,
            last_round: self.lock_reports().get(project).cloned(),
        })
    }

    fn note_applied(&self, project: &ProjectId, applied: &[Applied]) {
        let Some(journal) = &self.journal else {
            return;
        };

        for change in applied {
            if let Err(error) = note(journal, project, change) {
                eprintln!("synt-sync: cannot journal {}: {error}", change.path);
            }
        }
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
        let replica = Replica::new(
            self.history.clone(),
            folder.id.clone(),
            ProjectFiles::new(&folder.path),
            keyring.naming(&folder.id)?,
            membership.me(),
        );

        Ok(replica.with_merger(self.merger.clone()))
    }

    fn synced(&self, project: &ProjectId) -> Result<SyncedProject, SyncError> {
        self.history
            .synced(project)?
            .ok_or_else(|| SyncError::NotEnabled(project.to_string()))
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

    fn lock_reports(&self) -> MutexGuard<'_, BTreeMap<ProjectId, RoundReport>> {
        self.reports.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

fn note(journal: &JournalWriter, project: &ProjectId, change: &Applied) -> Result<(), String> {
    let details = json!({
        "project": project,
        "path": change.path,
        "revision": change.revision,
        "author": change.author,
        "removed": change.removed,
        "conflicts": change.conflicts,
    });
    let details = Details::from_serialize(&details).map_err(|error| error.to_string())?;
    let action = ActionId::new(APPLY_NOTE).map_err(|error| error.to_string())?;

    let event = NewEvent::new(Actor::Core, EventKind::Note, action).details(details);

    journal
        .append(event)
        .map(|_| ())
        .map_err(|error| error.to_string())
}
