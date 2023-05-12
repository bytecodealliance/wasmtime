use std::process::Command;
fn main() {
    let output = Command::new("git")
        .arg("for-each-ref")
        .arg("refs/remotes/origin")
        .arg("--format")
        .arg("%(refname)")
        .output()
        .unwrap();
    assert!(output.status.success());
    let mut releases = std::str::from_utf8(&output.stdout)
        .unwrap()
        .lines()
        .filter_map(|l| l.strip_prefix("refs/remotes/origin/release-"))
        .filter_map(|l| {
            let mut parts = l.split('.');
            let major = parts.next()?.parse::<u32>().ok()?;
            let minor = parts.next()?.parse::<u32>().ok()?;
            let patch = parts.next()?.parse::<u32>().ok()?;
            Some((major, minor, patch))
        })
        .collect::<Vec<_>>();
    releases.sort();
    let (major, minor, patch) = releases.last().unwrap();
    println!("{}.{}.{}", major, minor, patch);
}
