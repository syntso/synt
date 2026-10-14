use std::sync::Arc;

use synt_client::{Autostart, Client};
use synt_core::{Broker, Core};
use synt_journal::{Event, Filter, JournalWriter, Seq};
use synt_platform::Endpoint;
use synt_proto::{ClientKind, Hello, RequestSeq};

use super::{Approver, Grounds, RunningServer, app_hello};

const PAGE: u32 = 100_000;

pub struct Harness {
    grounds: Grounds,
    endpoint: Endpoint,
    server: Option<RunningServer>,
}

impl Harness {
    pub fn start() -> Self {
        Self::with_broker(Broker::new())
    }

    pub fn with_broker(broker: Broker) -> Self {
        let grounds = Grounds::new();
        let core = Core::new(grounds.registry(), grounds.journal(), broker);

        Self::serve(grounds, core)
    }

    pub fn serve(grounds: Grounds, core: impl Into<Arc<Core>>) -> Self {
        let endpoint = grounds.endpoint();
        let server = RunningServer::spawn(&endpoint, core).unwrap();

        Self {
            grounds,
            endpoint,
            server: Some(server),
        }
    }

    pub async fn restart(mut self, core: impl FnOnce(&Grounds) -> Core) -> Self {
        self.stop().await;

        let core = core(&self.grounds);
        Self::serve(self.grounds, core)
    }

    pub fn grounds(&self) -> &Grounds {
        &self.grounds
    }

    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    pub fn journal(&self) -> JournalWriter {
        self.grounds.journal()
    }

    pub fn touches(&self) -> usize {
        self.grounds.touches()
    }

    pub fn runs(&self) -> usize {
        self.grounds.runs()
    }

    pub async fn app(&self) -> Client {
        Client::connect(&self.endpoint, app_hello(), &Autostart::off())
            .await
            .unwrap()
    }

    pub async fn approver(&self) -> Approver {
        Approver::new(self.app().await)
    }

    pub async fn agent(&self, name: &str) -> Client {
        let kind = ClientKind::Agent {
            name: name.to_owned(),
        };

        Client::connect(
            &self.endpoint,
            Hello::new(kind, "synt-tests"),
            &Autostart::off(),
        )
        .await
        .unwrap()
    }

    pub fn events(&self) -> Vec<Event> {
        self.matching(&Filter::all())
    }

    pub fn matching(&self, filter: &Filter) -> Vec<Event> {
        let reader = self.journal().reader().unwrap();
        reader.page(filter, None, PAGE).unwrap()
    }

    pub fn trail(&self, request: RequestSeq) -> Vec<Event> {
        self.matching(&Filter::all().call(Seq::new(request.0)))
    }

    pub fn is_intact(&self) -> bool {
        let reader = self.journal().reader().unwrap();
        reader.verify().unwrap().is_intact()
    }

    pub async fn stop(&mut self) {
        if let Some(server) = self.server.take() {
            server.stop().await;
        }
    }
}
