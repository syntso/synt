use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_kube::{Cluster, LogRequest, LogText};
use synt_proto::StreamId;

use super::Kube;
use super::failure::failed;
use super::names::{check_namespace, check_object_name};
use super::streams::{Pump, StreamStarted};

#[derive(Debug, Clone)]
pub(crate) struct K8sLogs {
    kube: Kube,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub(crate) struct LogsParams {
    pub pod: String,

    #[serde(default)]
    pub namespace: Option<String>,

    #[serde(default)]
    pub container: Option<String>,

    #[serde(default)]
    pub tail: Option<u32>,

    #[serde(default)]
    pub since_seconds: Option<u32>,

    #[serde(default)]
    pub previous: bool,

    #[serde(default)]
    pub timestamps: bool,

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
    fn into_request(self) -> LogRequest {
        LogRequest {
            pod: self.pod,
            namespace: self.namespace,
            container: self.container,
            tail_lines: self.tail,
            since_seconds: self.since_seconds,
            previous: self.previous,
            timestamps: self.timestamps,
        }
    }
}

impl K8sLogs {
    pub(super) fn new(kube: Kube) -> Self {
        Self { kube }
    }

    async fn follow(
        &self,
        context: &Context,
        cluster: Arc<Cluster>,
        stream: Option<StreamId>,
        request: LogRequest,
    ) -> Result<Logs, ActionError> {
        let stream =
            stream.ok_or_else(|| ActionError::InvalidParams("follow needs a stream".to_owned()))?;

        let pump = Pump::open(context, stream)?;
        let logs = cluster.follow_logs(&request).await.map_err(failed)?;

        let about = json!({
            "pod": request.pod,
            "namespace": request.namespace.as_deref().unwrap_or(cluster.namespace()),
            "container": request.container,
            "stream": stream.get(),
        });
        self.kube
            .streams()
            .start(context, Self::ID, about, move |stop| async move {
                let ended = pump.run(logs, stop).await;
                drop(cluster);
                ended
            });

        Ok(Logs::Followed(StreamStarted {
            request: context.request().get(),
            stream: stream.get(),
        }))
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

        check_object_name("pod", &params.pod)?;
        check_namespace(params.namespace.as_deref())?;

        if let Some(container) = &params.container {
            check_object_name("container", container)?;
        }

        if params.follow != params.stream.is_some() {
            return Err(ActionError::InvalidParams(
                "follow and stream go together: name the stream to follow the logs on".to_owned(),
            ));
        }

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
        let cluster = self.kube.cluster(context).await?;

        let (follow, stream) = (params.follow, params.stream);
        let request = params.into_request();

        if follow {
            return self.follow(context, cluster, stream, request).await;
        }

        let text = cluster.logs(&request).await.map_err(failed)?;

        Ok(Logs::Text(text))
    }
}
