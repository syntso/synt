use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use synt_platform::{read_plain_file, replace_file};
use synt_ssh::Target;

use super::ProjectInfo;
use super::file::ProjectFile;
use crate::CoreError;
use crate::projects::names::check_name;
use crate::projects::{
    Problem, Resolved, ResolvedDatabase, Resource, ResourceEntry, ResourceListing,
};

pub(crate) const PROJECT_FILE: &str = "synt.toml";
pub(crate) const RESOURCES: &str = "resources";

const EXTENSION: &str = "toml";
const LONGEST_VIA_CHAIN: usize = 8;
const LARGEST_FILE: u64 = 64 * 1024;
const ID_LENGTH: usize = 32;

#[derive(Debug)]
pub(crate) struct Project {
    id: String,
    name: String,
    slug: String,
    path: PathBuf,
    resources: BTreeMap<String, Resource>,
    problems: BTreeMap<String, String>,
}

impl Project {
    pub(crate) fn load(path: &Path, slug: String) -> Result<Self, CoreError> {
        let file = read_or_create_project_file(path, &slug)?;
        let id = file.id.filter(|id| is_project_id(id)).ok_or_else(|| {
            CoreError::Project(format!(
                "{} needs an id of {ID_LENGTH} lowercase hex characters",
                path.join(PROJECT_FILE).display()
            ))
        })?;

        prepare_resources_folder(path)?;

        let mut project = Self {
            id,
            name: file.name.unwrap_or_else(|| slug.clone()),
            slug,
            path: path.to_owned(),
            resources: BTreeMap::new(),
            problems: BTreeMap::new(),
        };
        project.rescan();

        Ok(project)
    }

    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn slug(&self) -> &str {
        &self.slug
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn info(&self) -> ProjectInfo {
        ProjectInfo {
            id: self.id.clone(),
            name: self.name.clone(),
            slug: self.slug.clone(),
            path: self.path.display().to_string(),
            read_only: false,
            resources: self.resources.len(),
            problems: self.problems(),
        }
    }

    pub(crate) fn listing(&self) -> ResourceListing {
        let resources = self
            .resources
            .iter()
            .map(|(name, resource)| ResourceEntry {
                name: name.clone(),
                resource: resource.clone(),
            })
            .collect();

        ResourceListing {
            project: self.slug.clone(),
            resources,
            problems: self.problems(),
        }
    }

    pub(crate) fn resource(&self, name: &str) -> Result<ResourceEntry, CoreError> {
        let resource = self.find(name)?;

        Ok(ResourceEntry {
            name: name.to_owned(),
            resource: resource.clone(),
        })
    }

    pub(crate) fn resolve(&self, name: &str) -> Result<Resolved, CoreError> {
        match self.find(name)? {
            Resource::Host(_) => Ok(Resolved::Host(self.host_target(name)?)),
            Resource::Tunnel(tunnel) => {
                let remote = tunnel
                    .destination()
                    .map_err(|reason| self.broken(name, &reason))?;

                Ok(Resolved::Tunnel {
                    host: self.host_target(&tunnel.host)?,
                    remote,
                    local: tunnel.local,
                })
            }
            Resource::Database(_) => Err(CoreError::Project(format!(
                "@{}/{name} is a database; reach it with the db.* actions",
                self.slug
            ))),
        }
    }

    pub(crate) fn database(&self, name: &str) -> Result<ResolvedDatabase, CoreError> {
        let found = self.find(name)?;
        let Resource::Database(database) = found else {
            return Err(CoreError::Project(format!(
                "@{}/{name} is a {}, not a database",
                self.slug,
                found.kind()
            )));
        };

        let via = database
            .via
            .as_deref()
            .map(|host| self.host_target(host))
            .transpose()?;

        Ok(ResolvedDatabase {
            resource: database.clone(),
            via,
        })
    }

    pub(crate) fn put(&mut self, name: &str, resource: Resource) -> Result<(), CoreError> {
        check_name(name).map_err(CoreError::Project)?;
        resource.check().map_err(CoreError::Project)?;

        let text = resource
            .to_toml()
            .map_err(|error| CoreError::Project(format!("cannot write {name}: {error}")))?;

        check_resources_folder(&self.path)
            .and_then(|()| replace_file(&self.resource_path(name), text.as_bytes()))
            .map_err(|error| CoreError::Project(format!("cannot write {name}: {error}")))?;

        self.resources.insert(name.to_owned(), resource);
        self.problems.remove(&resource_file(name));

        Ok(())
    }

    pub(crate) fn remove(&mut self, name: &str) -> Result<Resource, CoreError> {
        check_name(name).map_err(CoreError::Project)?;

        let resource = self.find(name)?.clone();

        match fs::remove_file(self.resource_path(name)) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error.into()),
            _ => {}
        }

