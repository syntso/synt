use std::io;

use serde_json::Value;
use synt_actions::ActionError;
use synt_journal::{ActionId, NameError, ResourceRef, Seq};
use synt_proto::RunPhase;
use thiserror::Error;

use super::RunbookName;
use super::param::ParamName;

#[derive(Debug, Error)]
pub enum RunbookError {
    #[error("{text:?} must be 1 to {longest} characters on one line")]
    NotOneLine { text: String, longest: usize },

    #[error("{0}")]
    BadName(String),

    #[error("param {0:?}: a name starts with a letter or _ and holds only letters, digits and _")]
    BadParamName(String),

    #[error("expect field {0:?} must be a JSON pointer such as /state")]
    BadPointer(String),

    #[error("action: {0}")]
    BadAction(#[source] NameError),

    #[error("{0}")]
    BadReference(#[source] NameError),

    #[error("a runbook has at most {0} params")]
    TooManyParams(usize),

    #[error("param {0} is declared twice")]
    DeclaredTwice(ParamName),

    #[error("a runbook has 1 to {0} steps")]
    StepCount(usize),

    #[error("a runbook has at most {0} rollback steps")]
    RollbackCount(usize),

    #[error("a step rolls back, but the runbook has no rollback steps")]
    NoRollback,

    #[error("rollback steps always go on after a failure, so on_failure does not apply")]
    RollbackOnFailure,

    #[error("{action} cannot be a step of a runbook")]
    NestedRunbook { action: ActionId },

    #[error("{{{0}}} names no param; declare it, or write {{{{ and }}}} for literal braces")]
    UndeclaredParam(ParamName),

    #[error("{} {number}: {source}", phase_noun(*.phase))]
    Step {
        phase: RunPhase,
        number: usize,
        source: Box<RunbookError>,
    },

    #[error("retry attempts must be 1 to {most}, got {attempts}")]
    Attempts { attempts: u32, most: u32 },

    #[error("retry every_seconds must be at most {longest}, got {pause}")]
    Pause { pause: u64, longest: u64 },

    #[error("param {0}: a choice needs choices")]
    NoChoices(ParamName),

    #[error("param {0}: only a choice has choices")]
    StrayChoices(ParamName),

    #[error("param {0}: only an integer has min and max")]
    StrayBounds(ParamName),

    #[error("param {name}: min {min} is above max {max}")]
    UpsideDown { name: ParamName, min: i64, max: i64 },

    #[error("expected text, got {0}")]
    NotText(Value),

    #[error("text is longer than {0} bytes")]
    TextTooLong(usize),

    #[error("text cannot hold control characters such as new lines")]
    ControlInText,

    #[error("{word:?} is not a word: use 1 to {longest} letters, digits or {symbols}")]
    NotWord {
        word: String,
        longest: usize,
        symbols: &'static str,
    },

    #[error("expected a whole number, got {0}")]
    NotInteger(Value),

    #[error("expected true or false, got {0}")]
    NotBoolean(Value),

    #[error("{0:?} is not a target: name one host, such as @project/api-1 or deploy@10.0.1.12")]
    NotTarget(String),

    #[error("a path cannot be empty")]
    EmptyPath,

    #[error("{chosen:?} is not one of {}", .choices.join(", "))]
    NotAChoice {
        chosen: String,
        choices: Vec<String>,
    },

    #[error("{number} is outside {}..{}", bound(*.min), bound(*.max))]
    OutOfBounds {
        number: i64,
        min: Option<i64>,
        max: Option<i64>,
    },

    #[error("the runbook has no param {0}")]
    UnknownParam(String),

    #[error("param {0} is required")]
    MissingParam(ParamName),

    #[error("param {name}: {source}")]
    Param {
        name: ParamName,
        source: Box<RunbookError>,
    },

    #[error("no value for param {0}")]
    NoValue(String),

    #[error("expected exit {0}, but the result has no exit")]
    NoExitCarried(i64),

    #[error("exit {exit}, expected {wanted}")]
    WrongExit { exit: i64, wanted: i64 },

    #[error("no exit code, expected {wanted}{}", signal_note(.signal.as_deref()))]
    NoExitCode { wanted: i64, signal: Option<String> },

    #[error("the output does not contain {0:?}")]
    OutputLacks(String),

    #[error("{pointer} is {found}, expected {wanted}")]
    WrongField {
        pointer: String,
        found: Value,
        wanted: Value,
    },

    #[error("the result has no {pointer}, expected {wanted}")]
    MissingField { pointer: String, wanted: Value },

    #[error("there is no action {0}")]
    NoAction(ActionId),

    #[error("{0} is for the person only, so a runbook cannot run it")]
    PersonOnly(ActionId),

    #[error("{0} needs a target")]
    NeedsTarget(ActionId),

    #[error("{0} takes no target")]
    TakesNoTarget(ActionId),

    #[error("{0} does not name a host this core can reach")]
    Unreachable(ResourceRef),

    #[error(transparent)]
    Refused(#[from] ActionError),

    #[error("runbook {name}: {source}")]
    Runbook {
        name: RunbookName,
        source: Box<RunbookError>,
    },

    #[error("there is no such runbook")]
    NoSuchRunbook,

    #[error("the file is not UTF-8 text")]
    NotUtf8,

    #[error("{0}")]
    Unreadable(#[source] io::Error),

    #[error("{0}")]
    Malformed(String),

    #[error("cannot write {name}: {source}")]
    Unwritable {
        name: RunbookName,
        source: toml::ser::Error,
    },

    #[error("runbook {name} of project {project}: {source}")]
    InProject {
        name: RunbookName,
        project: String,
        source: Box<RunbookError>,
    },

    #[error(
        "runbook {name} of project {project} is now {actual}, which is not what the call was \
         admitted for; run it again"
    )]
    Moved {
        name: RunbookName,
        project: String,
        actual: String,
    },

    #[error("the runbook cannot be written as TOML: {0}")]
    Unprintable(#[source] toml::ser::Error),

    #[error("cannot write runbook {name}: {source}")]
    NotSaved {
        name: RunbookName,
        source: io::Error,
    },

    #[error("the journal has no event {0}")]
    NoEvent(Seq),

    #[error("journal event {0} belongs to no call")]
    NoCall(Seq),

    #[error("journal event {0} is not a request")]
    NotARequest(Seq),

    #[error("{what} must list 1 to {most} {numbers}")]
    Picks {
        what: &'static str,
        most: usize,
        numbers: &'static str,
    },

    #[error("there is no open session {0}")]
    NoSession(String),

    #[error("session {0} is a local shell, so its commands have no host to run on")]
    LocalSession(String),

    #[error("the session has no finished block {0}")]
    NoBlock(u64),

    #[error("block {0} has no command line; its shell did not report one")]
    NoCommand(u64),

    #[error("runbooks are not connected to a running core")]
    NotConnected,

    #[error("runbooks are already connected to a core")]
    AlreadyConnected,

    #[error("this call has no connection to report progress on")]
    NoConnection,

    #[error("the runbook was not planned before it ran")]
    NotPlanned,

    #[error("there is no running run {0}")]
    NotRunning(u64),
}

impl RunbookError {
    pub(crate) fn in_step(self, phase: RunPhase, number: usize) -> Self {
        Self::Step {
            phase,
            number,
            source: Box::new(self),
        }
    }

    pub(crate) fn in_param(self, name: &ParamName) -> Self {
        Self::Param {
            name: name.clone(),
            source: Box::new(self),
        }
    }

    pub(crate) fn in_runbook(self, name: &RunbookName) -> Self {
        Self::Runbook {
            name: name.clone(),
            source: Box::new(self),
        }
    }

    pub(crate) fn in_project(self, name: &RunbookName, project: &str) -> Self {
        Self::InProject {
            name: name.clone(),
            project: project.to_owned(),
            source: Box::new(self),
        }
    }

    fn is_failure(&self) -> bool {
        matches!(
            self,
            Self::Moved { .. }
                | Self::NotSaved { .. }
                | Self::Unprintable(_)
                | Self::NoSession(_)
                | Self::NotConnected
                | Self::AlreadyConnected
                | Self::NoConnection
                | Self::NotPlanned
                | Self::NotRunning(_)
        )
    }
}

impl From<RunbookError> for ActionError {
    fn from(error: RunbookError) -> Self {
        if error.is_failure() {
            return Self::Failed(error.to_string());
        }

        Self::InvalidParams(error.to_string())
    }
}

fn phase_noun(phase: RunPhase) -> &'static str {
    match phase {
        RunPhase::Steps => "step",
        RunPhase::Rollback => "rollback step",
    }
}

fn bound(limit: Option<i64>) -> String {
    limit.map(|limit| limit.to_string()).unwrap_or_default()
}

fn signal_note(signal: Option<&str>) -> String {
    signal
        .map(|signal| format!(" (ended by signal {signal})"))
        .unwrap_or_default()
}
