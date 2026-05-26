fn main() {
    if std::env::var("CARGO_CFG_FUZZING").is_ok() {
        println!("cargo:rustc-cfg=gc_zeal");
    }
}
