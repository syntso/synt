wasmtime::component::bindgen!({
    path: "wit/0.2",
    world: "extension",
    imports: { default: trappable },
});
