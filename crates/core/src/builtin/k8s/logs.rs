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
use super::object_names::{check_namespace, check_object_name};
use super::stream_pump::{outbound, pump_logs};
use super::streams::StreamStarted;

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

impl K8sLogs {
    pub(super) fn new(kube: Kube) -> Self {
        Self { kube }
    }

    async fn follow(
        &self,
        context: &Context,
        cluster: Arc<Cluster>,
        params: &LogsParams,
        request: LogRequest,
    ) -> Result<Logs, ActionError> {
        let stream = params
            .stream
            .ok_or_else(|| ActionError::InvalidParams("follow needs a stream".to_owned()))?;

        let outbound = outbound(context, stream)?;
        let logs = cluster.follow_logs(&request).await.map_err(failed)?;

        let about = json!({
            "pod": request.pod,
            "namespace": request.namespace.as_deref().unwrap_or(cluster.namespace()),
            "container": request.container,
            "stream": stream.0,
        });
        self.kube
            .streams()
            .start(context, Self::ID, about, move |stop| async move {
                let ended = pump_logs(logs, outbound, stop).await;
                drop(cluster);
                ended
            });

        Ok(Logs::Followed(StreamStarted {
            request: context.request().get(),
            stream: stream.0,
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

    fn check(&self, _target: Option<&ResourceRef>, params: &LogsParams) -> Result<(), ActionError> {
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

        let request = LogRequest {
            pod: params.pod.clone(),
            namespace: params.namespace.clone(),
            container: params.container.clone(),
            tail_lines: params.tail,
            since_seconds: params.since_seconds,
            previous: params.previous,
            timestamps: params.timestamps,
        };

        if params.follow {
            return self.follow(context, cluster, &params, request).await;
        }

        let text = cluster.logs(&request).await.map_err(failed)?;

        Ok(Logs::Text(text))
    }
}
