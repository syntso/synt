use std::time::Duration;

use synt_client::{Client, ClientError};
use synt_proto::{
    ActionCall, ActionOutcome, ApprovalDecision, ApprovalRequest, Notice, Notification,
};
use synt_testkit::TestError;
use tokio::sync::Mutex;
use tokio::sync::broadcast::{self, error::RecvError};
use tokio::time;

const PATIENCE: Duration = Duration::from_secs(60);

pub struct Desk {
    app: Client,
    questions: Mutex<broadcast::Receiver<Notification>>,
}

impl Desk {
    pub fn new(app: &Client) -> Self {
        Self {
            app: app.clone(),
            questions: Mutex::new(app.notifications()),
        }
    }

    pub async fn decide(
        &self,
        caller: &Client,
        call: ActionCall,
        decision: ApprovalDecision,
    ) -> Result<(ApprovalRequest, Result<ActionOutcome, ClientError>), TestError> {
        let mut questions = self.questions.lock().await;
        let caller = caller.clone();
        let running = tokio::spawn(async move { caller.call(call).await });

        let question = time::timeout(PATIENCE, next_question(&mut questions))
            .await
            .map_err(|_| "the core did not ask the person in time")??;
        self.app.respond(question.id, decision).await?;

        Ok((question, running.await?))
    }
}

async fn next_question(
    questions: &mut broadcast::Receiver<Notification>,
) -> Result<ApprovalRequest, TestError> {
    loop {
        match questions.recv().await {
            Ok(notification) if notification.method == ApprovalRequest::METHOD => {
                let question = notification
                    .read::<ApprovalRequest>()
                    .ok_or("the notification carries no question")?;

                return Ok(question?);
            }
            Ok(_) | Err(RecvError::Lagged(_)) => {}
            Err(RecvError::Closed) => return Err("the app connection closed".into()),
        }
    }
}
