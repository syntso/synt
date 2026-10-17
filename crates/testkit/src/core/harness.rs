use std::sync::Arc;

use synt_client::{Autostart, Client, ClientError};
use synt_core::{Broker, Core};
use synt_journal::{Event, Filter, JournalError, JournalWriter, Seq};
use synt_platform::Endpoint;
use synt_proto::{ClientKind, Hello, RequestSeq};

use super::{Approver, Grounds, RunningServer, app_hello};
use crate::TestError;

const PAGE: u32 = 100_000;

pub struct Harness {
    grounds: Grounds,
    endpoint: Endpoint,
    server: Option<RunningServer>,
}

impl Harness {
    pub fn start() -> Result<Self, TestError> {
        Self::with_broker(Broker::new())
    }

    pub fn with_broker(broker: Broker) -> Result<Self, TestError> {
        let grounds = Grounds::new()?;
        let core = Core::new(grounds.registry()?, grounds.journal(), broker);

        Self::serve(grounds, core)
    }

    pub fn serve(grounds: Grounds, core: impl Into<Arc<Core>>) -> Result<Self, TestError> {
        let endpoint = grounds.endpoint();
        let server = RunningServer::spawn(&endpoint, core)?;

        Ok(Self {
            grounds,
            endpoint,
            server: Some(server),
        })
    }

    pub async fn restart(
        mut self,
        core: impl FnOnce(&Grounds) -> Result<Core, TestError>,
    ) -> Result<Self, TestError> {
        self.stop().await?;

        let core = core(&self.grounds)?;

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

    pub async fn app(&self) -> Result<Client, ClientError> {
        Client::connect(&self.endpoint, app_hello(), &Autostart::off()).await
    }

    pub async fn approver(&self) -> Result<Approver, ClientError> {
        Ok(Approver::new(self.app().await?))
    }

    pub async fn agent(&self, name: &str) -> Result<Client, ClientError> {
        let kind = ClientKind::Agent {
            name: name.to_owned(),
        };

        Client::connect(
            &self.endpoint,
            Hello::new(kind, "synt-tests"),
            &Autostart::off(),
        )
        .await
    }

    pub fn events(&self) -> Result<Vec<Event>, JournalError> {
        self.matching(&Filter::all())
    }

    pub fn matching(&self, filter: &Filter) -> Result<Vec<Event>, JournalError> {
        let reader = self.journal().reader()?;

        reader.page(filter, None, PAGE)
    }

    pub fn trail(&self, request: RequestSeq) -> Result<Vec<Event>, JournalError> {
        self.matching(&Filter::all().call(Seq::new(request.get())))
    }

    pub fn is_intact(&self) -> Result<bool, JournalError> {
        let reader = self.journal().reader()?;

        Ok(reader.verify()?.is_intact())
    }

    pub async fn stop(&mut self) -> Result<(), TestError> {
        if let Some(server) = self.server.take() {
            server.stop().await?;
        }

        Ok(())
    }
}
