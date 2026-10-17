mod core_calls;
mod document_access;
mod guest_log;
mod item_storage;
mod picked_files;
mod screen_access;
mod timer_access;

use std::sync::Arc;

use core_calls::CoreCalls;
use document_access::DocumentAccess;
use guest_log::GuestLog;
use item_storage::ItemStorage;
use picked_files::PickedFiles;
use screen_access::ScreenAccess;
use timer_access::TimerAccess;

use super::{Guest, Outside};
use crate::documents::{DocumentInfo, Opened, Written};
use crate::files::Picked;
use crate::http::{HttpRequest, HttpResponse};
use crate::interface::{GuestError, Level};
use crate::runtime::Imports;

pub(crate) struct Visit {
    calls: CoreCalls,
    documents: DocumentAccess,
    files: PickedFiles,
    storage: ItemStorage,
    timers: TimerAccess,
    screen: ScreenAccess,
    log: GuestLog,
}

impl Visit {
    pub(crate) fn new(guest: &Arc<Guest>, outside: Option<Outside>) -> Self {
        Self {
            calls: CoreCalls::new(Arc::clone(guest), outside),
            documents: DocumentAccess::new(Arc::clone(guest)),
            files: PickedFiles::new(Arc::clone(guest)),
            storage: ItemStorage::new(Arc::clone(guest)),
            timers: TimerAccess::new(Arc::clone(guest)),
            screen: ScreenAccess::new(Arc::clone(guest)),
            log: GuestLog::new(guest.id()),
        }
    }
}

impl Imports for Visit {
    fn call(
        &mut self,
        action: String,
        target: Option<String>,
        params: String,
    ) -> Result<String, GuestError> {
        self.calls.call(action, target, &params)
    }

    fn log(&mut self, level: Level, message: String) {
        self.log.write(level, &message);
    }

    fn settings(&mut self) -> String {
        self.storage.settings()
    }

    fn storage_get(&mut self, key: String) -> Option<Vec<u8>> {
        self.storage.get(&key)
    }

    fn storage_set(&mut self, key: String, value: Vec<u8>) -> Result<(), GuestError> {
        self.storage.set(&key, &value)
    }

    fn send_http(&mut self, request: HttpRequest) -> Result<HttpResponse, GuestError> {
        self.calls.send(&request)
    }

    fn documents(&mut self, project: String) -> Result<Vec<DocumentInfo>, GuestError> {
        self.documents.list(&project)
    }

    fn read_document(&mut self, project: String, path: String) -> Result<Opened, GuestError> {
        self.documents.read(&project, &path)
    }

    fn write_document(
        &mut self,
        project: String,
        path: String,
        body: Vec<u8>,
        base: Option<String>,
    ) -> Result<Written, GuestError> {
        self.documents
            .write(&project, &path, &body, base.as_deref())
    }

    fn remove_document(&mut self, project: String, path: String) -> Result<(), GuestError> {
        self.documents.remove(&project, &path)
    }

    fn chosen_files(&mut self) -> Vec<Picked> {
        self.files.chosen()
    }

    fn read_file(&mut self, handle: String) -> Result<Vec<u8>, GuestError> {
        self.files.read(&handle)
    }

    fn write_file(&mut self, handle: String, body: Vec<u8>) -> Result<(), GuestError> {
        self.files.write(&handle, &body)
    }

    fn set_timer(&mut self, name: String, every_seconds: u32) -> Result<(), GuestError> {
        self.timers.set(&name, every_seconds)
    }

    fn clear_timer(&mut self, name: String) {
        self.timers.clear(&name);
    }

    fn patch_view(&mut self, view: String, patch: String) -> Result<(), GuestError> {
        self.screen.patch_view(&view, &patch)
    }

    fn post(&mut self, panel: String, message: String) -> Result<(), GuestError> {
        self.screen.post(&panel, &message)
    }
}
