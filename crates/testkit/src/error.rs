use std::error::Error;

pub type TestError = Box<dyn Error + Send + Sync>;
