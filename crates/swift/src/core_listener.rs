#[uniffi::export(with_foreign)]
pub trait CoreListener: Send + Sync {
    fn notification(&self, method: String, params: String);

    fn stream_data(&self, stream: u32, data: Vec<u8>);

    fn stream_ended(&self, stream: u32, ended: String);

    fn disconnected(&self);
}
