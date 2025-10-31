use std::process::Command;

fn main() {
    let url = "https://raw.githubusercontent.com/ua-parser/uap-core/f21592418f6323f9ce32f10e231841cf8e782b43/regexes.yaml";
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = std::path::Path::new(&out_dir).join("regexes.yaml");

    let status = Command::new("curl")
        .args(&["-L", url, "--retry", "3"])
        .arg("-o")
        .arg(dest_path)
        .status()
        .expect("failed to execute process");
    assert!(status.success());
}
