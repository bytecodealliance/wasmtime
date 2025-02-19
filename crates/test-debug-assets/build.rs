use std::{env, ffi::OsString, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    build_wasm_assets();
}

fn build_wasm_assets() {
    const ASSETS_REL_SRC_DIR: &'static str = "../../tests/all/debug/testsuite";
    println!("cargo:rerun-if-changed={ASSETS_REL_SRC_DIR}");

    // There are three types of assets at this time:
    // 1. Binary - we use them as-is from the source directory.
    //    They have the .wasm extension.
    // 2. C/C++ source - we compile them below.
    // 3. Explanatory - things like WAT for a binary we don't
    //    know how to compile (yet). They are ignored.
    //
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let assets_src_dir = fs::canonicalize(ASSETS_REL_SRC_DIR).unwrap();
    let binary_assets = [
        "dead_code.wasm",
        "dwarf_fission.wasm",
        "fib-wasm-dwarf5.wasm",
        "fib-wasm-split4.wasm",
        "fib-wasm.wasm",
        "fraction-norm.wasm",
        "reverse-str.wasm",
        "spilled_frame_base.wasm",
        "two_removed_branches.wasm",
    ];
    let mut paths_code = String::new();
    for asset in binary_assets {
        let (_, path_code) = get_asset_path(&assets_src_dir, asset);
        paths_code += &path_code;
    }

    // Compile the C/C++ assets.
    let compile_commands = [(
        "clang",
        "generic.wasm",
        [
            "-target",
            "wasm32-unknown-wasip1",
            "-g",
            "generic.cpp",
            "generic-satellite.cpp",
        ],
    )];

    // The debug tests relying on these assets are ignored by default,
    // so we cannot force the requirement of having a working WASI SDK
    // install on everyone. At the same time, those tests (due to their
    // monolithic nature), are always compiled, so we still have to
    // produce the path constants. To solve this, we move the failure
    // of missing WASI SDK from compile time to runtime by producing
    // fake paths (that themselves will serve as diagnostic messages).
    let wasi_sdk_bin_path = env::var_os("WASI_SDK_PATH").map(|p| PathBuf::from(p).join("bin"));
    let missing_sdk_path =
        PathBuf::from("Asset not compiled, WASI_SDK_PATH missing at compile time");
    let out_arg = OsString::from("-o");

    for (compiler, asset, args) in compile_commands {
        if let Some(compiler_dir) = &wasi_sdk_bin_path {
            let (out_path, path_code) = get_asset_path(&out_dir, asset);

            let mut command = Command::new(compiler_dir.join(compiler));
            let output = command
                .current_dir(&assets_src_dir)
                .args([&out_arg, out_path.as_os_str()])
                .args(args)
                .output();
            if !output.as_ref().is_ok_and(|o| o.status.success()) {
                panic!("{command:?}: {output:?}");
            }

            paths_code += &path_code;
        } else {
            let (_, path_code) = get_asset_path(&missing_sdk_path, asset);
            paths_code += &path_code;
        }
    }

    std::fs::write(out_dir.join("gen.rs"), paths_code).unwrap();
}

fn get_asset_path(dir: &PathBuf, asset: &str) -> (PathBuf, String) {
    let mut name = asset.replace("-", "_").replace(".", "_");
    name = name.to_uppercase();
    let out_path = dir.join(asset);
    let out_path_code = format!("pub const {name}_PATH: &'static str = {out_path:?};\n");
    (out_path, out_path_code)
}
