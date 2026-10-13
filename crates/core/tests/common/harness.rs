use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use synt_actions::Registry;
use synt_client::Client;
use synt_core::{Core, Ping, ReadOnlyGate};
use synt_journal::{Event, Filter, Journal, JournalWriter, MemoryKeyStore, Seq};
use synt_platform::Endpoint;
use synt_proto::{ClientKind, Hello, RequestSeq};
use tempfile::TempDir;

use super::{Failing, Panicking, RunningServer, Touch, app_hello};

const PAGE: u32 = 100_000;

pub struct Harness {
    directory: TempDir,
    endpoint: Endpoint,
    journal: JournalWriter,
    touches: Arc<AtomicUsize>,
    server: Option<RunningServer>,
}

impl Harness {
    pub fn start() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let endpoint = endpoint(&directory);
        let journal = journal(&directory);
        let touches = Arc::new(AtomicUsize::new(0));

        let core = Core::new(registry(&touches), journal.clone(), ReadOnlyGate);
        let server = RunningServer::spawn(&endpoint, core).unwrap();

        Self {
            directory,
            endpoint,
            journal,
            touches,
            server: Some(server),
        }
    }

    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    pub fn journal(&self) -> JournalWriter {
        self.journal.clone()
    }

    pub fn touches(&self) -> usize {
        self.touches.load(Ordering::SeqCst)
    }

    pub async fn app(&self) -> Client {
        Client::connect(&self.endpoint, app_hello()).await.unwrap()
    }

    pub async fn agent(&self, name: &str) -> Client {
        let kind = ClientKind::Agent {
            name: name.to_owned(),
        };

        Client::connect(&self.endpoint, Hello::new(kind, "synt-tests"))
            .await
            .unwrap()
    }

    pub fn events(&self) -> Vec<Event> {
        self.matching(&Filter::all())
    }

    pub fn matching(&self, filter: &Filter) -> Vec<Event> {
        let reader = self.journal.reader().unwrap();
        reader.page(filter, None, PAGE).unwrap()
    }

    pub fn trail(&self, request: RequestSeq) -> Vec<Event> {
        self.matching(&Filter::all().call(Seq::new(request.0)))
    }

    pub fn is_intact(&self) -> bool {
        let reader = self.journal.reader().unwrap();
        reader.verify().unwrap().is_intact()
    }

    pub async fn stop(&mut self) {
        if let Some(server) = self.server.take() {
            server.stop().await;
        }
    }
}

fn endpoint(directory: &TempDir) -> Endpoint {
    let unique = directory.path().file_name().unwrap().to_string_lossy();
    Endpoint::named(directory.path(), &format!("synt-test-{unique}"))
}

fn journal(directory: &TempDir) -> JournalWriter {
    let keys = Arc::new(MemoryKeyStore::new().unwrap());
    let journal = Journal::create(directory.path().join("journal.sqlite"), keys).unwrap();

    JournalWriter::spawn(journal).unwrap()
}

fn registry(touches: &Arc<AtomicUsize>) -> Registry {
    let mut registry = Registry::new();

    registry.register(Ping).unwrap();
    registry.register(Touch::new(Arc::clone(touches))).unwrap();
    registry.register(Failing).unwrap();
    registry.register(Panicking).unwrap();

    registry
}
