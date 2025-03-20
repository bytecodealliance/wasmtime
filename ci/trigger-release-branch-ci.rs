//! Helper script used by `.github/workflows/ci-cron-trigger.yml`

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

    let mut to_trigger: Vec<(u32, u32, u32)> = Vec::new();
    let mut iter = releases.iter().rev();

    // Pick the latest 3 release branches to keep up-to-date. Although we
    // only promise the last 2 are going to be released with security fixes when
    // a new release branch is made that means there's one "pending" release
    // branch and two "active" release branches. In that situation we want to
    // update 3 branches. If there's no "pending" branch then we'll just be
    // keeping some older branch's CI working, which shouldn't be too hard.
    to_trigger.extend(iter.by_ref().take(3));

    // We support two LTS channels 12 versions apart. If one is already included
    // in the above set of 3 latest releases, however, then we're only picking
    // one historical LTS release.
    let mut lts_channels = 2;
    if to_trigger.iter().any(|(major, _, _)| *major % 12 == 0) {
        lts_channels -= 1;
    }

    // Look for LTS releases, defined by every-12-versions which are after v24.
    to_trigger.extend(
        iter.filter(|(major, _, _)| *major % 12 == 0 && *major > 20)
            .take(lts_channels),
    );

    println!("{to_trigger:?}");

    for (major, minor, patch) in to_trigger {
        dbg!(major, minor, patch);
        let status = Command::new("gh")
            .arg("workflow")
            .arg("run")
            .arg("main.yml")
            .arg("--ref")
            .arg(format!("release-{major}.{minor}.{patch}"))
            .status()
            .unwrap();
        assert!(status.success());
    }
}
