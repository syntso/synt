use super::ScreenUpdate;

pub trait Screen: Send + Sync + 'static {
    fn show(&self, update: ScreenUpdate);
}