        self.resources.remove(name);
        self.problems.remove(&resource_file(name));

        Ok(resource)
    }

    pub(crate) fn rescan(&mut self) -> Vec<String> {
        let mut changes = self.reread_project_file();
        let found = match self.resource_files() {
            Ok(found) => found,
            Err(error) => {
                changes.push(format!("cannot read {RESOURCES}: {error}"));
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
        self.resources.get(name).ok_or_else(|| {
            let broken = self
                .problems
                .get(&resource_file(name))
                .map(|error| format!(": its file is broken: {error}"))
                .unwrap_or_default();

            CoreError::Project(format!(
                "project {} has no resource {name}{broken}",
                self.slug
            ))
        })
    }

    fn host_target(&self, name: &str) -> Result<Target, CoreError> {
        let mut chain = Vec::new();
        let mut next = Some(name);

        while let Some(current) = next {
            if chain.iter().any(|(known, _)| *known == current) {
                return Err(self.broken(name, &format!("its via chain loops through {current}")));
            }
            if chain.len() > LONGEST_VIA_CHAIN {
                return Err(self.broken(
                    name,
                    &format!("its via chain is longer than {LONGEST_VIA_CHAIN} hops"),
                ));
            }

            let found = self.find(current)?;
            let Resource::Host(host) = found else {
                return Err(CoreError::Project(format!(
                    "{current} in project {} is a {}, not a host",
                    self.slug,
                    found.kind()
                )));
            };

            next = host.via.as_deref();
            chain.push((current, host));
        }

        let mut hops = Vec::new();
        for (hop, host) in chain.iter().rev() {
            hops.push(host.target().map_err(|reason| self.broken(hop, &reason))?);
        }

        let target = hops.pop().ok_or_else(|| self.broken(name, "it is empty"))?;

        Ok(target.through(hops))
    }

    fn broken(&self, name: &str, reason: &str) -> CoreError {
        CoreError::Project(format!("@{}/{name}: {reason}", self.slug))
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
        let path = self.path.join(PROJECT_FILE);

        let file = match read_project_file(&path) {
            Ok(Some(file)) => file,
            Ok(None) => return self.note_problem(PROJECT_FILE, "the file is missing".to_owned()),
            Err(error) => return self.note_problem(PROJECT_FILE, error),
        };

        if file.id.as_deref() != Some(self.id.as_str()) {
            let reason = format!("the id cannot change; {} stays in use", self.id);
            return self.note_problem(PROJECT_FILE, reason);
        }

        let name = file.name.unwrap_or_else(|| self.slug.clone());
        let solved = self.problems.remove(PROJECT_FILE).is_some();

        if name == self.name && !solved {
            return Vec::new();
        }

        self.name = name;
        vec![format!("reloaded {PROJECT_FILE}")]
    }

    fn resource_files(&self) -> io::Result<BTreeMap<String, PathBuf>> {
        check_resources_folder(&self.path)?;

        let mut found = BTreeMap::new();

        for entry in fs::read_dir(self.path.join(RESOURCES))? {
            let entry = entry?;

            if !entry.file_type()?.is_file() {
                continue;
            }

            let path = entry.path();
            let Some(name) = resource_name(&path) else {
                continue;
            };

            found.insert(name, path);
        }

        Ok(found)
    }

    fn forget_missing(&mut self, found: &BTreeMap<String, PathBuf>) -> Vec<String> {
        let missing: Vec<String> = self
            .resources
            .keys()
            .filter(|name| !found.contains_key(*name))
            .cloned()
            .collect();

        self.problems.retain(|file, _| {
            file == PROJECT_FILE || found.keys().any(|name| resource_file(name) == *file)
        });

        missing
            .into_iter()
            .map(|name| {
                self.resources.remove(&name);
                format!("removed {}", resource_file(&name))
            })
            .collect()
    }

    fn reload(&mut self, name: &str, path: &Path) -> Vec<String> {
        let file = resource_file(name);

        let parsed = check_name(name).and_then(|()| read_resource(path));
        let resource = match parsed {
            Ok(resource) => resource,
            Err(error) => {
                let kept = if self.resources.contains_key(name) {
                    "; the last good version stays in use"
                } else {
                    ""
                };
                return self.note_problem(&file, format!("{error}{kept}"));
            }
        };

        let solved = self.problems.remove(&file).is_some();

        if self.resources.get(name) == Some(&resource) && !solved {
            return Vec::new();
        }

        let verb = if self.resources.contains_key(name) {
            "reloaded"
        } else {
            "loaded"
        };
        self.resources.insert(name.to_owned(), resource);

        vec![format!("{verb} {file}")]
    }

    fn note_problem(&mut self, file: &str, error: String) -> Vec<String> {
        if self.problems.get(file) == Some(&error) {
            return Vec::new();
        }

        let change = format!("{file} is broken: {error}");
        self.problems.insert(file.to_owned(), error);

        vec![change]
    }

    fn resource_path(&self, name: &str) -> PathBuf {
        self.path.join(resource_file(name))
    }
}

pub(crate) fn is_project_id(text: &str) -> bool {
    text.len() == ID_LENGTH
        && text
            .chars()
            .all(|character| character.is_ascii_digit() || ('a'..='f').contains(&character))
}

fn prepare_resources_folder(project: &Path) -> Result<(), CoreError> {
    match fs::create_dir(project.join(RESOURCES)) {
        Err(error) if error.kind() != io::ErrorKind::AlreadyExists => return Err(error.into()),
        _ => {}
    }

    check_resources_folder(project).map_err(|error| CoreError::Project(error.to_string()))
}

fn check_resources_folder(project: &Path) -> io::Result<()> {
    let folder = project.join(RESOURCES);

    if fs::symlink_metadata(&folder)?.is_dir() {
        return Ok(());
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("{} must be a plain folder, not a link", folder.display()),
    ))
}

