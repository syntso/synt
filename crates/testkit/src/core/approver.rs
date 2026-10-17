use std::time::Duration;

use synt_client::{Client, ClientError};
use synt_proto::{
    ActionCall, ActionOutcome, ApprovalDecision, ApprovalRequest, Notice, Notification,
};
use tokio::sync::broadcast::{self, error::TryRecvError};
use tokio::time;

use crate::TestError;

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

    pub async fn next(&mut self) -> Result<ApprovalRequest, TestError> {
        time::timeout(PATIENCE, self.next_question())
            .await
            .map_err(|_| "the core did not ask the app in time")?
    }

    pub fn was_not_asked(&mut self) -> bool {
        loop {
            match self.questions.try_recv() {
                Ok(notification) if notification.method == ApprovalRequest::METHOD => return false,
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
    ) -> Result<(ApprovalRequest, Result<ActionOutcome, ClientError>), TestError> {
        let caller = caller.clone();
        let running = tokio::spawn(async move { caller.call(call).await });

        let question = self.next().await?;
        self.answer(&question, decision).await?;

        Ok((question, running.await?))
    }

    pub async fn approve(
        &mut self,
        caller: &Client,
        call: ActionCall,
        decision: ApprovalDecision,
    ) -> Result<ActionOutcome, TestError> {
        let (_, outcome) = self.decide(caller, call, decision).await?;

        outcome.map_err(|error| format!("the approved call did not run: {error}").into())
    }

    async fn next_question(&mut self) -> Result<ApprovalRequest, TestError> {
        loop {
            let notification = self.questions.recv().await?;

            if let Some(question) = notification.read::<ApprovalRequest>() {
                return Ok(question?);
            }
        }
    }
}
