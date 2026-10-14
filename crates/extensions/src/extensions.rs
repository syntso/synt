use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{
    Arc, Mutex, MutexGuard, OnceLock, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard,
};

use semver::Version;
use synt_actions::{ActionError, Catalog, Description, Handle, Registry, Risk};
use synt_journal::ActionId;
use tokio::runtime::Handle as RuntimeHandle;

use crate::ExtensionError;
use crate::builtin::{self, ExtensionInfo, SearchHit};
use crate::caller::Caller;
use crate::documents::ProjectFolders;
use crate::events::ExtensionEvent;
use crate::extension::{Candidate, Guest, Loaded, Services};
use crate::http::SecretSource;
use crate::install_folder::InstallFolder;
use crate::local_build::LocalBuild;
use crate::manifest::{FileAccess, Manifest};
use crate::package::{Package, sha256};
use crate::remote::{Index, Remote};
use crate::runtime::Runtime;
use crate::screen::Screen;
use crate::shelf::{Installed, Shelf, now};

const FOLDER: &str = "extensions";
const RESERVED: [&str; 2] = ["core", "extension"];
const OWN_NAMESPACE: &str = "extension.";

#[derive(Clone)]
pub struct Extensions {
    runtime: Runtime,
    services: Arc<Services>,
    folder: InstallFolder,
    remote: Remote,
    reserved: Arc<OnceLock<BTreeSet<String>>>,
    core_risks: Arc<OnceLock<BTreeMap<String, Risk>>>,
    loaded: Arc<RwLock<BTreeMap<String, Arc<Loaded>>>>,
    problems: Arc<RwLock<BTreeMap<String, String>>>,
    changing: Arc<Mutex<()>>,
}

impl Extensions {
    pub fn open(
        data: &Path,
        state: &Path,
        runtime: Runtime,
        remote: Remote,
    ) -> Result<Self, ExtensionError> {
        Ok(Self {
            runtime,
            services: Arc::new(Services::new(Shelf::open(state)?)),
            folder: InstallFolder::open(data.join(FOLDER))?,
            remote,
            reserved: Arc::new(OnceLock::new()),
            core_risks: Arc::new(OnceLock::new()),
            loaded: Arc::default(),
            problems: Arc::default(),
            changing: Arc::default(),
        })
    }

    #[must_use]
    pub fn with_secrets(self, secrets: Arc<dyn SecretSource>) -> Self {
        let _ = self.services.secrets.set(secrets);
        self
    }

    #[must_use]
    pub fn with_projects(self, folders: Arc<dyn ProjectFolders>) -> Self {
        let _ = self.services.folders.set(folders);
        self
    }

    pub fn connect(&self, caller: Arc<dyn Caller>, screen: Arc<dyn Screen>) {
        let _ = self.services.caller.set(caller);
        let _ = self.services.screen.set(screen);

        if let Ok(handle) = RuntimeHandle::try_current() {
            let _ = self.services.background.set(handle);
        }

        for loaded in read(&self.loaded).values() {
            loaded.wake_up();
        }
    }

    pub fn register(&self, registry: &mut Registry) -> Result<(), ActionError> {
        builtin::register(self, registry)?;

        let _ = self.reserved.set(namespaces(registry));
        let _ = self.core_risks.set(callable(registry));
        registry.attach(Arc::new(self.clone()));

        Ok(())
    }

    pub fn load_enabled(&self) -> Result<usize, ExtensionError> {
        let _changing = self.begin_change();
        let enabled = self
            .shelf()
            .installed()?
            .into_iter()
            .filter(|entry| entry.enabled);

        let mut loaded = 0;
        for entry in enabled {
            let id = entry.id.clone();

            match self.activate(entry) {
                Ok(()) => loaded += 1,
                Err(error) => self.note_problem(&id, &error),
            }
        }

        Ok(loaded)
    }

    pub fn publish(&self, event: &ExtensionEvent) -> usize {
        read(&self.loaded)
            .values()
            .filter(|loaded| loaded.hear(event))
            .count()
    }

    pub(crate) fn loaded(&self, id: &str) -> Result<Arc<Loaded>, ExtensionError> {
        read(&self.loaded)
            .get(id)
            .cloned()
            .ok_or_else(|| ExtensionError::NotInstalled(id.to_owned()))
    }

    pub(crate) fn all_loaded(&self) -> Vec<Arc<Loaded>> {
        read(&self.loaded).values().cloned().collect()
    }

    pub(crate) fn services(&self) -> &Services {
        &self.services
    }

    pub(crate) fn search(&self, query: Option<&str>) -> Result<Vec<SearchHit>, ExtensionError> {
        let index = self.fresh_index()?;
        let installed: BTreeMap<String, Installed> = self
            .shelf()
            .installed()?
            .into_iter()
            .map(|entry| (entry.id.clone(), entry))
            .collect();

        let hits = index
            .extensions
            .iter()
            .filter(|listing| query.is_none_or(|query| listing.matches(query)))
            .map(|listing| SearchHit::new(listing, installed.get(&listing.id)))
            .collect();

        Ok(hits)
    }

