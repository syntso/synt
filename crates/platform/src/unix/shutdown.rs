use nix::sys::signal::{SigSet, Signal};
use tokio::signal::unix::{SignalKind, signal};

pub(crate) async fn shutdown_signal() {
    accept_shutdown_signals();

    let Ok(mut terminate) = signal(SignalKind::terminate()) else {
        let _ = tokio::signal::ctrl_c().await;
        return;
    };

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = terminate.recv() => {}
    }
}

fn accept_shutdown_signals() {
    let shutdown = SigSet::from_iter([Signal::SIGINT, Signal::SIGTERM]);
    let _ = shutdown.thread_unblock();
}
