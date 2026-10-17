use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use synt_ssh::Target;
use synt_sync::ProjectFolder;

use super::{
    HostResource, Problem, ProjectFile, ProjectId, ProjectInfo, ProjectSlug, ProjectSource,
    Resolved, ResolvedDatabase, Resource, ResourceEntry, ResourceFolder, ResourceListing,
    ResourceName,
};
use crate::CoreError;

const LONGEST_VIA_CHAIN: usize = 8;

#[derive(Debug)]
pub(crate) struct LoadedProject {
    place: ProjectFolder,
    name: String,
    slug: ProjectSlug,
    resources: BTreeMap<ResourceName, Resource>,
    problems: BTreeMap<String, Problem>,
}

impl LoadedProject {
    pub(crate) fn load(path: &Path, slug: ProjectSlug) -> Result<Self, CoreError> {
        let file = ProjectFile::read_or_create(path, &slug)?;
        let id = file
            .id()
            .cloned()
            .ok_or_else(|| CoreError::ProjectIdMissing(ProjectFile::path_in(path)))?;

        ResourceFolder::prepare(path)?;

        let mut project = Self {
            place: ProjectFolder::new(id, path.to_owned()),
            name: file.name().map_or_else(|| slug.to_string(), str::to_owned),
            slug,
            resources: BTreeMap::new(),
            problems: BTreeMap::new(),
        };
        project.rescan();

        Ok(project)
    }

    pub(crate) fn id(&self) -> &ProjectId {
        self.place.id()
    }

    pub(crate) fn slug(&self) -> &ProjectSlug {
        &self.slug
    }

    pub(crate) fn path(&self) -> &Path {
        self.place.root()
    }

    pub(crate) fn rescan(&mut self) -> Vec<String> {
        let mut changes = self.reread_project_file();

        let found = match ResourceFolder::of(self.path()).list() {
            Ok(found) => found,
            Err(error) => {
                changes.push(format!("cannot read {}: {error}", ResourceFolder::NAME));
                return changes;
            }
        };

        changes.extend(self.forget_missing(&found));

        for (name, path) in found {
            changes.extend(self.reload(&name, &path));
        }

        changes
    }

    fn find(&self, name: &ResourceName) -> Result<&Resource, CoreError> {
        if let Some(resource) = self.resources.get(name) {
            return Ok(resource);
        }

        let slug = self.slug.to_string();
        let name = name.to_string();

        match self.problems.get(&ResourceFolder::file_label(&name)) {
            Some(problem) => Err(CoreError::BrokenFile {
                slug,
                name,
                problem: problem.error.clone(),
            }),
            None => Err(CoreError::NoResource { slug, name }),
        }
    }

    fn host_target(&self, name: &ResourceName) -> Result<Target, CoreError> {
        let chain = self.via_chain(name)?;
        let mut hops = Vec::with_capacity(chain.len());

        for (hop, host) in chain.iter().rev() {
            hops.push(host.target().map_err(|reason| self.broken(hop, reason))?);
        }

        let target = hops
            .pop()
            .ok_or_else(|| self.broken(name, CoreError::EmptyChain))?;

        Ok(target.through(hops))
    }

