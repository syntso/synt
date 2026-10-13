use tokio::signal::windows::ctrl_close;

pub(crate) async fn shutdown_signal() {
    let Ok(mut close) = ctrl_close() else {
        let _ = tokio::signal::ctrl_c().await;
        return;
    };

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = close.recv() => {}
    }
}
