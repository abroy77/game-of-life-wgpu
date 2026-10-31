use naga::{
    front::wgsl,
    valid::{Capabilities, ValidationFlags, Validator},
};
fn validate(name: &str, src: &str) {
    let module =
        wgsl::parse_str(src).unwrap_or_else(|e| panic!("{name}:\n{}", e.emit_to_string(src)));
    Validator::new(ValidationFlags::all(), Capabilities::default())
        .validate(&module)
        .unwrap_or_else(|e| panic!("{name}:\n{}", e.emit_to_string(src)));
}

#[test]
fn shaders_are_valid() {
    validate("paint.wgsl", include_str!("../src/shaders/paint.wgsl"));
    validate("compute.wgsl", include_str!("../src/shaders/compute.wgsl"));
    validate("render.wgsl", include_str!("../src/shaders/render.wgsl"));
}