    fn via_chain<'a>(
        &'a self,
        name: &'a ResourceName,
    ) -> Result<Vec<(&'a ResourceName, &'a HostResource)>, CoreError> {
        let mut chain: Vec<(&ResourceName, &HostResource)> = Vec::new();
        let mut next = Some(name);

        while let Some(current) = next {
            if chain.iter().any(|(known, _)| *known == current) {
                return Err(self.broken(name, CoreError::LoopingChain(current.to_string())));
            }

            if chain.len() > LONGEST_VIA_CHAIN {
                return Err(self.broken(name, CoreError::LongChain(LONGEST_VIA_CHAIN)));
            }

            let host = self.host(current)?;

            next = host.via.as_ref();
            chain.push((current, host));
        }

        Ok(chain)
    }

    fn host(&self, name: &ResourceName) -> Result<&HostResource, CoreError> {
        match self.find(name)? {
            Resource::Host(host) => Ok(host),
            other => Err(CoreError::NotAHost {
                slug: self.slug.to_string(),
                name: name.to_string(),
                kind: other.kind(),
            }),
        }
    }

    fn via(&self, via: Option<&ResourceName>) -> Result<Option<Target>, CoreError> {
        via.map(|via| self.host_target(via)).transpose()
    }

    fn broken(&self, name: &ResourceName, reason: CoreError) -> CoreError {
        CoreError::BrokenResource {
            slug: self.slug.to_string(),
            name: name.to_string(),
            reason: Box::new(reason),
        }
    }

    fn check_admitted(&self, name: &ResourceName, admitted: Option<&str>) -> Result<(), CoreError> {
        let actual = format!("@{}/{name}", self.id());

        if admitted == Some(actual.as_str()) {
            return Ok(());
        }

        Err(CoreError::AdmittedElsewhere {
            slug: self.slug.to_string(),
            name: name.to_string(),
            actual,
        })
    }

    fn problems(&self) -> Vec<Problem> {
        self.problems.values().cloned().collect()
    }

    fn reread_project_file(&mut self) -> Vec<String> {
        let file = match ProjectFile::read(&ProjectFile::path_in(self.path())) {
            Ok(Some(file)) => file,
            Ok(None) => return self.note_problem(ProjectFile::NAME, "the file is missing"),
            Err(error) => return self.note_problem(ProjectFile::NAME, &error.to_string()),
        };

        if file.id() != Some(self.id()) {
            let reason = format!("the id cannot change; {} stays in use", self.id());
            return self.note_problem(ProjectFile::NAME, &reason);
        }

        let name = file
            .name()
            .map_or_else(|| self.slug.to_string(), str::to_owned);
        let solved = self.problems.remove(ProjectFile::NAME).is_some();

        if name == self.name && !solved {
            return Vec::new();
        }

        self.name = name;
        vec![format!("reloaded {}", ProjectFile::NAME)]
    }

    fn forget_missing(&mut self, found: &BTreeMap<String, PathBuf>) -> Vec<String> {
        let missing: Vec<ResourceName> = self
            .resources
            .keys()
            .filter(|name| !found.contains_key(name.as_str()))
            .cloned()
            .collect();

        self.problems.retain(|file, _| {
            file == ProjectFile::NAME
                || found
                    .keys()
                    .any(|name| ResourceFolder::file_label(name) == *file)
        });

        missing
            .into_iter()
            .map(|name| {
                self.resources.remove(&name);
                format!("removed {}", ResourceFolder::file_label(name.as_str()))
            })
            .collect()
    }

    fn reload(&mut self, name: &str, path: &Path) -> Vec<String> {
        let file = ResourceFolder::file_label(name);

        let parsed = ResourceName::parse(name)
            .and_then(|name| ResourceFolder::read(path).map(|resource| (name, resource)));
        let (name, resource) = match parsed {
            Ok(parsed) => parsed,
            Err(error) => return self.note_broken(name, &file, &error),
        };

        let known = self.resources.contains_key(&name);
        let solved = self.problems.remove(&file).is_some();

        if self.resources.get(&name) == Some(&resource) && !solved {
            return Vec::new();
        }

        let verb = if known { "reloaded" } else { "loaded" };
        self.resources.insert(name, resource);

        vec![format!("{verb} {file}")]
    }

    fn note_broken(&mut self, name: &str, file: &str, error: &CoreError) -> Vec<String> {
        let known = self.resources.keys().any(|known| known.as_str() == name);
        let kept = if known {
            "; the last good version stays in use"
        } else {
            ""
        };

        self.note_problem(file, &format!("{error}{kept}"))
    }

    fn note_problem(&mut self, file: &str, error: &str) -> Vec<String> {
        if self
            .problems
            .get(file)
            .is_some_and(|known| known.error == error)
        {
            return Vec::new();
        }

        let change = format!("{file} is broken: {error}");
        let problem = Problem {
            file: file.to_owned(),
            error: error.to_owned(),
        };
        self.problems.insert(file.to_owned(), problem);

        vec![change]
    }
}

