use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use synt_kube::KubeconfigSource;
use synt_ssh::Target;
use tokio::sync::broadcast;

use super::personal::PERSONAL;
use super::project::is_project_id;
use super::{
    OpenProject, Personal, Project, ProjectChange, ProjectInfo, ProjectPlace, Resolved,
    ResolvedDatabase, Resource, ResourceEntry, ResourceListing, TargetRef,
};
use crate::CoreError;
use crate::locked::locked;
use crate::state::State;

type Opened = BTreeMap<String, OpenProject>;

const CHANGES_KEPT: usize = 256;

#[derive(Clone)]
pub struct Projects {
    state: State,
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
            state,
            opened: Arc::default(),
            changes: broadcast::channel(CHANGES_KEPT).0,
            personal: Personal::new(kubeconfig),
        };

        for path in projects.state.open_projects()? {
            match projects.open(&path) {
                Ok(info) => eprintln!(
                    "synt-core: reopened project {} ({} resources) at {}",
                    info.slug, info.resources, info.path
                ),
                Err(error) => eprintln!(
                    "synt-core: cannot reopen the project at {}: {error}",
                    path.display()
                ),
            }
        }

        Ok(projects)
    }

    pub(crate) fn open(&self, path: &Path) -> Result<ProjectInfo, CoreError> {
        let path = folder(path)?;
        let slug = slug(&path)?;
        let mut opened = self.lock();

        if let Some(open) = opened.get(&slug) {
            let project = open.lock();

            if project.path() == path {
                return Ok(project.info());
            }

            return Err(CoreError::Project(format!(
                "a project named {slug} is already open from {}",
                project.path().display()
            )));
        }

        let project = Project::load(&path, slug.clone())?;
        let info = project.info();

        if let Some((other, _)) = opened.iter().find(|(_, open)| open.lock().id() == info.id) {
            return Err(CoreError::Project(format!(
                "project {other} is already open with the same id {}",
                info.id
            )));
        }

        opened.insert(slug, OpenProject::start(project, self.changes.clone())?);
        self.state.remember_project(&path)?;

        Ok(info)
    }

    pub(crate) fn close(&self, project: &str) -> Result<ProjectInfo, CoreError> {
        if is_personal(project) {
            return Err(CoreError::Project(
                "the Personal project is built in and cannot be closed".to_owned(),
            ));
        }

        let mut opened = self.lock();
        let slug = find_slug(&opened, project)?;

        let closed = opened.remove(&slug).ok_or_else(|| not_open(project))?;
        let info = closed.lock().info();

        self.state.forget_project(Path::new(&info.path))?;
        drop(opened);
        drop(closed);

        Ok(info)
    }

    pub fn changes(&self) -> broadcast::Receiver<ProjectChange> {
        self.changes.subscribe()
    }

    pub(crate) fn folder(&self, project: &str) -> Result<(String, PathBuf), CoreError> {
        if is_personal(project) {
            return Err(Personal::read_only());
        }

        self.with_project(project, |project| {
            Ok((project.id().to_owned(), project.path().to_owned()))
        })
    }

    pub(crate) fn list(&self) -> Result<Vec<ProjectInfo>, CoreError> {
        let mut infos = vec![self.personal.info()?];

        infos.extend(self.lock().values().map(|open| open.lock().info()));

        Ok(infos)
    }

    pub(crate) fn location(&self, project: &str) -> Result<(String, PathBuf), CoreError> {
        if is_personal(project) {
            return Err(CoreError::Project(
                "the Personal project is built from ~/.ssh/config and does not sync".to_owned(),
            ));
        }

        self.with_project(project, |project| {
            Ok((project.id().to_owned(), project.path().to_owned()))
        })
    }

    pub(crate) fn listing(&self, project: &str) -> Result<ResourceListing, CoreError> {
        if is_personal(project) {
            return self.personal.listing();
        }

        self.with_project(project, |project| Ok(project.listing()))
    }

    pub(crate) fn resource(&self, project: &str, name: &str) -> Result<ResourceEntry, CoreError> {
        if is_personal(project) {
            return self.personal.resource(name);
        }

        self.with_project(project, |project| project.resource(name))
    }

    pub(crate) fn put(
        &self,
        target: &str,
        admitted: Option<&str>,
        resource: Resource,
    ) -> Result<ResourceEntry, CoreError> {
        let (project, name) = split_target(target)?;

        if is_personal(project) {
            return Err(Personal::read_only());
        }

        self.with_project(project, |project| {
            check_admitted(project, name, admitted)?;
            project.put(name, resource)?;
            project.resource(name)
        })
    }

    pub(crate) fn remove(
        &self,
        target: &str,
        admitted: Option<&str>,
    ) -> Result<ResourceEntry, CoreError> {
        let (project, name) = split_target(target)?;

        if is_personal(project) {
            return Err(Personal::read_only());
        }

        self.with_project(project, |project| {
            check_admitted(project, name, admitted)?;
            let resource = project.remove(name)?;

            Ok(ResourceEntry {
                name: name.to_owned(),
                resource,
            })
        })
    }

    pub(crate) fn place(&self, project: &str) -> Result<ProjectPlace, CoreError> {
        if is_personal(project) {
            return Err(CoreError::Project(
                "the Personal project is ~/.ssh/config and holds no files of its own; \
                 open a project folder"
                    .to_owned(),
            ));
        }

        self.with_project(project, |project| {
            Ok(ProjectPlace {
                id: project.id().to_owned(),
                slug: project.slug().to_owned(),
                path: project.path().to_owned(),
            })
        })
    }

    pub(crate) fn resolve(&self, text: &str) -> Result<Resolved, CoreError> {
        let Some(reference) = TargetRef::parse(text) else {
            let target = Target::resolve(text)?;
            return Ok(Resolved::Host(target));
        };

        let reference = reference.map_err(CoreError::Project)?;

        if is_personal(reference.project) {
            return self.personal.resolve(reference.resource);
        }

        self.with_project(reference.project, |project| {
            project.resolve(reference.resource)
        })
    }

    pub(crate) fn resolve_cluster(&self, text: &str) -> Result<Resolved, CoreError> {
        let resolved = match TargetRef::parse(text) {
            Some(_) => self.resolve(text)?,
            None => self.personal.context(text),
        };

        if !matches!(resolved, Resolved::Cluster { .. }) {
            return Err(CoreError::Project(format!(
                "{text} is a {}, not a cluster",
                resolved.kind()
            )));
        }

        Ok(resolved)
    }

    pub(crate) fn personal_kubeconfig(&self) -> &KubeconfigSource {
        self.personal.kubeconfig()
    }

    pub(crate) fn database(&self, text: &str) -> Result<ResolvedDatabase, CoreError> {
        let reference = match TargetRef::parse(text) {
            Some(parsed) => parsed.map_err(CoreError::Project)?,
            None => {
                return Err(CoreError::Project(format!(
                    "{text:?} is not a database resource; write @<project>/<database>"
                )));
            }
        };

        if is_personal(reference.project) {
            return Err(CoreError::Project(
                "the Personal project holds ssh hosts only; put databases in a project".to_owned(),
            ));
        }

        self.with_project(reference.project, |project| {
            project.database(reference.resource)
        })
    }

    pub(crate) fn database_destination(&self, text: &str, resolved: &ResolvedDatabase) -> String {
        format!("{} = {}", self.canonical(text), resolved.describe())
    }

    pub(crate) fn destination(&self, text: &str, resolved: &Resolved) -> String {
        format!("{} = {}", self.canonical(text), resolved.describe())
    }

    pub(crate) fn canonical(&self, text: &str) -> String {
        let Some(Ok(reference)) = TargetRef::parse(text) else {
            return text.to_owned();
        };

        if is_personal(reference.project) {
            return format!("@{PERSONAL}/{}", reference.resource);
        }

        let id = self
            .with_project(reference.project, |project| Ok(project.id().to_owned()))
            .unwrap_or_else(|_| reference.project.to_owned());

        format!("@{id}/{}", reference.resource)
    }

    fn with_project<T>(
        &self,
        project: &str,
        work: impl FnOnce(&mut Project) -> Result<T, CoreError>,
    ) -> Result<T, CoreError> {
        let opened = self.lock();
        let slug = find_slug(&opened, project)?;
        let open = opened.get(&slug).ok_or_else(|| not_open(project))?;

        work(&mut open.lock())
    }

    fn lock(&self) -> MutexGuard<'_, Opened> {
        locked(&self.opened)
    }
}

