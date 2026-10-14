use kube::api::LogParams;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LogRequest {
    pub pod: String,
    pub namespace: Option<String>,
    pub container: Option<String>,
    pub tail_lines: Option<u32>,
    pub since_seconds: Option<u32>,
    pub previous: bool,
    pub timestamps: bool,
}

impl LogRequest {
    pub(crate) fn params(&self, follow: bool) -> LogParams {
        LogParams {
            container: self.container.clone(),
            follow,
            previous: self.previous,
            since_seconds: self.since_seconds.map(i64::from),
            tail_lines: self.tail_lines.map(i64::from),
            timestamps: self.timestamps,
            ..LogParams::default()
        }
    }
}
