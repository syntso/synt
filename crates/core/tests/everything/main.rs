#![allow(clippy::unwrap_used)]

#[path = "../common/mod.rs"]
mod common;

mod documents;
mod echo_server;
mod events;
mod files;
mod http;
mod settings;
mod stage;
mod sync;
mod timers;
mod views;

use echo_server::EchoServer;
use stage::Stage;
