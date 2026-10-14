use kube::api::AttachParams;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExecRequest {
    pub pod: String,
    pub namespace: Option<String>,
    pub container: Option<String>,
    pub command: Vec<String>,
}

impl ExecRequest {
    pub(crate) fn params(&self) -> AttachParams {
        let params = AttachParams::default()
            .stdin(false)
            .stdout(true)
            .stderr(true);

        match &self.container {
            Some(container) => params.container(container),
            None => params,
        }
    }
}
