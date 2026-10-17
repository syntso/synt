use tokio::signal::windows::ctrl_close;

use super::NativeShutdown;

pub(super) struct Windows;

impl NativeShutdown for Windows {
    async fn signal() {
        let Ok(mut close) = ctrl_close() else {
            let _ = tokio::signal::ctrl_c().await;
            return;
        };

        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = close.recv() => {}
        }
    }
}