impl ProjectSource for LoadedProject {
    fn info(&self) -> Result<ProjectInfo, CoreError> {
        Ok(ProjectInfo {
            id: self.id().to_string(),
            name: self.name.clone(),
            slug: self.slug.to_string(),
            path: self.path().display().to_string(),
            read_only: false,
            resources: self.resources.len(),
            problems: self.problems(),
        })
    }

    fn listing(&self) -> Result<ResourceListing, CoreError> {
        let resources = self
            .resources
            .iter()
            .map(|(name, resource)| ResourceEntry::new(name.as_str(), resource.clone()))
            .collect();

        Ok(ResourceListing {
            project: self.slug.to_string(),
            resources,
            problems: self.problems(),
        })
    }

    fn resource(&self, name: &ResourceName) -> Result<ResourceEntry, CoreError> {
        let resource = self.find(name)?;

        Ok(ResourceEntry::new(name.as_str(), resource.clone()))
    }

    fn resolve(&self, name: &ResourceName) -> Result<Resolved, CoreError> {
        match self.find(name)? {
            Resource::Host(_) => Ok(Resolved::Host(self.host_target(name)?)),
            Resource::Tunnel(tunnel) => Ok(Resolved::Tunnel {
                host: self.host_target(&tunnel.host)?,
                remote: tunnel.remote.clone(),
                local: tunnel.local,
            }),
            Resource::Cluster(cluster) => Ok(Resolved::Cluster {
                spec: cluster.spec().map_err(|reason| self.broken(name, reason))?,
                via: self.via(cluster.via.as_ref())?,
            }),
            Resource::Database(_) => Err(CoreError::IsADatabase {
                slug: self.slug.to_string(),
                name: name.to_string(),
            }),
        }
    }

    fn database(&self, name: &ResourceName) -> Result<ResolvedDatabase, CoreError> {
        let found = self.find(name)?;

        let Resource::Database(database) = found else {
            return Err(CoreError::NotADatabase {
                slug: self.slug.to_string(),
                name: name.to_string(),
                kind: found.kind(),
            });
        };

        Ok(ResolvedDatabase {
            resource: database.clone(),
            via: self.via(database.via.as_ref())?,
        })
    }

    fn put(
        &mut self,
        name: &ResourceName,
        admitted: Option<&str>,
        resource: Resource,
    ) -> Result<ResourceEntry, CoreError> {
        self.check_admitted(name, admitted)?;
        resource.check()?;

        ResourceFolder::of(self.path())
            .write(name, &resource)
            .map_err(|reason| CoreError::ResourceWrite {
                name: name.to_string(),
                reason: Box::new(reason),
            })?;

        self.resources.insert(name.clone(), resource.clone());
        self.problems
            .remove(&ResourceFolder::file_label(name.as_str()));

        Ok(ResourceEntry::new(name.as_str(), resource))
    }

    fn remove(
        &mut self,
        name: &ResourceName,
        admitted: Option<&str>,
    ) -> Result<ResourceEntry, CoreError> {
        self.check_admitted(name, admitted)?;

        let resource = self.find(name)?.clone();

        ResourceFolder::of(self.path()).remove(name)?;

        self.resources.remove(name);
        self.problems
            .remove(&ResourceFolder::file_label(name.as_str()));

        Ok(ResourceEntry::new(name.as_str(), resource))
    }

    fn place(&self) -> Result<&ProjectFolder, CoreError> {
        Ok(&self.place)
    }

    fn canonical_name(&self) -> String {
        self.id().to_string()
    }
}
