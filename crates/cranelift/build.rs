fn main() {
    if std::env::var("CARGO_CFG_FUZZING").is_ok() {
        println!("cargo:rustc-cfg=gc_zeal");
    }

    // Pretend that Cranelift has these features. These are injected via the
    // `wasmtime_environ::foreach_builtin_function!` macro so to ensure that we
    // don't get spurious warnings about unused cfg's these are printed out
    // here.
    faux_feature("stack-switching");
    faux_feature("gc");
    faux_feature("gc-copying");
    faux_feature("gc-null");
    faux_feature("gc-drc");
    faux_feature("threads");
    faux_feature("wmemcheck");
    faux_feature("component-model");
    faux_feature("incremental-cache");
}

fn faux_feature(feature: &str) {
    println!("cargo:rustc-check-cfg=cfg(feature, values(\"{feature}\"))");
    println!("cargo:rustc-cfg=feature=\"{feature}\"");
}
