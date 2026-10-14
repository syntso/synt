use serde_json::Value;
use synt_journal::Name;

use super::Outside;
use super::log_allowance::LogAllowance;
use crate::ExtensionError;
use crate::bindings::synt::extension::types::{Error, Level};
use crate::caller::ExtensionCall;
use crate::manifest::Manifest;
use crate::runtime::Imports;
use crate::shelf::Shelf;

const LONGEST_KEY: usize = 256;
const LONGEST_LOG: usize = 4096;

pub(crate) struct Visit {
    manifest: Manifest,
    shelf: Shelf,
    outside: Option<Outside>,
    logs: LogAllowance,
}

impl Visit {
    pub(crate) fn new(manifest: Manifest, shelf: Shelf, outside: Option<Outside>) -> Self {
        Self {
            manifest,
            shelf,
            outside,
            logs: LogAllowance::new(),
        }
    }

    fn id(&self) -> &str {
        &self.manifest.id
    }

    fn note_cut(&mut self) {
        if self.logs.cut_once() {
            eprintln!(
                "synt-core: extension {}: further log lines of this call are dropped",
                self.id()
            );
        }
    }
}

impl Imports for Visit {
    fn call(
        &mut self,
        action: String,
        target: Option<String>,
        params: String,
    ) -> Result<String, Error> {
        if !self.manifest.capabilities.may_call(&action) {
            return Err(error(
                "not_allowed",
                format!(
                    "{action} is not listed in capabilities.call of {}",
                    self.id()
                ),
            ));
        }

        let Some(outside) = &self.outside else {
            return Err(error(
                "unavailable",
                "calls are not available while describing",
            ));
        };

        let params = parse_params(&params)?;
        let extension =
            Name::new(self.id()).map_err(|reason| error("internal", reason.to_string()))?;
        let call = ExtensionCall {
            extension,
            action,
            target,
            params,
        };

        let answer = outside
            .runtime
            .block_on(outside.caller.call(call, &outside.context));

        match answer {
            Ok(value) => Ok(value.to_string()),
            Err(failure) => Err(error(failure.code, failure.message)),
        }
    }

    fn storage_get(&mut self, key: String) -> Option<Vec<u8>> {
        if !self.manifest.capabilities.storage {
            return None;
        }

        match self.shelf.item(self.id(), &key) {
            Ok(value) => value,
            Err(failure) => {
                eprintln!("synt-core: extension {}: storage: {failure}", self.id());
                None
            }
        }
    }

    fn storage_set(&mut self, key: String, value: Vec<u8>) -> Result<(), Error> {
        if !self.manifest.capabilities.storage {
            return Err(error(
                "not_allowed",
                format!("{} has no storage = true in its capabilities", self.id()),
            ));
        }

        if key.is_empty() || key.len() > LONGEST_KEY {
            return Err(error(
                "invalid_key",
                format!("a key holds 1 to {LONGEST_KEY} bytes"),
            ));
        }

        self.shelf
            .put_item(self.id(), &key, &value)
            .map_err(|failure| storage_error(&failure))
    }

    fn log(&mut self, level: Level, message: String) {
        let shown: String = message
            .chars()
            .take(LONGEST_LOG)
            .map(|character| {
                if character.is_control() {
                    ' '
                } else {
                    character
                }
            })
            .collect();

        if !self.logs.admit(shown.len()) {
            self.note_cut();
            return;
        }

        eprintln!(
            "synt-core: extension {}: {}: {shown}",
            self.id(),
            level_name(level)
        );
    }
}

fn parse_params(params: &str) -> Result<Value, Error> {
    if params.trim().is_empty() {
        return Ok(Value::Null);
    }

    serde_json::from_str(params)
        .map_err(|reason| error("invalid_params", format!("params are not JSON: {reason}")))
}

fn storage_error(failure: &ExtensionError) -> Error {
    match failure {
        ExtensionError::Refused(reason) => error("storage_full", reason.clone()),
        other => error("storage_failed", other.to_string()),
    }
}

fn level_name(level: Level) -> &'static str {
    match level {
        Level::Trace => "trace",
        Level::Debug => "debug",
        Level::Info => "info",
        Level::Warn => "warn",
        Level::Error => "error",
    }
}

fn error(code: impl Into<String>, message: impl Into<String>) -> Error {
    Error {
        code: code.into(),
        message: message.into(),
    }
}
