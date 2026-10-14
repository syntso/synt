use std::time::Duration;

use synt_client::{Client, ClientError};
use synt_proto::{
    ActionCall, ActionOutcome, ApprovalDecision, ApprovalRequest, Notice, Notification,
};
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
    ) -> (ApprovalRequest, Result<ActionOutcome, ClientError>) {
        let mut questions = self.questions.lock().await;
        let caller = caller.clone();
        let running = tokio::spawn(async move { caller.call(call).await });

        let question = time::timeout(PATIENCE, next_question(&mut questions))
            .await
            .expect("the core asks the person in time");
        self.app.respond(question.id, decision).await.unwrap();

        (question, running.await.unwrap())
    }
}

async fn next_question(questions: &mut broadcast::Receiver<Notification>) -> ApprovalRequest {
    loop {
        match questions.recv().await {
            Ok(notification) if notification.method == ApprovalRequest::METHOD => {
                return notification.read::<ApprovalRequest>().unwrap().unwrap();
            }
            Ok(_) | Err(RecvError::Lagged(_)) => {}
            Err(RecvError::Closed) => panic!("the app connection closed"),
        }
    }
}
