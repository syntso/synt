use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use synt_ssh::Target;

use super::{
    HostResource, Problem, ProjectFile, ProjectId, ProjectInfo, ProjectPlace, ProjectSlug,
    ProjectSource, Resolved, ResolvedDatabase, Resource, ResourceEntry, ResourceFolder,
    ResourceListing, ResourceName,
};
use crate::CoreError;

const LONGEST_VIA_CHAIN: usize = 8;

#[derive(Debug)]
pub(crate) struct ProjectFolder {
    id: ProjectId,
    name: String,
    slug: ProjectSlug,
    path: PathBuf,
    resources: BTreeMap<String, Resource>,
    problems: BTreeMap<String, String>,
}

impl ProjectFolder {
    pub(crate) fn load(path: &Path, slug: ProjectSlug) -> Result<Self, CoreError> {
        let file = ProjectFile::read_or_create(path, &slug)?;
        let id = file
            .id()
            .cloned()
            .ok_or_else(|| CoreError::ProjectIdMissing(ProjectFile::path_in(path)))?;

        ResourceFolder::prepare(path)?;

        let mut folder = Self {
            id,
            name: file.name().map_or_else(|| slug.to_string(), str::to_owned),
            slug,
            path: path.to_owned(),
            resources: BTreeMap::new(),
            problems: BTreeMap::new(),
        };
        folder.rescan();

        Ok(folder)
    }

    pub(crate) fn id(&self) -> &ProjectId {
        &self.id
    }

