use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use synt_actions::{ActionError, ActionGroup, Context, Prepared, RegisteredAction, Registry};
use synt_extensions::{Extensions, Remote, Runtime};
use synt_journal::{ActionId, Actor, ResourceRef, Seq};
use tempfile::TempDir;

use super::{CoreStub, FakeCaller, RecordingScreen};

pub const SAMPLE_MANIFEST: &str = r#"
id = "t"
name = "Test"
version = "0.1.0"
api = "0.1"
description = "Exercises the extension host."

[capabilities]
call = ["ssh.exec"]
storage = true
"#;

const UNUSED_REGISTRY: &str = "http://127.0.0.1:9";
const BUILT_FIXTURE: &str = "SYNT_EVERYTHING_WASM";
const UNUSED_KEY: &str = "RWQOuk/x7vRoqs2E6Vxffy293iLd8y+C5Yo2oilreJ2V22RL3qFeV7f2";

pub struct Harness {
    pub folder: TempDir,
    pub extensions: Extensions,
    pub registry: Registry,
    pub caller: Arc<FakeCaller>,
    pub screen: Arc<RecordingScreen>,
}

impl Harness {
    pub fn new() -> Self {
        Self::with(
            Runtime::new().unwrap(),
            Remote::new(UNUSED_REGISTRY, UNUSED_KEY).unwrap(),
        )
    }

    pub fn with_limits(fuel: u64, time: Duration) -> Self {
        Self::with(
            Runtime::new().unwrap().with_limits(fuel, time),
            Remote::new(UNUSED_REGISTRY, UNUSED_KEY).unwrap(),
        )
    }

    pub fn with_remote(remote: Remote) -> Self {
        Self::with(Runtime::new().unwrap(), remote)
    }

    pub fn with(runtime: Runtime, remote: Remote) -> Self {
        let folder = tempfile::tempdir().unwrap();
        Self::in_folder(folder, runtime, remote)
    }

    pub fn in_folder(folder: TempDir, runtime: Runtime, remote: Remote) -> Self {
        let data = folder.path().join("data");
        let state = folder.path().join("state.sqlite");
        let extensions = Extensions::open(&data, &state, runtime, remote).unwrap();

        let caller = Arc::new(FakeCaller::default());
        let screen = Arc::new(RecordingScreen::default());
        extensions.connect(Arc::clone(&caller) as _, Arc::clone(&screen) as _);

        let mut registry = Registry::new();
        registry.attach(Arc::new(CoreStub));
        extensions.register(&mut registry).unwrap();

        Self {
            folder,
            extensions,
            registry,
            caller,
            screen,
        }
    }

    pub fn build(&self, name: &str, manifest: &str) -> PathBuf {
        let build = self.folder.path().join(name);
        fs::create_dir_all(&build).unwrap();

        fs::write(build.join("extension.toml"), manifest).unwrap();
        fs::write(build.join("extension.wasm"), sample_component()).unwrap();

        build
    }

    pub fn handle(&self, action: &str) -> Option<RegisteredAction> {
        self.registry.get(&ActionId::new(action).unwrap())
    }

    pub fn prepare(
        &self,
        action: &str,
        target: Option<&str>,
        params: Value,
    ) -> Result<Prepared, ActionError> {
        let target = target.map(|target| ResourceRef::new(target).unwrap());
        let handle = self
            .handle(action)
            .unwrap_or_else(|| panic!("no action {action}"));

        handle.prepare(target.as_ref(), params)
    }

    pub async fn call(
        &self,
        action: &str,
        target: Option<&str>,
        params: Value,
    ) -> Result<Value, ActionError> {
        let prepared = self.prepare(action, target, params)?;
        let target = target.map(|target| ResourceRef::new(target).unwrap());
        let context = Context::new(Actor::Person, Seq::FIRST, target);

        prepared
            .run(&context)
            .await
            .map(|completed| completed.value)
    }

    pub async fn install_local(&self, build: &Path) -> Result<Value, ActionError> {
        let params = serde_json::json!({ "path": build.to_string_lossy() });

        self.call("extension.install_local", None, params).await
    }

    pub async fn install_sample(&self) -> Value {
        let build = self.build("sample", SAMPLE_MANIFEST);

        self.install_local(&build).await.unwrap()
    }
}

pub fn sample_component() -> Vec<u8> {
    wat::parse_str(include_str!("../fixtures/sample.wat")).unwrap()
}

pub fn everything_component() -> Vec<u8> {
    let path = env::var_os(BUILT_FIXTURE).map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/extension/everything.wasm"),
        PathBuf::from,
    );

    fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}
