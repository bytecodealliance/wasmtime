fn main() {
    println!("cargo:rerun-if-changed=signalhandlers/SignalHandlers.cpp");
    println!("cargo:rerun-if-changed=signalhandlers/SignalHandlers.hpp");
    println!("cargo:rerun-if-changed=signalhandlers/Trampolines.cpp");
    let target = std::env::var("TARGET").unwrap();
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .warnings(false)
        .file("signalhandlers/SignalHandlers.cpp")
        .file("signalhandlers/Trampolines.cpp");
    if !target.contains("windows") {
        build
            .flag("-std=c++11")
            .flag("-fno-exceptions")
            .flag("-fno-rtti");
    }

    build.compile("signalhandlers");
}
