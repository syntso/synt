use crate::native;

pub async fn shutdown_signal() {
    native::shutdown_signal().await;
}
