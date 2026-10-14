mod notice;
mod request;

pub use notice::{NoticeKind, WatchNotice};
pub use request::WatchRequest;

use std::collections::VecDeque;
use std::fmt::{self, Debug, Formatter};

use futures_util::StreamExt;
use futures_util::stream::BoxStream;
use kube::api::{Api, DynamicObject, ListParams, WatchEvent, WatchParams};

use crate::{Kind, KubeError};

const WATCH_SECONDS: u32 = 290;
const PAGE: u32 = 500;
const GONE: u16 = 410;

type Events = BoxStream<'static, Result<WatchEvent<DynamicObject>, kube::Error>>;

pub struct Watch {
    api: Api<DynamicObject>,
    kind: Kind,
    selector: Option<String>,
    version: Option<String>,
    events: Option<Events>,
    pending: VecDeque<WatchNotice>,
}

impl Watch {
    pub(crate) fn new(api: Api<DynamicObject>, kind: Kind, selector: Option<String>) -> Self {
        Self {
            api,
            kind,
            selector,
            version: None,
            events: None,
            pending: VecDeque::new(),
        }
    }

    pub async fn next(&mut self) -> Result<WatchNotice, KubeError> {
        loop {
            if let Some(notice) = self.pending.pop_front() {
                return Ok(notice);
            }

            if self.version.is_none() {
                self.relist().await?;
                continue;
            }

            if self.events.is_none() {
                self.events = Some(self.open().await?);
            }

            self.take_event().await?;
        }
    }

    async fn relist(&mut self) -> Result<(), KubeError> {
        let mut continue_token: Option<String> = None;

        loop {
            let mut params = self.list_params();
            if let Some(token) = &continue_token {
                params = params.continue_token(token);
            }

            let page = self.api.list(&params).await?;
            self.version.clone_from(&page.metadata.resource_version);
            continue_token = page.metadata.continue_.filter(|token| !token.is_empty());

            for object in page.items {
                self.pending.push_back(WatchNotice::about(
                    NoticeKind::Existing,
                    self.kind.tidy(object),
                ));
            }

            if continue_token.is_none() {
                break;
            }
        }

        self.pending
            .push_back(WatchNotice::synced(self.version.clone()));
        self.events = None;

        Ok(())
    }

    async fn open(&mut self) -> Result<Events, KubeError> {
        let mut params = WatchParams::default().timeout(WATCH_SECONDS);
        if let Some(selector) = &self.selector {
            params = params.labels(selector);
        }

        let version = self.version.clone().unwrap_or_default();
        let events = self.api.watch(&params, &version).await?;

        Ok(events.boxed())
    }

    async fn take_event(&mut self) -> Result<(), KubeError> {
        let Some(events) = self.events.as_mut() else {
            return Ok(());
        };

        let Some(event) = events.next().await else {
            self.events = None;
            return Ok(());
        };

        match event? {
            WatchEvent::Added(object) => self.notice(NoticeKind::Added, object),
            WatchEvent::Modified(object) => self.notice(NoticeKind::Modified, object),
            WatchEvent::Deleted(object) => self.notice(NoticeKind::Deleted, object),
            WatchEvent::Bookmark(bookmark) => {
                self.version = Some(bookmark.metadata.resource_version);
            }
            WatchEvent::Error(status) if status.code == GONE => self.version = None,
            WatchEvent::Error(status) => return Err(KubeError::Api(status.message)),
        }

        Ok(())
    }

    fn notice(&mut self, kind: NoticeKind, object: DynamicObject) {
        if let Some(version) = &object.metadata.resource_version {
            self.version = Some(version.clone());
        }

        self.pending
            .push_back(WatchNotice::about(kind, self.kind.tidy(object)));
    }

    fn list_params(&self) -> ListParams {
        let params = ListParams::default().limit(PAGE);

        match &self.selector {
            Some(selector) => params.labels(selector),
            None => params,
        }
    }
}

impl Debug for Watch {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Watch")
            .field("kind", &self.kind.to_string())
            .field("version", &self.version)
            .finish_non_exhaustive()
    }
}
