wasmtime::component::bindgen!({
    path: "wit/0.1",
    world: "extension",
    imports: { default: trappable },
});