fn read_text(path: &Path) -> io::Result<String> {
    let bytes = read_plain_file(path, LARGEST_FILE)?;

    String::from_utf8(bytes).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} is not UTF-8 text", path.display()),
        )
    })
}

fn resource_file(name: &str) -> String {
    format!("{RESOURCES}/{name}.{EXTENSION}")
}

fn resource_name(path: &Path) -> Option<String> {
    if path.extension()? != EXTENSION {
        return None;
    }

    let stem = path.file_stem()?.to_str()?;

    if stem.starts_with('.') {
        return None;
    }

    Some(stem.to_owned())
}

fn read_resource(path: &Path) -> Result<Resource, String> {
    let text = read_text(path).map_err(|error| error.to_string())?;

    Resource::from_toml(&text)
}

fn read_project_file(path: &Path) -> Result<Option<ProjectFile>, String> {
    let text = match read_text(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };

    toml::from_str(&text)
        .map(Some)
        .map_err(|error| error.message().to_owned())
}

fn read_or_create_project_file(path: &Path, slug: &str) -> Result<ProjectFile, CoreError> {
    let file_path = path.join(PROJECT_FILE);
    let invalid = |reason: String| CoreError::Project(format!("{}: {reason}", file_path.display()));

    let existing = read_project_file(&file_path).map_err(invalid)?;

    if let Some(file) = &existing
        && file.id.is_some()
    {
        return Ok(file.clone());
    }

    let id = new_id()?;
    let old = read_text(&file_path).unwrap_or_default();
    let name_line = if existing
        .as_ref()
        .and_then(|file| file.name.as_ref())
        .is_some()
    {
        String::new()
    } else {
        format!("name = {}\n", toml_string(slug))
    };

    let text = format!("id = \"{id}\"\n{name_line}{old}");
    replace_file(&file_path, text.as_bytes())?;

    let written = read_project_file(&file_path).map_err(invalid)?;
    written.ok_or_else(|| invalid("the file vanished after it was written".to_owned()))
}

fn new_id() -> Result<String, CoreError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| CoreError::Random(error.to_string()))?;

    Ok(data_encoding::HEXLOWER.encode(&bytes))
}

fn toml_string(text: &str) -> String {
    toml::Value::String(text.to_owned()).to_string()
}
