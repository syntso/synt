use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use synt_actions::{ActionError, Catalog, Description, Handle, Registry};
use synt_journal::ActionId;
use tokio::runtime::Handle as RuntimeHandle;

use crate::ExtensionError;
use crate::background::ExtensionEvent;
use crate::builtin::{self, ExtensionInfo, SearchHit};
use crate::call::Caller;
use crate::clock::now;
use crate::documents::ProjectFolders;
use crate::extension::{Candidate, Guest, Loaded, Roster, Services};
use crate::http::SecretSource;
use crate::install::{Admission, InstallFolder, LocalBuild, Package};
use crate::remote::{Index, Remote};
use crate::runtime::Runtime;
use crate::shelf::{Installed, Shelf};
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
        Ok(Self {
            runtime,
            services: Arc::new(Services::new(Shelf::open(state)?)),
            folder: InstallFolder::open(data.join(FOLDER))?,
            remote,
            admission: Arc::default(),
            roster: Arc::default(),
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

        self.roster.wake_up();
    }

    pub fn register(&self, registry: &mut Registry) -> Result<(), ActionError> {
        builtin::register(self, registry)?;

        self.admission.learn(registry);
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
                Err(error) => self.roster.note_problem(&id, &error),
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
        sides: (Option<&[u8]>, &[u8], &[u8]),
    ) -> Result<Vec<u8>, ExtensionError> {
        let guest = Arc::clone(&self.loaded(extension)?.guest);
        let kind = guest.manifest().document_kind(suffix).ok_or_else(|| {
            ExtensionError::Refused(format!(
                "{extension} owns no document type with the suffix .{suffix}"
            ))
        })?;

        guest
            .merge(&kind, sides)?
            .map_err(|error| ExtensionError::Refused(format!("the merge failed: {error}")))
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

        if let Some(current) = self.shelf().find(id)? {
            current.check_not_newer_than(&listing.version)?;
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
        let Candidate {
            package,
            guest,
            specs,
            ceiling,
            ..
        } = candidate;

        let current = self.shelf().find(&package.manifest.id)?;
        if package.signed
            && let Some(current) = &current
        {
            current.check_not_newer_than(&package.manifest.version)?;
        }
        let clear_items = current.is_some_and(|current| current.signed != package.signed);

        let installed = Installed {
            id: package.manifest.id.clone(),
            version: package.manifest.version.clone(),
            sha256: package.sha256.clone(),
            manifest_sha256: self.folder.store(&package)?,
            signed: package.signed,
            enabled: true,
            added_at: now(),
        };
        self.shelf().record(&installed, clear_items)?;

        let loaded = Arc::new(Loaded::new(guest, specs, ceiling, installed));
        let info = ExtensionInfo::loaded(&loaded);
        self.roster.put(loaded);

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
            self.roster.note_problem(id, &error);
        }

        Ok(self.info(entry))
    }

    pub(crate) fn disable(&self, id: &str) -> Result<ExtensionInfo, ExtensionError> {
        let _changing = self.begin_change();
        let mut entry = self.installed(id)?;

        entry.enabled = false;
        self.shelf().set_enabled(id, false)?;
        self.roster.take(id);

        Ok(self.info(entry))
    }

    pub(crate) fn remove(&self, id: &str) -> Result<ExtensionInfo, ExtensionError> {
        let _changing = self.begin_change();
        let entry = self.installed(id)?;
        let info = self.info(entry);

        self.roster.take(id);
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
        self.admission.check_id(&manifest.id)?;
        let ceiling = self.admission.ceiling(&manifest)?;
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
        guest.check_contributed_actions()?;

        Ok(Candidate {
            package,
            guest,
            specs,
            ceiling,
            replaces,
        })
    }

    fn activate(&self, entry: Installed) -> Result<(), ExtensionError> {
        let package = self.folder.read(&entry.id)?.package_for(&entry)?;
        let candidate = self.examine(package)?;

        let loaded = Loaded::new(candidate.guest, candidate.specs, candidate.ceiling, entry);
        self.roster.put(Arc::new(loaded));

        Ok(())
    }

    fn info(&self, entry: Installed) -> ExtensionInfo {
        if let Some(loaded) = self.roster.get(&entry.id) {
            return ExtensionInfo::loaded(&loaded);
        }

        let problem = self.roster.problem(&entry.id);
        let manifest = self.folder.read(&entry.id).map(|files| files.manifest);

        ExtensionInfo::resting(entry, manifest.ok(), problem)
    }

    fn installed(&self, id: &str) -> Result<Installed, ExtensionError> {
        self.shelf()
            .find(id)?
            .ok_or_else(|| ExtensionError::NotInstalled(id.to_owned()))
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
        self.roster.handle(id)
    }

    fn describe(&self) -> Vec<Description> {
        self.roster.describe()
    }
}
