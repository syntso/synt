mod collector;
mod interruption;
mod outcome;
mod remote_command;

use collector::Collector;
use interruption::Interruption;
pub use outcome::ExecOutcome;
pub(crate) use remote_command::RemoteCommand;
