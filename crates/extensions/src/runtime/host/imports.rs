use crate::documents::{Document, DocumentInfo, Written};
use crate::http::{HttpRequest, HttpResponse};
use crate::interface::{GuestError, Level};
use crate::picked::Picked;

pub(crate) trait Imports: Send {
    fn call(
        &mut self,
        action: String,
        target: Option<String>,
        params: String,
    ) -> Result<String, GuestError>;

    fn log(&mut self, level: Level, message: String);

    fn settings(&mut self) -> String;

    fn storage_get(&mut self, key: String) -> Option<Vec<u8>>;

    fn storage_set(&mut self, key: String, value: Vec<u8>) -> Result<(), GuestError>;

    fn send_http(&mut self, request: HttpRequest) -> Result<HttpResponse, GuestError>;

    fn documents(&mut self, project: String) -> Result<Vec<DocumentInfo>, GuestError>;

    fn read_document(&mut self, project: String, path: String) -> Result<Document, GuestError>;

    fn write_document(
        &mut self,
        project: String,
        path: String,
        body: Vec<u8>,
        base: Option<String>,
    ) -> Result<Written, GuestError>;

    fn remove_document(&mut self, project: String, path: String) -> Result<(), GuestError>;

    fn chosen_files(&mut self) -> Vec<Picked>;

    fn read_file(&mut self, handle: String) -> Result<Vec<u8>, GuestError>;

    fn write_file(&mut self, handle: String, body: Vec<u8>) -> Result<(), GuestError>;

    fn set_timer(&mut self, name: String, every_seconds: u32) -> Result<(), GuestError>;

    fn clear_timer(&mut self, name: String);

    fn patch_view(&mut self, view: String, patch: String) -> Result<(), GuestError>;

    fn post(&mut self, panel: String, message: String) -> Result<(), GuestError>;
}
