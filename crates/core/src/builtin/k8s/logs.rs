use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_kube::{Cluster, LogRequest, LogText, Namespace};
use synt_proto::StreamId;

use super::streams::{Pump, StreamStarted};
use super::{K8sError, Kube};

#[derive(Debug, Clone)]
pub(crate) struct K8sLogs {
    kube: Kube,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub(crate) struct LogsParams {
    #[serde(flatten)]
    pub request: LogRequest,

    #[serde(default)]
    pub follow: bool,

    #[serde(default)]
    #[schemars(with = "Option<u32>")]
    pub stream: Option<StreamId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub(crate) enum Logs {
    Text(LogText),
    Followed(StreamStarted),
}

impl LogsParams {
    fn followed_on(&self) -> Result<Option<StreamId>, K8sError> {
        if self.follow != self.stream.is_some() {
            return Err(K8sError::FollowWithoutStream);
        }

        Ok(self.stream)
    }
}

impl K8sLogs {
    pub(super) fn new(kube: Kube) -> Self {
        Self { kube }
    }

    async fn logs(&self, context: &Context, params: LogsParams) -> Result<Logs, K8sError> {
        let cluster = self.kube.cluster(context).await?;

        if let Some(stream) = params.followed_on()? {
            return self.follow(context, cluster, stream, params.request).await;
        }

        let text = params.request.read(&cluster).await?;

        Ok(Logs::Text(text))
    }

    async fn follow(
        &self,
        context: &Context,
        cluster: Arc<Cluster>,
        stream: StreamId,
        request: LogRequest,
    ) -> Result<Logs, K8sError> {
        let pump = Pump::open(context, stream)?;
        let logs = request.follow(&cluster).await?;

        let about = json!({
            "pod": request.pod.as_str(),
            "namespace": request.namespace.as_ref().map_or(cluster.namespace(), Namespace::as_str),
            "container": request.container.as_ref().map(ToString::to_string),
            "stream": stream.0,
        });
        self.kube
            .streams()
            .start(context, Self::ID, about, move |stop| async move {
                let ended = pump.run(logs, stop).await;
                drop(cluster);
                ended
            });

        Ok(Logs::Followed(StreamStarted::new(context, stream)))
    }
}

impl Action for K8sLogs {
    const ID: &'static str = "k8s.logs";
    const TITLE: &'static str = "Read or follow the logs of a pod";
    const RISK: Risk = Risk::Read;
    const TAKES_TARGET: bool = true;

    type Params = LogsParams;
    type Output = Logs;

    fn check(&self, target: Option<&ResourceRef>, params: &LogsParams) -> Result<(), ActionError> {
        self.kube.reach(target)?;
        params.followed_on()?;

        Ok(())
    }

    fn risk(&self, target: Option<&ResourceRef>, _params: &LogsParams) -> Risk {
        self.kube.risk(target, Self::RISK)
    }

    fn destination(&self, target: Option<&ResourceRef>, _params: &LogsParams) -> Option<String> {
        self.kube.destination(target)
    }

    fn finishes_later(&self, params: &LogsParams) -> bool {
        params.follow
    }

    fn summary(&self, output: &Logs) -> Value {
        match output {
            Logs::Text(text) => json!({
                "bytes": text.bytes,
                "lines": text.lines,
                "truncated": text.truncated,
            }),
            Logs::Followed(started) => json!({ "stream": started.stream }),
        }
    }

    async fn run(&self, context: &Context, params: LogsParams) -> Result<Logs, ActionError> {
        Ok(self.logs(context, params).await?)
    }
}
