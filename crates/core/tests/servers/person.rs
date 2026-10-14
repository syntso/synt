use synt_client::Client;
use synt_proto::{ApprovalDecision, ApprovalRequest, Notification};
use tokio::sync::broadcast::error::RecvError;
use tokio::task::JoinHandle;

pub struct Person {
    answering: JoinHandle<()>,
}

impl Person {
    pub fn approving_once(app: &Client) -> Self {
        let app = app.clone();
        let questions = app.notifications();

        let answering = tokio::spawn(async move {
            let mut questions = questions;

            loop {
                let notification = match questions.recv().await {
                    Ok(notification) => notification,
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => return,
                };

                if let Some(question) = question(&notification) {
                    let _ = app.respond(question.id, ApprovalDecision::AllowOnce).await;
                }
            }
        });

        Self { answering }
    }
}

impl Drop for Person {
    fn drop(&mut self) {
        self.answering.abort();
    }
}

fn question(notification: &Notification) -> Option<ApprovalRequest> {
    notification.read::<ApprovalRequest>()?.ok()
}
