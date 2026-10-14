use std::time::Duration;

use synt_client::{Client, ClientError};
use synt_proto::{ActionCall, ActionOutcome, ApprovalDecision, ApprovalRequest, Notification};
use tokio::sync::broadcast::{self, error::TryRecvError};
use tokio::time;

const PATIENCE: Duration = Duration::from_secs(10);

pub struct Approver {
    app: Client,
    questions: broadcast::Receiver<Notification>,
}

impl Approver {
    pub fn new(app: Client) -> Self {
        let questions = app.notifications();

        Self { app, questions }
    }

    pub fn app(&self) -> &Client {
        &self.app
    }

    pub async fn next(&mut self) -> ApprovalRequest {
        time::timeout(PATIENCE, self.next_question())
            .await
            .expect("the core asks the app in time")
    }

    pub fn was_not_asked(&mut self) -> bool {
        loop {
            match self.questions.try_recv() {
                Ok(notification) if is_question(&notification) => return false,
                Ok(_) => {}
                Err(TryRecvError::Empty | TryRecvError::Closed) => return true,
                Err(TryRecvError::Lagged(_)) => return false,
            }
        }
    }

    pub async fn answer(
        &self,
        question: &ApprovalRequest,
        decision: ApprovalDecision,
    ) -> Result<(), ClientError> {
        self.app.respond(question.id, decision).await
    }

    pub async fn decide(
        &mut self,
        caller: &Client,
        call: ActionCall,
        decision: ApprovalDecision,
    ) -> (ApprovalRequest, Result<ActionOutcome, ClientError>) {
        let caller = caller.clone();
        let running = tokio::spawn(async move { caller.call(call).await });

        let question = self.next().await;
        self.answer(&question, decision).await.unwrap();

        (question, running.await.unwrap())
    }

    pub async fn approve(
        &mut self,
        caller: &Client,
        call: ActionCall,
        decision: ApprovalDecision,
    ) -> ActionOutcome {
        let (_, outcome) = self.decide(caller, call, decision).await;

        outcome.expect("the approved call runs")
    }

    async fn next_question(&mut self) -> ApprovalRequest {
        loop {
            let notification = self.questions.recv().await.unwrap();

            if is_question(&notification) {
                return notification.params().unwrap();
            }
        }
    }
}

fn is_question(notification: &Notification) -> bool {
    notification.method == ApprovalRequest::METHOD
}
