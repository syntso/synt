pub(super) trait NativeShutdown {
    async fn signal();
}
