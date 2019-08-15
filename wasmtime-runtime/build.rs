use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=signalhandlers/SignalHandlers.cpp");
    println!("cargo:rerun-if-changed=signalhandlers/SignalHandlers.hpp");
    println!("cargo:rerun-if-changed=signalhandlers/sjlj.c");
    let target = std::env::var("TARGET").unwrap();
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .warnings(false)
        .file("signalhandlers/SignalHandlers.cpp");
    if !target.contains("windows") {
        build
            .flag("-std=c++11")
            .flag("-fno-exceptions")
            .flag("-fno-rtti");
    }

    build.compile("signalhandlers");

    // As explained in sjlj.c, we need a not unwinding setjmp/longjmp.
    // One way is usage of clang/gcc builtins. They are not available
    // in MSVC which is chosen by default by cc crate on Windows,
    // so as a workaround we call clang directly. We assume that compiler
    // on other platforms supports the used builtins.
    let mut build_sjlj = cc::Build::new();
    build_sjlj.cpp(false);
    if target.contains("windows") {
        let out_dir = std::env::var("OUT_DIR").unwrap();
        let path = Path::new(&out_dir).join("signalhandlers_sjlj.o");
        Command::new("clang")
            .args(&["-c", "signalhandlers/sjlj.c", "-o", path.to_str().unwrap()])
            .output()
            .expect("compilation failed");
        build_sjlj.object(path.to_str().unwrap());
    } else {
        build_sjlj.file("signalhandlers/sjlj.c"); // chosen compiler must support __builtin_{set,long}jmp
    }

    build_sjlj.compile("signalhandlers_sjlj");
}
