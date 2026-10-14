use tokio::task::JoinHandle;

pub struct SyncLoop {
    task: JoinHandle<()>,
}

impl SyncLoop {
    pub(crate) fn new(task: JoinHandle<()>) -> Self {
        Self { task }
    }
}

impl Drop for SyncLoop {
    fn drop(&mut self) {
        self.task.abort();
    }
}
