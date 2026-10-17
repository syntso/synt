use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use synt_actions::{ActionDescription, ActionError, Catalog, Handle, Registry, Risk};
use synt_journal::{ActionId, SystemClock};

use crate::ExtensionError;
use crate::background::ExtensionEvent;
use crate::builtin::{ExtensionInfo, Registration, SearchHit};
use crate::call::Caller;
use crate::documents::{MergeSides, ProjectFolders};
use crate::extension::{ActionSpec, Candidate, CoreLink, Guest, Loaded, Roster, Services};
use crate::http::SecretSource;
use crate::install::{Admission, InstallFolder, InstalledFiles, LocalBuild, Package};
use crate::manifest::Manifest;
use crate::remote::{Index, Remote};
use crate::runtime::Runtime;
use crate::shelf::{Installed, Inventory, Shelf};
use crate::ui::Screen;

const FOLDER: &str = "extensions";

#[derive(Clone)]
pub struct Extensions {
    runtime: Runtime,
    services: Arc<Services>,
    folder: InstallFolder,
    remote: Remote,
    admission: Arc<Admission>,
    roster: Arc<Roster>,
    changing: Arc<Mutex<()>>,
}

impl Extensions {
    pub fn open(
        data: &Path,
        state: &Path,
        runtime: Runtime,
        remote: Remote,
    ) -> Result<Self, ExtensionError> {
        let shelf = Shelf::open(state)?;
        let folder = InstallFolder::open(data.join(FOLDER))?;

        Ok(Self {
            runtime,
            services: Arc::new(Services::new(shelf, Arc::new(SystemClock))),
            folder,
            remote,
            admission: Arc::default(),
            roster: Arc::default(),
            changing: Arc::default(),
        })
    }

    #[must_use]
    pub fn with_secrets(self, secrets: Arc<dyn SecretSource>) -> Self {
        self.services.give_secrets(secrets);
        self
    }

    #[must_use]
    pub fn with_projects(self, folders: Arc<dyn ProjectFolders>) -> Self {
        self.services.give_folders(folders);
        self
    }

    pub fn connect(&self, caller: Arc<dyn Caller>, screen: Arc<dyn Screen>) {
        self.services.connect(CoreLink::new(caller, screen));
        self.roster.wake_up();
    }

    pub fn register(&self, registry: &mut Registry) -> Result<(), ActionError> {
        Registration::register(self, registry)?;

        self.admission.learn(registry);
        registry.attach(Arc::new(self.clone()));

        Ok(())
    }

    pub fn load_enabled(&self) -> Result<usize, ExtensionError> {
        let _changing = self.begin_change();
        let enabled = self
            .inventory()
            .all()?
            .into_iter()
            .filter(Installed::enabled);

        let mut loaded = 0;

        for entry in enabled {
            let id = entry.id().clone();

            match self.activate(entry) {
                Ok(()) => loaded += 1,
                Err(error) => self.roster.note_problem(id.as_str(), &error),
            }
        }

        Ok(loaded)
    }

    pub fn publish(&self, event: &ExtensionEvent) -> usize {
        self.roster.publish(event)
    }

    pub fn merge_document(
        &self,
        extension: &str,
        suffix: &str,
        sides: MergeSides<'_>,
    ) -> Result<Vec<u8>, ExtensionError> {
        let guest = Arc::clone(self.loaded(extension)?.guest());
        let kind = guest.manifest().document_kind(suffix).ok_or_else(|| {
            ExtensionError::NoDocumentType {
                extension: extension.to_owned(),
                suffix: suffix.to_owned(),
            }
        })?;

        guest.merge(&kind, sides)
    }

    pub(crate) fn loaded(&self, id: &str) -> Result<Arc<Loaded>, ExtensionError> {
        self.roster
            .get(id)
            .ok_or_else(|| ExtensionError::NotInstalled(id.to_owned()))
    }

    pub(crate) fn all_loaded(&self) -> Vec<Arc<Loaded>> {
        self.roster.all()
    }

    pub(crate) fn services(&self) -> &Services {
        &self.services
    }

    pub(crate) fn search(&self, query: Option<&str>) -> Result<Vec<SearchHit>, ExtensionError> {
        let index = self.fresh_index()?;
        let installed: BTreeMap<String, Installed> = self
            .inventory()
            .all()?
            .into_iter()
            .map(|entry| (entry.id().to_string(), entry))
            .collect();

        let hits = index
            .listings()
            .iter()
            .filter(|listing| query.is_none_or(|query| listing.matches(query)))
            .map(|listing| {
                let id = listing.manifest().id().as_str();
                SearchHit::new(listing, installed.get(id))
            })
            .collect();

        Ok(hits)
    }

    pub(crate) fn examine_listed(&self, id: &str) -> Result<Candidate, ExtensionError> {
        let index = self.fresh_index()?;
        let listing = index
            .find(id)
            .ok_or_else(|| ExtensionError::NotListed(id.to_owned()))?;

        if let Some(current) = self.inventory().find(id)? {
            current.check_not_newer_than(listing.manifest().version())?;
        }

        let package = self.remote.download(listing)?;

        self.examine(package)
    }