    pub(crate) fn slug(&self) -> &ProjectSlug {
        &self.slug
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn rescan(&mut self) -> Vec<String> {
        let mut changes = self.reread_project_file();

        let found = match ResourceFolder::of(&self.path).list() {
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

    fn find(&self, name: &str) -> Result<&Resource, CoreError> {
        if let Some(resource) = self.resources.get(name) {
            return Ok(resource);
        }

        let slug = self.slug.to_string();
        let name = name.to_owned();

        match self.problems.get(&ResourceFolder::file_label(&name)) {
            Some(problem) => Err(CoreError::BrokenFile {
                slug,
                name,
                problem: problem.clone(),
            }),
            None => Err(CoreError::NoResource { slug, name }),
        }
    }

    fn host_target(&self, name: &str) -> Result<Target, CoreError> {
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
        name: &'a str,
    ) -> Result<Vec<(&'a str, &'a HostResource)>, CoreError> {
        let mut chain = Vec::new();
        let mut next = Some(name);

        while let Some(current) = next {
            if chain.iter().any(|(known, _)| *known == current) {
                return Err(self.broken(name, CoreError::LoopingChain(current.to_owned())));
            }

            if chain.len() > LONGEST_VIA_CHAIN {
                return Err(self.broken(name, CoreError::LongChain(LONGEST_VIA_CHAIN)));
            }

            let host = self.host(current)?;

            next = host.via.as_deref();
            chain.push((current, host));
        }

        Ok(chain)
    }

    fn host(&self, name: &str) -> Result<&HostResource, CoreError> {
        match self.find(name)? {
            Resource::Host(host) => Ok(host),
            other => Err(CoreError::NotAHost {
                slug: self.slug.to_string(),
                name: name.to_owned(),
                kind: other.kind(),
            }),
        }
    }

    fn via(&self, via: Option<&str>) -> Result<Option<Target>, CoreError> {
        via.map(|via| self.host_target(via)).transpose()
    }

    fn broken(&self, name: &str, reason: CoreError) -> CoreError {
        CoreError::BrokenResource {
            slug: self.slug.to_string(),
            name: name.to_owned(),
            reason: Box::new(reason),
        }
    }

    fn check_admitted(&self, name: &str, admitted: Option<&str>) -> Result<(), CoreError> {
        let actual = format!("@{}/{name}", self.id);

        if admitted == Some(actual.as_str()) {
            return Ok(());
        }

        Err(CoreError::AdmittedElsewhere {
            slug: self.slug.to_string(),
            name: name.to_owned(),
            actual,
        })
    }

    fn problems(&self) -> Vec<Problem> {
        self.problems
            .iter()
            .map(|(file, error)| Problem {
                file: file.clone(),
                error: error.clone(),
            })
            .collect()
    }

    fn reread_project_file(&mut self) -> Vec<String> {
        let file = match ProjectFile::read(&ProjectFile::path_in(&self.path)) {
            Ok(Some(file)) => file,
            Ok(None) => return self.note_problem(ProjectFile::NAME, "the file is missing"),
            Err(error) => return self.note_problem(ProjectFile::NAME, &error.to_string()),
        };

        if file.id() != Some(&self.id) {
            let reason = format!("the id cannot change; {} stays in use", self.id);
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
        let missing: Vec<String> = self
            .resources
            .keys()
            .filter(|name| !found.contains_key(*name))
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
                format!("removed {}", ResourceFolder::file_label(&name))
            })
            .collect()
    }

    fn reload(&mut self, name: &str, path: &Path) -> Vec<String> {
        let file = ResourceFolder::file_label(name);
        let known = self.resources.contains_key(name);

        let parsed = ResourceName::parse(name).and_then(|_| ResourceFolder::read(path));
        let resource = match parsed {
            Ok(resource) => resource,
            Err(error) => {
                let kept = if known {
                    "; the last good version stays in use"
                } else {
                    ""
                };
                return self.note_problem(&file, &format!("{error}{kept}"));
            }
        };

        let solved = self.problems.remove(&file).is_some();

        if self.resources.get(name) == Some(&resource) && !solved {
            return Vec::new();
        }

        let verb = if known { "reloaded" } else { "loaded" };
        self.resources.insert(name.to_owned(), resource);

        vec![format!("{verb} {file}")]
    }

    fn note_problem(&mut self, file: &str, error: &str) -> Vec<String> {
        if self.problems.get(file).map(String::as_str) == Some(error) {
            return Vec::new();
        }

        let change = format!("{file} is broken: {error}");
        self.problems.insert(file.to_owned(), error.to_owned());

        vec![change]
    }
}

impl ProjectSource for ProjectFolder {
    fn info(&self) -> Result<ProjectInfo, CoreError> {
        Ok(ProjectInfo {
            id: self.id.to_string(),
            name: self.name.clone(),
            slug: self.slug.to_string(),
            path: self.path.display().to_string(),
            read_only: false,
            resources: self.resources.len(),
            problems: self.problems(),
        })
    }

    fn listing(&self) -> Result<ResourceListing, CoreError> {
        let resources = self
            .resources
            .iter()
            .map(|(name, resource)| ResourceEntry::new(name, resource.clone()))
            .collect();

        Ok(ResourceListing {
            project: self.slug.to_string(),
            resources,
            problems: self.problems(),
        })
    }

    fn resource(&self, name: &str) -> Result<ResourceEntry, CoreError> {
        let resource = self.find(name)?;

        Ok(ResourceEntry::new(name, resource.clone()))
    }

    fn resolve(&self, name: &str) -> Result<Resolved, CoreError> {
        match self.find(name)? {
            Resource::Host(_) => Ok(Resolved::Host(self.host_target(name)?)),
            Resource::Tunnel(tunnel) => Ok(Resolved::Tunnel {
                host: self.host_target(&tunnel.host)?,
                remote: tunnel
                    .destination()
                    .map_err(|reason| self.broken(name, reason))?,
                local: tunnel.local,
            }),
            Resource::Cluster(cluster) => Ok(Resolved::Cluster {
                spec: cluster.spec().map_err(|reason| self.broken(name, reason))?,
                via: self.via(cluster.via.as_deref())?,
            }),
            Resource::Database(_) => Err(CoreError::IsADatabase {
                slug: self.slug.to_string(),
                name: name.to_owned(),
            }),
        }
    }

    fn database(&self, name: &str) -> Result<ResolvedDatabase, CoreError> {
        let found = self.find(name)?;

        let Resource::Database(database) = found else {
            return Err(CoreError::NotADatabase {
                slug: self.slug.to_string(),
                name: name.to_owned(),
                kind: found.kind(),
            });
        };

        Ok(ResolvedDatabase {
            resource: database.clone(),
            via: self.via(database.via.as_deref())?,
        })
    }

    fn put(
        &mut self,
        name: &str,
        admitted: Option<&str>,
        resource: Resource,
    ) -> Result<ResourceEntry, CoreError> {
        self.check_admitted(name, admitted)?;

        let checked = ResourceName::parse(name)?;
        resource.check()?;

        ResourceFolder::of(&self.path)
            .write(&checked, &resource)
            .map_err(|reason| CoreError::ResourceWrite {
                name: name.to_owned(),
                reason: Box::new(reason),
            })?;

        self.resources.insert(name.to_owned(), resource.clone());
        self.problems.remove(&ResourceFolder::file_label(name));

        Ok(ResourceEntry::new(name, resource))
    }

    fn remove(&mut self, name: &str, admitted: Option<&str>) -> Result<ResourceEntry, CoreError> {
        self.check_admitted(name, admitted)?;

        let checked = ResourceName::parse(name)?;
        let resource = self.find(name)?.clone();

        ResourceFolder::of(&self.path).remove(&checked)?;

        self.resources.remove(name);
        self.problems.remove(&ResourceFolder::file_label(name));

        Ok(ResourceEntry::new(name, resource))
    }

    fn place(&self) -> Result<ProjectPlace, CoreError> {
        Ok(ProjectPlace {
            id: self.id.clone(),
            slug: self.slug.clone(),
            path: self.path.clone(),
        })
    }

    fn canonical_name(&self) -> String {
        self.id.to_string()
    }
}
