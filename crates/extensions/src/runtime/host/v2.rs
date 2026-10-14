use super::Sandbox;
use crate::bindings::v2::synt::extension::types::{Error, Level};
use crate::bindings::v2::synt::extension::{
    documents, files, host, http, storage, timers, types, ui,
};
use crate::documents::{Document, DocumentInfo, Written};
use crate::http::{Header, HttpRequest, HttpResponse, SecretHeader};
use crate::interface::{self, GuestError};
use crate::manifest::FileAccess;
use crate::picked::Picked;

impl types::Host for Sandbox {}

impl host::Host for Sandbox {
    fn call(
        &mut self,
        action: String,
        target: Option<String>,
        params: String,
    ) -> wasmtime::Result<Result<String, Error>> {
        self.reach_out(|imports| imports.call(action, target, params).map_err(Error::from))
    }

    fn log(&mut self, level: Level, message: String) -> wasmtime::Result<()> {
        self.quietly(|imports| imports.log(level.into(), message));

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
        self.keep(|imports| imports.storage_set(key, value).map_err(Error::from))
    }
}

impl http::Host for Sandbox {
    fn send(&mut self, request: http::Request) -> wasmtime::Result<Result<http::Response, Error>> {
        let request = HttpRequest::from(request);

        self.reach_out(|imports| {
            imports
                .send_http(request)
                .and_then(http::Response::try_from)
                .map_err(Error::from)
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
                .map(|found| {
                    found
                        .into_iter()
                        .map(documents::DocumentInfo::from)
                        .collect()
                })
                .map_err(Error::from)
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
                .map(documents::Document::from)
                .map_err(Error::from)
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
                .map(documents::Written::from)
                .map_err(Error::from)
        })
    }

    fn remove(&mut self, project: String, path: String) -> wasmtime::Result<Result<(), Error>> {
        self.keep(|imports| imports.remove_document(project, path).map_err(Error::from))
    }
}

impl files::Host for Sandbox {
    fn chosen(&mut self) -> wasmtime::Result<Vec<files::Picked>> {
        self.keep(|imports| {
            imports
                .chosen_files()
                .into_iter()
                .map(files::Picked::from)
                .collect()
        })
    }

    fn read(&mut self, handle: String) -> wasmtime::Result<Result<Vec<u8>, Error>> {
        self.keep(|imports| imports.read_file(handle).map_err(Error::from))
    }

    fn write(&mut self, handle: String, body: Vec<u8>) -> wasmtime::Result<Result<(), Error>> {
        self.keep(|imports| imports.write_file(handle, body).map_err(Error::from))
    }
}

impl timers::Host for Sandbox {
    fn set(&mut self, name: String, every_seconds: u32) -> wasmtime::Result<Result<(), Error>> {
        self.keep(|imports| imports.set_timer(name, every_seconds).map_err(Error::from))
    }

    fn clear(&mut self, name: String) -> wasmtime::Result<()> {
        self.keep(|imports| imports.clear_timer(name))
    }
}

impl ui::Host for Sandbox {
    fn patch_view(&mut self, view: String, patch: String) -> wasmtime::Result<Result<(), Error>> {
        self.tell(|imports| imports.patch_view(view, patch).map_err(Error::from))
    }

    fn post(&mut self, panel: String, message: String) -> wasmtime::Result<Result<(), Error>> {
        self.tell(|imports| imports.post(panel, message).map_err(Error::from))
    }
}

impl From<http::Request> for HttpRequest {
    fn from(request: http::Request) -> Self {
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

        Self {
            headers,
            secrets,
            timeout_ms: request.timeout_ms,
            ..Self::new(request.method, request.url).body_bytes(request.body.as_deref())
        }
    }
}

impl TryFrom<HttpResponse> for http::Response {
    type Error = GuestError;

    fn try_from(response: HttpResponse) -> Result<Self, GuestError> {
        let body = response.decoded_body().map_err(GuestError::failed)?;
        let headers = response
            .headers
            .into_iter()
            .map(|header| http::Header {
                name: header.name,
                value: header.value,
            })
            .collect();

        Ok(Self {
            status: response.status,
            headers,
            body,
        })
    }
}

impl From<DocumentInfo> for documents::DocumentInfo {
    fn from(info: DocumentInfo) -> Self {
        Self {
            project: info.project,
            path: info.path,
            kind: info.kind,
            revision: info.revision.into(),
            size: info.size,
        }
    }
}

impl From<Document> for documents::Document {
    fn from(found: Document) -> Self {
        Self {
            info: found.info.into(),
            body: found.body,
        }
    }
}

impl From<Written> for documents::Written {
    fn from(written: Written) -> Self {
        Self {
            info: written.info.into(),
            merged: written.merged,
        }
    }
}

impl From<Picked> for files::Picked {
    fn from(picked: Picked) -> Self {
        let access = match picked.access {
            FileAccess::Read => files::Access::Read,
            FileAccess::ReadWrite => files::Access::ReadWrite,
        };

        Self {
            handle: picked.handle,
            name: picked.name,
            access,
        }
    }
}

impl From<GuestError> for Error {
    fn from(error: GuestError) -> Self {
        Self {
            code: error.code().to_string(),
            message: error.message().to_owned(),
        }
    }
}

impl From<Level> for interface::Level {
    fn from(level: Level) -> Self {
        match level {
            Level::Trace => Self::Trace,
            Level::Debug => Self::Debug,
            Level::Info => Self::Info,
            Level::Warn => Self::Warn,
            Level::Error => Self::Error,
        }
    }
}