    pub(crate) fn examine_listed(&self, id: &str) -> Result<Candidate, ExtensionError> {
        let index = self.fresh_index()?;
        let listing = index
            .find(id)
            .ok_or_else(|| ExtensionError::NotListed(id.to_owned()))?;

        check_not_older(&listing.version, self.shelf().find(id)?.as_ref())?;
        let package = self.remote.download(listing)?;

        self.examine(package)
    }

    pub(crate) fn examine_local(&self, folder: &Path) -> Result<Candidate, ExtensionError> {
        let package = LocalBuild::new(folder).read()?;

        self.examine(package)
    }

    pub(crate) fn install(&self, candidate: Candidate) -> Result<ExtensionInfo, ExtensionError> {
        let _changing = self.begin_change();
        let Candidate {
            package,
            guest,
            specs,
            ceiling,
            ..
        } = candidate;

        let current = self.shelf().find(&package.manifest.id)?;
        if package.signed {
            check_not_older(&package.manifest.version, current.as_ref())?;
        }
        let clear_items = current.is_some_and(|current| current.signed != package.signed);

        let manifest_sha256 = self.folder.store(&package)?;

        let installed = Installed {
            id: package.manifest.id.clone(),
            version: package.manifest.version.clone(),
            sha256: package.sha256.clone(),
            manifest_sha256,
            signed: package.signed,
            enabled: true,
            added_at: now(),
        };
        self.shelf().record(&installed, clear_items)?;

        let loaded = Arc::new(Loaded::new(guest, specs, ceiling, installed));
        let info = ExtensionInfo::loaded(&loaded);
        self.put_loaded(loaded);

        Ok(info)
    }

    pub(crate) fn list(&self) -> Result<Vec<ExtensionInfo>, ExtensionError> {
        let installed = self.shelf().installed()?;

        Ok(installed
            .into_iter()
            .map(|entry| self.info(entry))
            .collect())
    }

    pub(crate) fn enable(&self, id: &str) -> Result<ExtensionInfo, ExtensionError> {
        let _changing = self.begin_change();
        let mut entry = self.installed(id)?;

        entry.enabled = true;
        self.shelf().set_enabled(id, true)?;

        if let Err(error) = self.activate(entry.clone()) {
            self.note_problem(id, &error);
        }

        Ok(self.info(entry))
    }

    pub(crate) fn disable(&self, id: &str) -> Result<ExtensionInfo, ExtensionError> {
        let _changing = self.begin_change();
        let mut entry = self.installed(id)?;

        entry.enabled = false;
        self.shelf().set_enabled(id, false)?;
        self.take_loaded(id);

        Ok(self.info(entry))
    }

    pub(crate) fn remove(&self, id: &str) -> Result<ExtensionInfo, ExtensionError> {
        let _changing = self.begin_change();
        let entry = self.installed(id)?;
        let info = self.info(entry);

        self.take_loaded(id);
        self.folder.remove(id)?;
        self.shelf().forget(id)?;

        Ok(info)
    }

    fn fresh_index(&self) -> Result<Index, ExtensionError> {
        let index = self.remote.index()?;

        self.shelf().accept_index(index.serial)?;
        Ok(index)
    }

    fn examine(&self, package: Package) -> Result<Candidate, ExtensionError> {
        let manifest = package.manifest.clone();
        manifest.check()?;
        self.check_reserved(&manifest.id)?;
        let ceiling = self.call_ceiling(&manifest)?.max(reach_ceiling(&manifest));
        let replaces = self.shelf().find(&manifest.id)?;

        let compiled = self.runtime.compile(manifest.api, &package.wasm)?;
        let guest = Arc::new(Guest::new(
            manifest,
            compiled,
            self.runtime.clone(),
            Arc::clone(&self.services),
        ));
        let specs: Vec<_> = guest
            .describe()?
            .into_iter()
            .map(|spec| spec.at_least(ceiling))
            .collect();
        check_contributed_actions(&guest)?;

        Ok(Candidate {
            package,
            guest,
            specs,
            ceiling,
            replaces,
        })
    }

    fn activate(&self, entry: Installed) -> Result<(), ExtensionError> {
        let (manifest, manifest_sha256, wasm) = self.folder.read(&entry.id)?;

        let unchanged = sha256(&wasm) == entry.sha256
            && manifest_sha256 == entry.manifest_sha256
            && manifest.id == entry.id;
        if !unchanged {
            return Err(ExtensionError::Refused(format!(
                "the installed files of {} changed since they were installed; install it again",
                entry.id
            )));
        }

        let package = Package::new(manifest, wasm, entry.signed);
        let candidate = self.examine(package)?;

        let loaded = Loaded::new(candidate.guest, candidate.specs, candidate.ceiling, entry);
        self.put_loaded(Arc::new(loaded));

        Ok(())
    }

