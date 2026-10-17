mod change;
mod file;
mod info;
mod loaded;
mod local_path;
mod open;
mod personal;
mod problem;
mod resolved;
mod resource;
mod slug;
mod source;
mod sync;
mod table;
mod target_ref;
mod text;

pub use change::ProjectChange;
pub use info::ProjectInfo;
pub use problem::Problem;
pub(crate) use resolved::{Resolved, ResolvedDatabase};
pub use resource::{
    ClusterResource, DatabaseResource, HostResource, Resource, ResourceEntry, ResourceListing,
    TunnelResource,
};
pub(crate) use resource::{ResourceFolder, ResourceName};
pub use slug::ProjectSlug;
pub use synt_sync::ProjectId;
pub(crate) use target_ref::TargetRef;

use file::ProjectFile;
use loaded::LoadedProject;
use local_path::LocalPath;
use open::OpenProject;
use personal::Personal;
use source::ProjectSource;
use table::ProjectTable;
use text::Text;

use std::collections::BTreeMap;
use std::fmt::{self, Display};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use synt_kube::{ClusterSpec, KubeconfigSource};
use synt_platform::Lock;
use synt_ssh::Target;
use synt_sync::ProjectFolder;
use tokio::sync::broadcast;

use crate::CoreError;
use crate::log::log;
use crate::state::State;

type Opened = BTreeMap<ProjectSlug, OpenProject>;

const CHANGES_KEPT: usize = 256;

#[derive(Clone)]
pub struct Projects {
    table: ProjectTable,
    opened: Arc<Mutex<Opened>>,
    changes: broadcast::Sender<ProjectChange>,
    personal: Personal,
}

impl Projects {
    pub fn start(state: State) -> Result<Self, CoreError> {
        Self::start_with(state, KubeconfigSource::Default)
    }

    pub fn start_with(state: State, kubeconfig: KubeconfigSource) -> Result<Self, CoreError> {
        let projects = Self {
            table: ProjectTable::new(state),
            opened: Arc::default(),
            changes: broadcast::channel(CHANGES_KEPT).0,
            personal: Personal::new(kubeconfig),
        };

        projects.reopen()?;

        Ok(projects)
    }

    pub fn changes(&self) -> broadcast::Receiver<ProjectChange> {
        self.changes.subscribe()
    }

    pub(crate) fn open(&self, path: &Path) -> Result<ProjectInfo, CoreError> {
        let path = Self::folder_path(path)?;
        let slug = ProjectSlug::of_folder(&path)?;
        let mut opened = self.lock();

        if let Some(open) = opened.get(&slug) {
            return Self::already_open(&open.lock(), &path);
        }

        let folder = LoadedProject::load(&path, slug.clone())?;
        let info = folder.info()?;

        if let Some((other, _)) = opened
            .iter()
            .find(|(_, open)| open.lock().id() == folder.id())
        {
            return Err(CoreError::IdTaken {
                other: other.to_string(),
                id: info.id,
            });
        }

        opened.insert(slug, OpenProject::start(folder, self.changes.clone())?);
        self.table.remember(&path)?;

        Ok(info)
    }

    pub(crate) fn close(&self, project: &ProjectSlug) -> Result<ProjectInfo, CoreError> {
        if Personal::is_named(project.as_str()) {
            return Err(CoreError::PersonalCannotClose);
        }

        let mut opened = self.lock();
        let slug = Self::find_slug(&opened, project)?;

        let closed = opened
            .remove(&slug)
            .ok_or_else(|| CoreError::ProjectNotOpen(project.to_string()))?;
        let info = closed.lock().info()?;

        self.table.forget(Path::new(&info.path))?;
        drop(opened);
        drop(closed);

        Ok(info)
    }

    pub(crate) fn list(&self) -> Result<Vec<ProjectInfo>, CoreError> {
        let mut infos = vec![self.personal.info()?];

        for open in self.lock().values() {
            infos.push(open.lock().info()?);
        }

        Ok(infos)
    }

    pub(crate) fn listing(&self, project: &ProjectSlug) -> Result<ResourceListing, CoreError> {
        self.with_source(project, |source| source.listing())
    }

    pub(crate) fn resource(
        &self,
        project: &ProjectSlug,
        name: &ResourceName,
    ) -> Result<ResourceEntry, CoreError> {
        self.with_source(project, |source| source.resource(name))
    }

    pub(crate) fn put(
        &self,
        target: &TargetRef,
        admitted: Option<&str>,
        resource: Resource,
    ) -> Result<ResourceEntry, CoreError> {
        self.with_source(target.project(), |source| {
            source.put(target.resource(), admitted, resource)
        })
    }

    pub(crate) fn remove(
        &self,
        target: &TargetRef,
        admitted: Option<&str>,
    ) -> Result<ResourceEntry, CoreError> {
        self.with_source(target.project(), |source| {
            source.remove(target.resource(), admitted)
        })
    }

    pub(crate) fn place(&self, project: &ProjectSlug) -> Result<ProjectFolder, CoreError> {
        self.with_source(project, |source| source.place().cloned())
    }

    pub(crate) fn slug(&self, project: &ProjectSlug) -> Result<ProjectSlug, CoreError> {
        Self::find_slug(&self.lock(), project)
    }

    pub(crate) fn resolve(&self, text: &str) -> Result<Resolved, CoreError> {
        match TargetRef::parse(text) {
            Some(target) => self.resolve_target(&target?),
            None => Ok(Resolved::Host(Target::resolve(text)?)),
        }
    }

