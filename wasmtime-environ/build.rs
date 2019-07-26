use std::process::Command;

fn main() {
    let git_rev = match Command::new("git").args(&["rev-parse", "HEAD"]).output() {
        Ok(output) => String::from_utf8(output.stdout).unwrap(),
        Err(_) => String::from("git-not-found"),
    };
    println!("cargo:rustc-env=GIT_REV={}", git_rev);
}
