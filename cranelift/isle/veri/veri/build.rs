use std::env;

fn write_build_meta() {
    let out_dir = env::var("OUT_DIR").expect("The OUT_DIR environment variable must be set");

    // Profile: debug, release, ...
    let build_profile = env::var("PROFILE").expect("The PROFILE environment variable must be set");

    // Git hash
    let output = std::process::Command::new("git")
        .arg("describe")
        .arg("--always")
        .arg("--dirty")
        .arg("--abbrev=12")
        .arg("--exclude")
        .arg("*")
        .output()
        .expect("Failed to execute git describe");
    let output = String::from_utf8(output.stdout).unwrap();
    let git_version = output.trim();

    std::fs::write(
        std::path::Path::new(&out_dir).join("meta.rs"),
        format!(
            "pub const BUILD_PROFILE: &str = \"{build_profile}\";\n\
            pub const GIT_VERSION: &str = \"{git_version}\";\n\
            "
        ),
    )
    .unwrap();
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=filetests");

    write_build_meta();
}
