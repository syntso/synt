use std::sync::Arc;

use wasmtime::Store;
use wasmtime::component::InstancePre;

use super::{Exports, Template};
use crate::bindings::{v1, v2};
use crate::manifest::Api;
use crate::runtime::Sandbox;

#[derive(Clone)]
pub(crate) struct Compiled {
    template: Arc<dyn Template>,
}

impl Compiled {
    pub(crate) fn prepare(api: Api, instance: InstancePre<Sandbox>) -> wasmtime::Result<Self> {
        let template: Arc<dyn Template> = match api {
            Api::V0_1 => Arc::new(v1::ExtensionPre::new(instance)?),
            Api::V0_2 => Arc::new(v2::ExtensionPre::new(instance)?),
        };

        Ok(Self { template })
    }

    pub(crate) fn instance(
        &self,
        store: &mut Store<Sandbox>,
    ) -> wasmtime::Result<Box<dyn Exports>> {
        self.template.instance(store)
    }
}
