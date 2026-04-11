fn main() {
    if cfg!(fuzzing) {
        println!("cargo:rustc-cfg=gc_zeal");
    }
}