    fn info(&self, entry: Installed) -> ExtensionInfo {
        if let Some(loaded) = read(&self.loaded).get(&entry.id) {
            return ExtensionInfo::loaded(loaded);
        }

        let problem = read(&self.problems).get(&entry.id).cloned();
        let manifest = self.folder.read(&entry.id).map(|(manifest, _, _)| manifest);

        ExtensionInfo::resting(entry, manifest.ok(), problem)
    }

    fn installed(&self, id: &str) -> Result<Installed, ExtensionError> {
        self.shelf()
            .find(id)?
            .ok_or_else(|| ExtensionError::NotInstalled(id.to_owned()))
    }

    fn check_reserved(&self, id: &str) -> Result<(), ExtensionError> {
        let taken = RESERVED.contains(&id)
            || self
                .reserved
                .get()
                .is_some_and(|reserved| reserved.contains(id));

        if taken {
            return Err(ExtensionError::Refused(format!(
                "the id {id} is taken by the core's own actions"
            )));
        }

        Ok(())
    }

    fn call_ceiling(&self, manifest: &Manifest) -> Result<Risk, ExtensionError> {
        let callable = self.core_risks.get();

        manifest
            .capabilities
            .call
            .iter()
            .try_fold(Risk::Read, |ceiling, action| {
                let risk = callable
                    .and_then(|callable| callable.get(action))
                    .ok_or_else(|| {
                        ExtensionError::Refused(format!(
                            "capabilities.call names {action}, which is not a core action an \
                             extension may call"
                        ))
                    })?;

                Ok(ceiling.max(*risk))
            })
    }

    fn note_problem(&self, id: &str, error: &ExtensionError) {
        eprintln!("synt-core: extension {id} is not loaded: {error}");

        written(&self.problems).insert(id.to_owned(), error.to_string());
    }

    fn put_loaded(&self, loaded: Arc<Loaded>) {
        let id = loaded.installed.id.clone();

        written(&self.problems).remove(&id);
        written(&self.loaded).insert(id, loaded);
    }

    fn take_loaded(&self, id: &str) {
        written(&self.loaded).remove(id);
        written(&self.problems).remove(id);
    }

    fn shelf(&self) -> &Shelf {
        &self.services.shelf
    }

    fn begin_change(&self) -> MutexGuard<'_, ()> {
        self.changing.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Catalog for Extensions {
    fn get(&self, id: &ActionId) -> Option<Handle> {
        let (extension, _) = id.as_str().split_once('.')?;

        read(&self.loaded).get(extension)?.handle(id)
    }

    fn describe(&self) -> Vec<Description> {
        read(&self.loaded)
            .values()
            .flat_map(|loaded| loaded.describe())
            .collect()
    }
}

fn callable(registry: &Registry) -> BTreeMap<String, Risk> {
    registry
        .describe()
        .into_iter()
        .filter(|description| !description.person_only)
        .filter(|description| !description.id.as_str().starts_with(OWN_NAMESPACE))
        .map(|description| (description.id.as_str().to_owned(), description.risk))
        .collect()
}

fn reach_ceiling(manifest: &Manifest) -> Risk {
    let capabilities = &manifest.capabilities;
    let writes_files = capabilities.files == Some(FileAccess::ReadWrite);

    if capabilities.http.is_empty() && !capabilities.documents && !writes_files {
        return Risk::Read;
    }

    Risk::Write
}

fn check_contributed_actions(guest: &Guest) -> Result<(), ExtensionError> {
    let missing = guest
        .manifest()
        .contributes
        .actions()
        .find(|action| !guest.owns_action(action));

    match missing {
        Some(action) => Err(ExtensionError::Component(format!(
            "the manifest contributes {action}, which the extension does not describe"
        ))),
        None => Ok(()),
    }
}

fn check_not_older(listed: &str, installed: Option<&Installed>) -> Result<(), ExtensionError> {
    let Some(installed) = installed else {
        return Ok(());
    };

    let listed = Version::parse(listed).map_err(|error| {
        ExtensionError::Refused(format!(
            "the listed version {listed} is not semver: {error}"
        ))
    })?;
    let Ok(current) = Version::parse(&installed.version) else {
        return Ok(());
    };

    if listed < current {
        return Err(ExtensionError::Refused(format!(
            "the registry offers {} {listed}, older than the installed {current}",
            installed.id
        )));
    }

    Ok(())
}

fn namespaces(registry: &Registry) -> BTreeSet<String> {
    registry
        .ids()
        .filter_map(|id| id.as_str().split_once('.'))
        .map(|(namespace, _)| namespace.to_owned())
        .collect()
}

fn read<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(PoisonError::into_inner)
}

fn written<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(PoisonError::into_inner)
}
