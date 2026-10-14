use std::sync::Arc;

use super::log_allowance::LogAllowance;
use crate::documents::{Document, DocumentInfo, Written};
use crate::extension::{Guest, Outside};
use crate::http::{HttpRequest, HttpResponse};
use crate::interface::{GuestError, Level};
use crate::manifest::Manifest;
use crate::picked::Picked;
use crate::runtime::Imports;

pub(crate) struct Visit {
    pub(super) guest: Arc<Guest>,
    pub(super) outside: Option<Outside>,
    pub(super) logs: LogAllowance,
}

impl Visit {
    pub(crate) fn new(guest: Arc<Guest>, outside: Option<Outside>) -> Self {
        Self {
            guest,
            outside,
            logs: LogAllowance::new(),
        }
    }

    pub(super) fn id(&self) -> &str {
        &self.guest.manifest().id
    }

    pub(super) fn manifest(&self) -> &Manifest {
        self.guest.manifest()
    }

    pub(super) fn allowed(&self, granted: bool, missing: &str) -> Result<(), GuestError> {
        if granted {
            return Ok(());
        }

        Err(GuestError::missing_capability(self.id(), missing))
    }
}

impl Imports for Visit {
    fn call(
        &mut self,
        action: String,
        target: Option<String>,
        params: String,
    ) -> Result<String, GuestError> {
        self.call_core(action, target, &params)
    }

    fn log(&mut self, level: Level, message: String) {
        self.write_log(level, &message);
    }

    fn settings(&mut self) -> String {
        self.read_settings()
    }

    fn storage_get(&mut self, key: String) -> Option<Vec<u8>> {
        self.read_item(&key)
    }

    fn storage_set(&mut self, key: String, value: Vec<u8>) -> Result<(), GuestError> {
        self.write_item(&key, &value)
    }

    fn send_http(&mut self, request: HttpRequest) -> Result<HttpResponse, GuestError> {
        self.send(&request)
    }

    fn documents(&mut self, project: String) -> Result<Vec<DocumentInfo>, GuestError> {
        self.list_documents(&project)
    }

    fn read_document(&mut self, project: String, path: String) -> Result<Document, GuestError> {
        self.open_document(&project, &path)
    }

    fn write_document(
        &mut self,
        project: String,
        path: String,
        body: Vec<u8>,
        base: Option<String>,
    ) -> Result<Written, GuestError> {
        self.save_document(&project, &path, &body, base.as_deref())
    }

    fn remove_document(&mut self, project: String, path: String) -> Result<(), GuestError> {
        self.delete_document(&project, &path)
    }

    fn chosen_files(&mut self) -> Vec<Picked> {
        self.picked_files()
    }

    fn read_file(&mut self, handle: String) -> Result<Vec<u8>, GuestError> {
        self.read_picked(&handle)
    }

    fn write_file(&mut self, handle: String, body: Vec<u8>) -> Result<(), GuestError> {
        self.write_picked(&handle, &body)
    }

    fn set_timer(&mut self, name: String, every_seconds: u32) -> Result<(), GuestError> {
        self.arm_timer(&name, every_seconds)
    }

    fn clear_timer(&mut self, name: String) {
        self.disarm_timer(&name);
    }

    fn patch_view(&mut self, view: String, patch: String) -> Result<(), GuestError> {
        self.send_patch(&view, &patch)
    }

    fn post(&mut self, panel: String, message: String) -> Result<(), GuestError> {
        self.send_message(&panel, &message)
    }
}
