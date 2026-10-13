use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError, RwLock, RwLockReadGuard};

use semver::Version;
use synt_actions::{ActionError, Catalog, Description, Handle, Registry, Risk};
use synt_journal::ActionId;

use crate::ExtensionError;
use crate::builtin::{
    ExtensionDisable, ExtensionEnable, ExtensionInfo, ExtensionInstall, ExtensionInstallLocal,
    ExtensionList, ExtensionRemove, ExtensionSearch, SearchHit,
};
use crate::caller::Caller;
use crate::extension::{CallerCell, Candidate, Guest, Loaded};
use crate::install_folder::InstallFolder;
use crate::local_build::LocalBuild;
use crate::manifest::Manifest;
use crate::package::{Package, sha256};
use crate::remote::{Index, Remote};
use crate::runtime::Runtime;
use crate::shelf::{Installed, Shelf, now};

const FOLDER: &str = "extensions";
const RESERVED: [&str; 2] = ["core", "extension"];

#[derive(Clone)]
pub struct Extensions {
    runtime: Runtime,
    shelf: Shelf,
    folder: InstallFolder,
    remote: Remote,
    reserved: Arc<OnceLock<BTreeSet<String>>>,
    core_risks: Arc<OnceLock<BTreeMap<String, Risk>>>,
    caller: CallerCell,
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
            shelf: Shelf::open(state)?,
            folder: InstallFolder::open(data.join(FOLDER))?,
            remote,
            reserved: Arc::new(OnceLock::new()),
            core_risks: Arc::new(OnceLock::new()),
            caller: Arc::new(OnceLock::new()),
            loaded: Arc::default(),
            problems: Arc::default(),
            changing: Arc::default(),
        })
    }

    pub fn connect(&self, caller: Arc<dyn Caller>) {
        let _ = self.caller.set(caller);
    }

    pub fn register(&self, registry: &mut Registry) -> Result<(), ActionError> {
        registry.register(ExtensionSearch::new(self.clone()))?;
        registry.register(ExtensionInstall::new(self.clone()))?;
        registry.register(ExtensionInstallLocal::new(self.clone()))?;
        registry.register(ExtensionList::new(self.clone()))?;
        registry.register(ExtensionEnable::new(self.clone()))?;
        registry.register(ExtensionDisable::new(self.clone()))?;
        registry.register(ExtensionRemove::new(self.clone()))?;

        let _ = self.reserved.set(namespaces(registry));
        let _ = self.core_risks.set(callable(registry));
        registry.attach(Arc::new(self.clone()));

        Ok(())
    }

    pub fn load_enabled(&self) -> Result<usize, ExtensionError> {
        let _changing = self.begin_change();
        let enabled = self
            .shelf
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

    pub(crate) fn search(&self, query: Option<&str>) -> Result<Vec<SearchHit>, ExtensionError> {
        let index = self.fresh_index()?;
        let installed: BTreeMap<String, Installed> = self
            .shelf
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

        check_not_older(&listing.version, self.shelf.find(id)?.as_ref())?;
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
            ..
        } = candidate;

        let current = self.shelf.find(&package.manifest.id)?;
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
            installed_at: now(),
        };
        self.shelf.record(&installed, clear_items)?;

        let loaded = Arc::new(Loaded::new(guest, specs, installed));
        let info = ExtensionInfo::loaded(&loaded);
        self.put_loaded(loaded);

        Ok(info)
    }

    pub(crate) fn list(&self) -> Result<Vec<ExtensionInfo>, ExtensionError> {
        let installed = self.shelf.installed()?;

        Ok(installed
            .into_iter()
            .map(|entry| self.info(entry))
            .collect())
    }

    pub(crate) fn enable(&self, id: &str) -> Result<ExtensionInfo, ExtensionError> {
        let _changing = self.begin_change();
        let mut entry = self.installed(id)?;

        entry.enabled = true;
        self.shelf.set_enabled(id, true)?;

        if let Err(error) = self.activate(entry.clone()) {
            self.note_problem(id, &error);
        }

        Ok(self.info(entry))
    }

    pub(crate) fn disable(&self, id: &str) -> Result<ExtensionInfo, ExtensionError> {
        let _changing = self.begin_change();
        let mut entry = self.installed(id)?;

        entry.enabled = false;
        self.shelf.set_enabled(id, false)?;
        self.take_loaded(id);

        Ok(self.info(entry))
    }

    pub(crate) fn remove(&self, id: &str) -> Result<ExtensionInfo, ExtensionError> {
        let _changing = self.begin_change();
        let entry = self.installed(id)?;
        let info = self.info(entry);

        self.take_loaded(id);
        self.folder.remove(id)?;
        self.shelf.forget(id)?;

        Ok(info)
    }

    fn fresh_index(&self) -> Result<Index, ExtensionError> {
        let index = self.remote.index()?;

        self.shelf.accept_index(index.serial)?;
        Ok(index)
    }

    fn examine(&self, package: Package) -> Result<Candidate, ExtensionError> {
        let manifest = package.manifest.clone();
        manifest.check()?;
        self.check_reserved(&manifest.id)?;
        let ceiling = self.call_ceiling(&manifest)?;
        let replaces = self.shelf.find(&manifest.id)?;

        let compiled = self.runtime.compile(&package.wasm)?;
        let guest = Arc::new(Guest::new(
            manifest,
            compiled,
            self.runtime.clone(),
            self.shelf.clone(),
            Arc::clone(&self.caller),
        ));
        let specs = guest
            .describe()?
            .into_iter()
            .map(|spec| spec.at_least(ceiling))
            .collect();

        Ok(Candidate {
            package,
            guest,
            specs,
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

        let loaded = Loaded::new(candidate.guest, candidate.specs, entry);
        self.put_loaded(Arc::new(loaded));

        Ok(())
    }

    fn info(&self, entry: Installed) -> ExtensionInfo {
        if let Some(loaded) = self.read_loaded().get(&entry.id) {
            return ExtensionInfo::loaded(loaded);
        }

        let problem = self.read_problems().get(&entry.id).cloned();
        let manifest = self.folder.read(&entry.id).map(|(manifest, _, _)| manifest);

        ExtensionInfo::resting(entry, manifest.ok(), problem)
    }

    fn installed(&self, id: &str) -> Result<Installed, ExtensionError> {
        self.shelf
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

        self.problems
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(id.to_owned(), error.to_string());
    }

    fn put_loaded(&self, loaded: Arc<Loaded>) {
        let id = loaded.installed.id.clone();

        self.problems
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&id);
        self.loaded
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(id, loaded);
    }

    fn take_loaded(&self, id: &str) {
        self.loaded
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(id);
        self.problems
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(id);
    }

    fn read_loaded(&self) -> RwLockReadGuard<'_, BTreeMap<String, Arc<Loaded>>> {
        self.loaded.read().unwrap_or_else(PoisonError::into_inner)
    }

    fn read_problems(&self) -> RwLockReadGuard<'_, BTreeMap<String, String>> {
        self.problems.read().unwrap_or_else(PoisonError::into_inner)
    }

    fn begin_change(&self) -> MutexGuard<'_, ()> {
        self.changing.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Catalog for Extensions {
    fn get(&self, id: &ActionId) -> Option<Handle> {
        let (extension, _) = id.as_str().split_once('.')?;

        self.read_loaded().get(extension)?.handle(id)
    }

    fn describe(&self) -> Vec<Description> {
        self.read_loaded()
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
        .map(|description| (description.id.as_str().to_owned(), description.risk))
        .collect()
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