    pub(crate) fn resolve_target(&self, target: &TargetRef) -> Result<Resolved, CoreError> {
        if Personal::is_named(target.project().as_str()) {
            return self.personal.resolve(target.resource());
        }

        let resolved =
            self.with_source(target.project(), |source| source.resolve(target.resource()))?;

        self.check_kubeconfig(&resolved)?;

        Ok(resolved)
    }

    pub(crate) fn resolve_cluster(&self, text: &str) -> Result<Resolved, CoreError> {
        let resolved = match TargetRef::parse(text) {
            Some(_) => self.resolve(text)?,
            None => self.personal.context(text),
        };

        if !matches!(resolved, Resolved::Cluster { .. }) {
            return Err(CoreError::NotACluster {
                text: text.to_owned(),
                kind: resolved.kind(),
            });
        }

        Ok(resolved)
    }

    pub(crate) fn database(&self, text: &str) -> Result<ResolvedDatabase, CoreError> {
        let target = TargetRef::parse(text)
            .ok_or_else(|| CoreError::NotADatabaseTarget(text.to_owned()))??;

        self.with_source(target.project(), |source| {
            source.database(target.resource())
        })
    }

    pub(crate) fn personal_kubeconfig(&self) -> &KubeconfigSource {
        self.personal.kubeconfig()
    }

    pub(crate) fn destination(&self, text: &str, reached: &impl Display) -> String {
        format!("{} = {reached}", self.canonical(text))
    }

    pub(crate) fn canonical(&self, text: &str) -> String {
        let Some(Ok(target)) = TargetRef::parse(text) else {
            return text.to_owned();
        };

        let project = self
            .with_source(target.project(), |source| Ok(source.canonical_name()))
            .unwrap_or_else(|_| target.project().to_string());

        format!("@{project}/{}", target.resource())
    }

    fn reopen(&self) -> Result<(), CoreError> {
        for path in self.table.open()? {
            match self.open(&path) {
                Ok(info) => log!(
                    "reopened project {} ({} resources) at {}",
                    info.slug,
                    info.resources,
                    info.path
                ),
                Err(error) => log!("cannot reopen the project at {}: {error}", path.display()),
            }
        }

        Ok(())
    }

    fn with_source<T>(
        &self,
        project: &ProjectSlug,
        work: impl FnOnce(&mut dyn ProjectSource) -> Result<T, CoreError>,
    ) -> Result<T, CoreError> {
        if Personal::is_named(project.as_str()) {
            return work(&mut self.personal.clone());
        }

        let opened = self.lock();
        let slug = Self::find_slug(&opened, project)?;
        let open = opened
            .get(&slug)
            .ok_or_else(|| CoreError::ProjectNotOpen(project.to_string()))?;

        work(&mut *open.lock())
    }

    fn check_kubeconfig(&self, resolved: &Resolved) -> Result<(), CoreError> {
        let Resolved::Cluster {
            spec:
                ClusterSpec {
                    kubeconfig: KubeconfigSource::File(path),
                    ..
                },
            ..
        } = resolved
        else {
            return Ok(());
        };

        let settled = Self::settled_path(path);
        let roots: Vec<PathBuf> = self
            .lock()
            .values()
            .map(|open| open.lock().path().to_owned())
            .collect();

        let Some(root) = roots.into_iter().find(|root| settled.starts_with(root)) else {
            return Ok(());
        };

        Err(CoreError::KubeconfigInsideProject {
            kubeconfig: path.clone(),
            folder: root,
        })
    }

    fn already_open(folder: &LoadedProject, path: &Path) -> Result<ProjectInfo, CoreError> {
        if folder.path() == path {
            return folder.info();
        }

        Err(CoreError::SlugTaken {
            slug: folder.slug().to_string(),
            path: folder.path().to_owned(),
        })
    }

    fn find_slug(opened: &Opened, project: &ProjectSlug) -> Result<ProjectSlug, CoreError> {
        let not_open = || CoreError::ProjectNotOpen(project.to_string());

        if opened.contains_key(project) {
            return Ok(project.clone());
        }

        let id = ProjectId::parse(project.as_str()).map_err(|_| not_open())?;

        opened
            .iter()
            .find(|(_, open)| *open.lock().id() == id)
            .map(|(slug, _)| slug.clone())
            .ok_or_else(not_open)
    }

    fn folder_path(path: &Path) -> Result<PathBuf, CoreError> {
        let path = path
            .canonicalize()
            .map_err(|source| CoreError::UnreadableFolder {
                path: path.to_owned(),
                source,
            })?;

        if !path.is_dir() {
            return Err(CoreError::NotAFolder(path));
        }

        Ok(path)
    }

    fn settled_path(path: &Path) -> PathBuf {
        if let Ok(found) = path.canonicalize() {
            return found;
        }

        let parent = path.parent().and_then(|parent| parent.canonicalize().ok());

        match (parent, path.file_name()) {
            (Some(parent), Some(name)) => parent.join(name),
            _ => path.to_owned(),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Opened> {
        self.opened.locked()
    }
}

impl fmt::Debug for Projects {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let slugs: Vec<ProjectSlug> = self.lock().keys().cloned().collect();

        formatter
            .debug_struct("Projects")
            .field("opened", &slugs)
            .finish_non_exhaustive()
    }
}
