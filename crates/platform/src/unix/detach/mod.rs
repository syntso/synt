mod attributes;
mod descriptors;
mod environment;
mod spawn;

pub(crate) use environment::detached_environment;
pub(crate) use spawn::spawn_detached;
