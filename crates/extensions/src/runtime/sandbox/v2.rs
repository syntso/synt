use super::Sandbox;
use crate::bindings::v2::synt::extension::types::{Error, Level};
use crate::bindings::v2::synt::extension::{
    documents, files, host, http, storage, timers, types, ui,
};
use crate::documents::{Document, DocumentInfo, Written};
use crate::files::Picked;
use crate::http::{Header, HttpRequest, HttpResponse, SecretHeader};
use crate::interface::{self, GuestError};
use crate::manifest::FileAccess;

impl types::Host for Sandbox {}

impl host::Host for Sandbox {
    fn call(
        &mut self,
        action: String,
        target: Option<String>,
        params: String,
    ) -> wasmtime::Result<Result<String, Error>> {
        self.reach_out(|imports| imports.call(action, target, params).map_err(error))
    }

    fn log(&mut self, level: Level, message: String) -> wasmtime::Result<()> {
        self.quietly(|imports| imports.log(level_of(level), message));

        Ok(())
    }

    fn settings(&mut self) -> wasmtime::Result<String> {
        self.keep(|imports| imports.settings())
    }
}

impl storage::Host for Sandbox {
    fn get(&mut self, key: String) -> wasmtime::Result<Option<Vec<u8>>> {
        self.keep(|imports| imports.storage_get(key))
    }

    fn set(&mut self, key: String, value: Vec<u8>) -> wasmtime::Result<Result<(), Error>> {
        self.keep(|imports| imports.storage_set(key, value).map_err(error))
    }
}

impl http::Host for Sandbox {
    fn send(&mut self, request: http::Request) -> wasmtime::Result<Result<http::Response, Error>> {
        let request = http_request(request);

        self.reach_out(|imports| {
            imports
                .send_http(request)
                .and_then(http_response)
                .map_err(error)
        })
    }
}

impl documents::Host for Sandbox {
    fn listing(
        &mut self,
        project: String,
    ) -> wasmtime::Result<Result<Vec<documents::DocumentInfo>, Error>> {
        self.keep(|imports| {
            imports
                .documents(project)
                .map(|found| found.into_iter().map(document_info).collect())
                .map_err(error)
        })
    }

    fn read(
        &mut self,
        project: String,
        path: String,
    ) -> wasmtime::Result<Result<documents::Document, Error>> {
        self.keep(|imports| {
            imports
                .read_document(project, path)
                .map(document)
                .map_err(error)
        })
    }

    fn write(
        &mut self,
        project: String,
        path: String,
        body: Vec<u8>,
        base: Option<String>,
    ) -> wasmtime::Result<Result<documents::Written, Error>> {
        self.keep(|imports| {
            imports
                .write_document(project, path, body, base)
                .map(written)
                .map_err(error)
        })
    }

    fn remove(&mut self, project: String, path: String) -> wasmtime::Result<Result<(), Error>> {
        self.keep(|imports| imports.remove_document(project, path).map_err(error))
    }
}

impl files::Host for Sandbox {
    fn chosen(&mut self) -> wasmtime::Result<Vec<files::Picked>> {
        self.keep(|imports| imports.chosen_files().into_iter().map(picked).collect())
    }

    fn read(&mut self, handle: String) -> wasmtime::Result<Result<Vec<u8>, Error>> {
        self.keep(|imports| imports.read_file(handle).map_err(error))
    }

    fn write(&mut self, handle: String, body: Vec<u8>) -> wasmtime::Result<Result<(), Error>> {
        self.keep(|imports| imports.write_file(handle, body).map_err(error))
    }
}

impl timers::Host for Sandbox {
    fn set(&mut self, name: String, every_seconds: u32) -> wasmtime::Result<Result<(), Error>> {
        self.keep(|imports| imports.set_timer(name, every_seconds).map_err(error))
    }

    fn clear(&mut self, name: String) -> wasmtime::Result<()> {
        self.keep(|imports| imports.clear_timer(name))
    }
}

impl ui::Host for Sandbox {
    fn patch_view(&mut self, view: String, patch: String) -> wasmtime::Result<Result<(), Error>> {
        self.tell(|imports| imports.patch_view(view, patch).map_err(error))
    }

    fn post(&mut self, panel: String, message: String) -> wasmtime::Result<Result<(), Error>> {
        self.tell(|imports| imports.post(panel, message).map_err(error))
    }
}

fn http_request(request: http::Request) -> HttpRequest {
    let headers = request
        .headers
        .into_iter()
        .map(|header| Header {
            name: header.name,
            value: header.value,
        })
        .collect();
    let secrets = request
        .secrets
        .into_iter()
        .map(|secret| SecretHeader {
            name: secret.name,
            secret: secret.secret,
            prefix: secret.prefix,
        })
        .collect();

    HttpRequest {
        headers,
        secrets,
        timeout_ms: request.timeout_ms,
        ..HttpRequest::new(request.method, request.url).body_bytes(request.body.as_deref())
    }
}

fn http_response(response: HttpResponse) -> Result<http::Response, GuestError> {
    let body = response
        .decoded_body()
        .map_err(|reason| GuestError::new("failed", reason))?;
    let headers = response
        .headers
        .into_iter()
        .map(|header| http::Header {
            name: header.name,
            value: header.value,
        })
        .collect();

    Ok(http::Response {
        status: response.status,
        headers,
        body,
    })
}

fn document_info(info: DocumentInfo) -> documents::DocumentInfo {
    documents::DocumentInfo {
        project: info.project,
        path: info.path,
        kind: info.kind,
        revision: info.revision,
        size: info.size,
    }
}

fn document(found: Document) -> documents::Document {
    documents::Document {
        info: document_info(found.info),
        body: found.body,
    }
}

fn written(written: Written) -> documents::Written {
    documents::Written {
        info: document_info(written.info),
        merged: written.merged,
    }
}

fn picked(picked: Picked) -> files::Picked {
    let access = match picked.access {
        FileAccess::Read => files::Access::Read,
        FileAccess::ReadWrite => files::Access::ReadWrite,
    };

    files::Picked {
        handle: picked.handle,
        name: picked.name,
        access,
    }
}

fn error(error: GuestError) -> Error {
    Error {
        code: error.code,
        message: error.message,
    }
}

fn level_of(level: Level) -> interface::Level {
    match level {
        Level::Trace => interface::Level::Trace,
        Level::Debug => interface::Level::Debug,
        Level::Info => interface::Level::Info,
        Level::Warn => interface::Level::Warn,
        Level::Error => interface::Level::Error,
    }
}
