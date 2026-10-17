use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use synt_journal::{Clock, JournalWriter};
use synt_platform::Vault;

use super::notes::Notes;
use super::reports::Reports;
use super::round::Round;
use crate::document::DocumentPath;
use crate::history::{History, SyncedProject};
use crate::id::{Generation, MemberId, ProjectId, RevisionId};
use crate::membership::{Membership, Standing};
use crate::merge::DocumentMerger;
use crate::provider::Place;
use crate::replica::Replica;
use crate::report::{Disabled, MembershipChange, RevisionInfo, RoundReport, StatusReport};
use crate::seal::{DeviceKey, Keyring};
use crate::{OpenProjects, ProjectFolder, ProjectSlug, SyncError};

pub(crate) struct Engine {
    history: History,
    vault: Arc<dyn Vault>,
    projects: Arc<dyn OpenProjects>,
    merger: Option<Arc<dyn DocumentMerger>>,
    clock: Arc<dyn Clock>,
    notes: Notes,
    reports: Reports,
    work: Mutex<()>,
}

impl Engine {
    pub(crate) fn new(
        history: History,
        vault: Arc<dyn Vault>,
        projects: Arc<dyn OpenProjects>,
        journal: Option<JournalWriter>,
        merger: Option<Arc<dyn DocumentMerger>>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            history,
            vault,
            projects,
            merger,
            clock,
            notes: Notes::new(journal),
            reports: Reports::default(),
            work: Mutex::default(),
        }
    }

    pub(crate) fn enable(
        &self,
        project: &ProjectSlug,
        place: &Place,
    ) -> Result<StatusReport, SyncError> {
        let _work = self.work();
        let folder = self.projects.folder(project)?;
        let id = folder.id();

        if let Some(synced) = self.history.synced(id)?
            && &synced.place != place
        {
            return Err(SyncError::AlreadySyncs {
                project: synced.project,
                place: synced.place,
            });
        }

        let membership = Membership::new(Arc::clone(&self.vault), place.connect()?, id.clone())?;
        membership.join()?;
        self.history.enable(id, place, self.clock.now())?;

        let synced = self.synced(id)?;
        self.round_of(&synced);

        self.status_of(&synced)
    }

    pub(crate) fn disable(&self, project: &ProjectSlug) -> Result<Disabled, SyncError> {
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

    pub(crate) fn status(&self, project: &ProjectSlug) -> Result<StatusReport, SyncError> {
        let _work = self.work();
        let synced = self.synced_by_name(project)?;

        self.status_of(&synced)
    }

    pub(crate) fn share(
        &self,
        project: &ProjectSlug,
        member: &MemberId,
    ) -> Result<MembershipChange, SyncError> {
        self.change_member(project, member, Membership::share)
    }

    pub(crate) fn unshare(
        &self,
        project: &ProjectSlug,
        member: &MemberId,
    ) -> Result<MembershipChange, SyncError> {
        self.change_member(project, member, Membership::unshare)
    }

    pub(crate) fn history_of(
        &self,
        project: &ProjectSlug,
        path: &DocumentPath,
    ) -> Result<Vec<RevisionInfo>, SyncError> {
        let _work = self.work();

        self.working_replica(project)?.history_of(path)
    }

    pub(crate) fn resolve(
        &self,
        project: &ProjectSlug,
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
            .ok_or(SyncError::ResolutionLost)
    }

    pub(crate) fn round_all(&self) -> Vec<RoundReport> {
        let _work = self.work();

        match self.history.synced_projects() {
            Ok(projects) => projects
                .iter()
                .map(|synced| self.round_of(synced))
                .collect(),
            Err(error) => vec![RoundReport::failed(&error, self.clock.now())],
        }
    }

    fn change_member(
        &self,
        project: &ProjectSlug,
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
        let mut report = RoundReport::starting(self.clock.now());

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
        let folder = self.projects.folder(&ProjectSlug::from(&synced.project))?;
        let membership = self.membership(synced)?;

        let Standing::Member(keyring) = membership.standing()? else {
            return Ok(());
        };
        let replica = self.replica(folder, &keyring, &membership)?;

        let round = Round {
            history: &self.history,
            synced,
            provider: membership.provider().as_ref(),
            keyring: &keyring,
            replica: &replica,
        };

        round.run(report)
    }

    fn status_of(&self, synced: &SyncedProject) -> Result<StatusReport, SyncError> {
        let project = &synced.project;
        let membership = self.membership(synced)?;
        let standing = membership.standing()?;

        Ok(StatusReport {
            project: project.clone(),
            place: synced.place.clone(),
            device: membership.me(),
            access: standing.access(),
            generation: standing.generation(),
            members: membership.members()?,
            documents: self.history.documents(project)?.len(),
            unpushed: self.history.unpushed(project)?.len(),
            held: self.history.held(project)?.len(),
            conflicts: self.history.conflicts(project)?,
            last_round: self.reports.last(project),
        })
    }

    fn working_replica(&self, project: &ProjectSlug) -> Result<Replica, SyncError> {
        let folder = self.projects.folder(project)?;
        let synced = self.synced(folder.id())?;

        let membership = self.membership(&synced)?;
        let keyring = membership.keyring_for_work()?;

        self.replica(folder, &keyring, &membership)
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
        folder: ProjectFolder,
        keyring: &Keyring,
        membership: &Membership,
    ) -> Result<Replica, SyncError> {
        let naming = keyring.naming(folder.id())?;

        Ok(Replica::new(
            self.history.clone(),
            folder,
            naming,
            membership.me(),
            self.merger.clone(),
            Arc::clone(&self.clock),
        ))
    }

    fn synced(&self, project: &ProjectId) -> Result<SyncedProject, SyncError> {
        self.history
            .synced(project)?
            .ok_or_else(|| SyncError::NotEnabled(project.clone()))
    }

    fn synced_by_name(&self, project: &ProjectSlug) -> Result<SyncedProject, SyncError> {
        self.synced(&self.project_id(project)?)
    }

    fn project_id(&self, project: &ProjectSlug) -> Result<ProjectId, SyncError> {
        match self.projects.folder(project) {
            Ok(folder) => Ok(folder.id().clone()),
            Err(error) => ProjectId::parse(project.as_str()).map_err(|_| error),
        }
    }

    fn work(&self) -> MutexGuard<'_, ()> {
        self.work.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
