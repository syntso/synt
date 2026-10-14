use super::Sandbox;
use crate::bindings::{v1, v2};

#[derive(Clone)]
pub(crate) enum Compiled {
    V1(v1::ExtensionPre<Sandbox>),
    V2(v2::ExtensionPre<Sandbox>),
}
