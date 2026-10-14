wasmtime::component::bindgen!({
    path: "wit",
    world: "extension",
    imports: { default: trappable },
});
