wasmtime::component::bindgen!({
    path: "wit/0.2",
    world: "extension",
    imports: { default: trappable },
});

super::types::share_types!(synt::extension::types);
