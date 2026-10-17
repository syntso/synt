use super::Sandbox;
use crate::bindings::v2::synt::extension::types::{Error, Level};
use crate::bindings::v2::synt::extension::{
    documents, files, host, http, storage, timers, types, ui,
};
use crate::documents::{DocumentInfo, Opened, Written};
use crate::files::{FileAccess, Picked};
use crate::http::{Header, HttpRequest, HttpResponse, SecretHeader};
use crate::interface::GuestError;
use crate::runtime::Spend;

impl types::Host for Sandbox {}

impl host::Host for Sandbox {
    fn call(
        &mut self,
        action: String,
        target: Option<String>,
        params: String,
    ) -> wasmtime::Result<Result<String, Error>> {
        self.spend(Spend::Call, |imports| {
            imports.call(action, target, params).map_err(Error::from)
        })
    }

    fn log(&mut self, level: Level, message: String) -> wasmtime::Result<()> {
        self.spend(Spend::Free, |imports| imports.log(level.into(), message))
    }

    fn settings(&mut self) -> wasmtime::Result<String> {
        self.spend(Spend::Storage, |imports| imports.settings())
    }
}

impl storage::Host for Sandbox {
    fn get(&mut self, key: String) -> wasmtime::Result<Option<Vec<u8>>> {
        self.spend(Spend::Storage, |imports| imports.storage_get(key))
    }

    fn set(&mut self, key: String, value: Vec<u8>) -> wasmtime::Result<Result<(), Error>> {
        self.spend(Spend::Storage, |imports| {
            imports.storage_set(key, value).map_err(Error::from)
        })
    }
}

impl http::Host for Sandbox {
    fn send(&mut self, request: http::Request) -> wasmtime::Result<Result<http::Response, Error>> {
        let request = HttpRequest::from(request);

        self.spend(Spend::Call, |imports| {
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
        self.spend(Spend::Storage, |imports| {
            let found = imports.documents(project).map_err(Error::from)?;

            Ok(found.iter().map(documents::DocumentInfo::from).collect())
        })
    }

    fn read(
        &mut self,
        project: String,
        path: String,
    ) -> wasmtime::Result<Result<documents::Document, Error>> {
        self.spend(Spend::Storage, |imports| {
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
        self.spend(Spend::Storage, |imports| {
            imports
                .write_document(project, path, body, base)
                .map(documents::Written::from)
                .map_err(Error::from)
        })
    }

    fn remove(&mut self, project: String, path: String) -> wasmtime::Result<Result<(), Error>> {
        self.spend(Spend::Storage, |imports| {
            imports.remove_document(project, path).map_err(Error::from)
        })
    }
}

impl files::Host for Sandbox {
    fn chosen(&mut self) -> wasmtime::Result<Vec<files::Picked>> {
        self.spend(Spend::Storage, |imports| {
            imports
                .chosen_files()
                .into_iter()
                .map(files::Picked::from)
                .collect()
        })
    }

    fn read(&mut self, handle: String) -> wasmtime::Result<Result<Vec<u8>, Error>> {
        self.spend(Spend::Storage, |imports| {
            imports.read_file(handle).map_err(Error::from)
        })
    }

    fn write(&mut self, handle: String, body: Vec<u8>) -> wasmtime::Result<Result<(), Error>> {
        self.spend(Spend::Storage, |imports| {
            imports.write_file(handle, body).map_err(Error::from)
        })
    }
}

impl timers::Host for Sandbox {
    fn set(&mut self, name: String, every_seconds: u32) -> wasmtime::Result<Result<(), Error>> {
        self.spend(Spend::Storage, |imports| {
            imports.set_timer(name, every_seconds).map_err(Error::from)
        })
    }

    fn clear(&mut self, name: String) -> wasmtime::Result<()> {
        self.spend(Spend::Storage, |imports| imports.clear_timer(name))
    }
}

impl ui::Host for Sandbox {
    fn patch_view(&mut self, view: String, patch: String) -> wasmtime::Result<Result<(), Error>> {
        self.spend(Spend::Message, |imports| {
            imports.patch_view(view, patch).map_err(Error::from)
        })
    }

    fn post(&mut self, panel: String, message: String) -> wasmtime::Result<Result<(), Error>> {
        self.spend(Spend::Message, |imports| {
            imports.post(panel, message).map_err(Error::from)
        })
    }
}

impl From<http::Request> for HttpRequest {
    fn from(request: http::Request) -> Self {
        let headers = request
            .headers
            .into_iter()
            .map(|header| Header::new(header.name, header.value))
            .collect();
        let secrets = request
            .secrets
            .into_iter()
            .map(|secret| SecretHeader::new(secret.name, secret.secret, secret.prefix))
            .collect();

        Self::new(request.method, request.url)
            .with_headers(headers, secrets)
            .with_body(request.body.as_deref())
            .with_timeout_ms(request.timeout_ms)
    }
}

impl TryFrom<HttpResponse> for http::Response {
    type Error = GuestError;

    fn try_from(response: HttpResponse) -> Result<Self, GuestError> {
        let body = response.decoded_body()?;
        let headers = response
            .headers()
            .iter()
            .map(|header| http::Header {
                name: header.name().to_owned(),
                value: header.value().to_owned(),
            })
            .collect();

        Ok(Self {
            status: response.status(),
            headers,
            body,
        })
    }
}

impl From<&DocumentInfo> for documents::DocumentInfo {
    fn from(info: &DocumentInfo) -> Self {
        Self {
            project: info.project().to_owned(),
            path: info.path().to_owned(),
            kind: info.kind().to_owned(),
            revision: info.revision().to_string(),
            size: info.size(),
        }
    }
}

impl From<Opened> for documents::Document {
    fn from(opened: Opened) -> Self {
        Self {
            info: opened.info().into(),
            body: opened.into_body(),
        }
    }
}

impl From<Written> for documents::Written {
    fn from(written: Written) -> Self {
        Self {
            info: written.info().into(),
            merged: written.merged(),
        }
    }
}

impl From<Picked> for files::Picked {
    fn from(picked: Picked) -> Self {
        let access = match picked.access() {
            FileAccess::Read => files::Access::Read,
            FileAccess::ReadWrite => files::Access::ReadWrite,
        };

        Self {
            handle: picked.handle().to_string(),
            name: picked.name().to_owned(),
            access,
        }
    }
}
