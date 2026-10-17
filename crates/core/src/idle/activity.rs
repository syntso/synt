use synt_ssh::Tunnels;
use tokio_util::task::TaskTracker;

use crate::builtin::K8sActions;
use crate::runbooks::Runs;
use crate::terminals::Terminals;

pub trait Activity: Send + Sync {
    fn is_active(&self) -> bool;
}

impl Activity for TaskTracker {
    fn is_active(&self) -> bool {
        !self.is_empty()
    }
}

impl Activity for Terminals {
    fn is_active(&self) -> bool {
        !self.all().is_empty()
    }
}

impl Activity for Tunnels {
    fn is_active(&self) -> bool {
        !self.list().is_empty()
    }
}

impl Activity for Runs {
    fn is_active(&self) -> bool {
        !self.is_empty()
    }
}

impl Activity for K8sActions {
    fn is_active(&self) -> bool {
        self.is_busy()
    }
}