impl fmt::Debug for Projects {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let slugs: Vec<String> = self.lock().keys().cloned().collect();

        formatter
            .debug_struct("Projects")
            .field("opened", &slugs)
            .finish_non_exhaustive()
    }
}

fn folder(path: &Path) -> Result<PathBuf, CoreError> {
    let path = path
        .canonicalize()
        .map_err(|error| CoreError::Project(format!("cannot open {}: {error}", path.display())))?;

    if !path.is_dir() {
        return Err(CoreError::Project(format!(
            "{} is not a folder",
            path.display()
        )));
    }

    Ok(path)
}

fn slug(path: &Path) -> Result<String, CoreError> {
    let slug = path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty() && !name.contains(char::is_control))
        .ok_or_else(|| {
            CoreError::Project(format!(
                "the folder name of {} cannot name a project",
                path.display()
            ))
        })?;

    if is_personal(slug) {
        return Err(CoreError::Project(format!(
            "a project folder cannot be called {slug}: the name belongs to the Personal project"
        )));
    }

    if is_project_id(slug) {
        return Err(CoreError::Project(format!(
            "a project folder cannot be called {slug}: names of that shape are project ids"
        )));
    }

    Ok(slug.to_owned())
}

fn find_slug(opened: &Opened, project: &str) -> Result<String, CoreError> {
    if opened.contains_key(project) {
        return Ok(project.to_owned());
    }

    if !is_project_id(project) {
        return Err(not_open(project));
    }

    opened
        .iter()
        .find(|(_, open)| open.lock().id() == project)
        .map(|(slug, _)| slug.clone())
        .ok_or_else(|| not_open(project))
}

fn split_target(text: &str) -> Result<(&str, &str), CoreError> {
    match TargetRef::parse(text) {
        Some(Ok(reference)) => Ok((reference.project, reference.resource)),
        Some(Err(reason)) => Err(CoreError::Project(reason)),
        None => Err(CoreError::Project(format!(
            "{text:?} is not a project resource; write @<project>/<resource>"
        ))),
    }
}

fn check_admitted(project: &Project, name: &str, admitted: Option<&str>) -> Result<(), CoreError> {
    let actual = format!("@{}/{name}", project.id());

    if admitted == Some(actual.as_str()) {
        return Ok(());
    }

    Err(CoreError::Project(format!(
        "@{}/{name} is now {actual}, which is not what the call was admitted for; \
         run it again",
        project.slug()
    )))
}

fn not_open(project: &str) -> CoreError {
    CoreError::Project(format!("no open project is called {project}"))
}

fn is_personal(project: &str) -> bool {
    project.eq_ignore_ascii_case(PERSONAL)
}