    pub(crate) fn examine_local(&self, folder: &Path) -> Result<Candidate, ExtensionError> {
        let package = LocalBuild::new(folder).read()?;

        self.examine(package)
    }

    pub(crate) fn install(&self, candidate: Candidate) -> Result<ExtensionInfo, ExtensionError> {
        let _changing = self.begin_change();
        let package = candidate.package();
        let current = self.inventory().find(package.manifest().id().as_str())?;

        if package.signed()
            && let Some(current) = &current
        {
            current.check_not_newer_than(package.manifest().version())?;
        }

        let clear_items = current.is_some_and(|current| current.signed() != package.signed());
        let manifest_sha256 = self.folder.store(package)?;
        let installed = Installed::new(package, manifest_sha256, self.services.now());

        self.inventory().record(&installed, clear_items)?;

        let loaded = Arc::new(candidate.into_loaded(installed));
        let info = ExtensionInfo::loaded(&loaded);

        self.roster.put(loaded);

        Ok(info)
    }

    pub(crate) fn list(&self) -> Result<Vec<ExtensionInfo>, ExtensionError> {
        let installed = self.inventory().all()?;

        Ok(installed.iter().map(|entry| self.info(entry)).collect())
    }

    pub(crate) fn enable(&self, id: &str) -> Result<ExtensionInfo, ExtensionError> {
        let _changing = self.begin_change();
        let entry = self.installed(id)?.enabled_as(true);

        self.inventory().set_enabled(id, true)?;

        if let Err(error) = self.activate(entry.clone()) {
            self.roster.note_problem(id, &error);
        }

        Ok(self.info(&entry))
    }

    pub(crate) fn disable(&self, id: &str) -> Result<ExtensionInfo, ExtensionError> {
        let _changing = self.begin_change();
        let entry = self.installed(id)?.enabled_as(false);

        self.inventory().set_enabled(id, false)?;
        self.roster.take(id);

        Ok(self.info(&entry))
    }

    pub(crate) fn remove(&self, id: &str) -> Result<ExtensionInfo, ExtensionError> {
        let _changing = self.begin_change();
        let info = self.info(&self.installed(id)?);

        self.roster.take(id);
        self.folder.remove(id)?;
        self.inventory().forget(id)?;

        Ok(info)
    }

    fn fresh_index(&self) -> Result<Index, ExtensionError> {
        let index = self.remote.index(self.services.now())?;

        self.services.shelf().index().accept(index.serial())?;

        Ok(index)
    }

    fn examine(&self, package: Package) -> Result<Candidate, ExtensionError> {
        let ceiling = self.admit(package.manifest())?;
        let replaces = self.inventory().find(package.manifest().id().as_str())?;

        let guest = self.compile_guest(&package)?;
        let specs = Self::describe(&guest, ceiling)?;

        Ok(Candidate::new(package, guest, specs, ceiling, replaces))
    }

    fn admit(&self, manifest: &Manifest) -> Result<Risk, ExtensionError> {
        self.admission.check_id(manifest.id())?;

        self.admission.ceiling(manifest)
    }

    fn compile_guest(&self, package: &Package) -> Result<Arc<Guest>, ExtensionError> {
        let manifest = package.manifest().clone();
        let compiled = self.runtime.compile(manifest.api(), package.wasm())?;

        Ok(Arc::new(Guest::new(
            manifest,
            compiled,
            self.runtime.clone(),
            Arc::clone(&self.services),
        )))
    }

    fn describe(guest: &Arc<Guest>, ceiling: Risk) -> Result<Vec<ActionSpec>, ExtensionError> {
        let specs = guest
            .describe()?
            .into_iter()
            .map(|spec| spec.at_least(ceiling))
            .collect();

        guest.check_contributed_actions()?;

        Ok(specs)
    }

    fn activate(&self, entry: Installed) -> Result<(), ExtensionError> {
        let package = self.folder.read(entry.id().as_str())?.package_for(&entry)?;
        let candidate = self.examine(package)?;

        self.roster.put(Arc::new(candidate.into_loaded(entry)));

        Ok(())
    }

    fn info(&self, entry: &Installed) -> ExtensionInfo {
        let id = entry.id().as_str();

        if let Some(loaded) = self.roster.get(id) {
            return ExtensionInfo::loaded(&loaded);
        }

        let problem = self.roster.problem(id);
        let manifest = self.folder.read(id).ok().map(InstalledFiles::into_manifest);

        ExtensionInfo::resting(entry, manifest, problem)
    }

    fn installed(&self, id: &str) -> Result<Installed, ExtensionError> {
        self.inventory()
            .find(id)?
            .ok_or_else(|| ExtensionError::NotInstalled(id.to_owned()))
    }

    fn inventory(&self) -> &Inventory {
        self.services.shelf().inventory()
    }

    fn begin_change(&self) -> MutexGuard<'_, ()> {
        self.changing.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Catalog for Extensions {
    fn get(&self, id: &ActionId) -> Option<Handle> {
        self.roster.handle(id)
    }

    fn describe(&self) -> Vec<ActionDescription> {
        self.roster.describe()
    }
}
