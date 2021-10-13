//! Helper script to publish the wasmtime and cranelift suites of crates
//!
//! See documentation in `docs/contributing-release-process.md` for more
//! information, but in a nutshell:
//!
//! * `./publish bump` - bump crate versions in-tree
//! * `./publish verify` - verify crates can be published to crates.io
//! * `./publish publish` - actually publish crates to crates.io

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

// note that this list must be topologically sorted by dependencies
const CRATES_TO_PUBLISH: &[&str] = &[
    // peepmatic
    "peepmatic-traits",
    "peepmatic-macro",
    "peepmatic-automata",
    "peepmatic-test-operator",
    "peepmatic-runtime",
    "peepmatic",
    "peepmatic-souper",
    // cranelift
    "isle",
    "cranelift-entity",
    "wasmtime-types",
    "cranelift-bforest",
    "cranelift-codegen-shared",
    "cranelift-codegen-meta",
    "cranelift-codegen",
    "cranelift-reader",
    "cranelift-serde",
    "cranelift-module",
    "cranelift-preopt",
    "cranelift-frontend",
    "cranelift-wasm",
    "cranelift-native",
    "cranelift-object",
    "cranelift-interpreter",
    "cranelift",
    "cranelift-jit",
    // wiggle
    "wiggle-generate",
    "wiggle-macro",
    // wasmtime
    "wasmtime-fiber",
    "wasmtime-environ",
    "wasmtime-runtime",
    "wasmtime-cranelift",
    "wasmtime-jit",
    "wasmtime-cache",
    "wasmtime",
    // wasi-common/wiggle
    "wiggle",
    "wasi-common",
    "wasi-cap-std-sync",
    "wasi-tokio",
    // other mic wasmtime crates
    "wasmtime-wasi",
    "wasmtime-wasi-nn",
    "wasmtime-wasi-crypto",
    "wasmtime-wast",
    "wasmtime-cli",
];

// Anything **not** mentioned in this array is required to have an `=a.b.c`
// dependency requirement on it to enable breaking api changes even in "patch"
// releases since everything not mentioned here is just an organizational detail
// that no one else should rely on.
const PUBLIC_CRATES: &[&str] = &[
    // just here to appease the script because these are submodules of this
    // repository.
    "wasi-crypto",
    "witx",
    // these are actually public crates which we cannot break the API of in
    // patch releases.
    "wasmtime",
    "wasmtime-wasi",
    "wasmtime-wasi-nn",
    "wasmtime-wasi-crypto",
    "wasmtime-cli",
    // all cranelift crates are considered "public" in that they can't
    // have breaking API changes in patch releases
    "cranelift-entity",
    "cranelift-bforest",
    "cranelift-codegen-shared",
    "cranelift-codegen-meta",
    "cranelift-codegen",
    "cranelift-reader",
    "cranelift-serde",
    "cranelift-module",
    "cranelift-preopt",
    "cranelift-frontend",
    "cranelift-wasm",
    "cranelift-native",
    "cranelift-object",
    "cranelift-interpreter",
    "cranelift",
    "cranelift-jit",
    // This is a dependency of cranelift crates and as a result can't break in
    // patch releases as well
    "wasmtime-types",
];

struct Crate {
    manifest: PathBuf,
    name: String,
    version: String,
    publish: bool,
}

fn main() {
    let mut crates = Vec::new();
    crates.push(read_crate("./Cargo.toml".as_ref()));
    find_crates("crates".as_ref(), &mut crates);
    find_crates("cranelift".as_ref(), &mut crates);

    let pos = CRATES_TO_PUBLISH
        .iter()
        .enumerate()
        .map(|(i, c)| (*c, i))
        .collect::<HashMap<_, _>>();
    crates.sort_by_key(|krate| pos.get(&krate.name[..]));

    match &env::args().nth(1).expect("must have one argument")[..] {
        name @ "bump" | name @ "bump-patch" => {
            for krate in crates.iter() {
                bump_version(&krate, &crates, name == "bump-patch");
            }
            // update the lock file
            assert!(Command::new("cargo")
                .arg("fetch")
                .status()
                .unwrap()
                .success());
        }

        "publish" => {
            // We have so many crates to publish we're frequently either
            // rate-limited or we run into issues where crates can't publish
            // successfully because they're waiting on the index entries of
            // previously-published crates to propagate. This means we try to
            // publish in a loop and we remove crates once they're successfully
            // published. Failed-to-publish crates get enqueued for another try
            // later on.
            for _ in 0..5 {
                crates.retain(|krate| !publish(krate));

                if crates.is_empty() {
                    break;
                }

                println!(
                    "{} crates failed to publish, waiting for a bit to retry",
                    crates.len(),
                );
                thread::sleep(Duration::from_secs(20));
            }

            assert!(crates.is_empty(), "failed to publish all crates");

            println!("");
            println!("===================================================================");
            println!("");
            println!("Don't forget to push a git tag for this release!");
            println!("");
            println!("    $ git tag vX.Y.Z");
            println!("    $ git push git@github.com:bytecodealliance/wasmtime.git vX.Y.Z");
        }

        "verify" => {
            verify(&crates);
        }

        s => panic!("unknown command: {}", s),
    }
}

fn find_crates(dir: &Path, dst: &mut Vec<Crate>) {
    if dir.join("Cargo.toml").exists() {
        let krate = read_crate(&dir.join("Cargo.toml"));
        if !krate.publish || CRATES_TO_PUBLISH.iter().any(|c| krate.name == *c) {
            dst.push(krate);
        } else {
            panic!("failed to find {:?} in whitelist or blacklist", krate.name);
        }
    }

    for entry in dir.read_dir().unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            find_crates(&entry.path(), dst);
        }
    }
}

fn read_crate(manifest: &Path) -> Crate {
    let mut name = None;
    let mut version = None;
    let mut publish = true;
    for line in fs::read_to_string(manifest).unwrap().lines() {
        if name.is_none() && line.starts_with("name = \"") {
            name = Some(
                line.replace("name = \"", "")
                    .replace("\"", "")
                    .trim()
                    .to_string(),
            );
        }
        if version.is_none() && line.starts_with("version = \"") {
            version = Some(
                line.replace("version = \"", "")
                    .replace("\"", "")
                    .trim()
                    .to_string(),
            );
        }
        if line.starts_with("publish = false") {
            publish = false;
        }
    }
    let name = name.unwrap();
    let version = version.unwrap();
    if ["witx", "witx-cli", "wasi-crypto"].contains(&&name[..]) {
        publish = false;
    }
    Crate {
        manifest: manifest.to_path_buf(),
        name,
        version,
        publish,
    }
}

fn bump_version(krate: &Crate, crates: &[Crate], patch: bool) {
    let contents = fs::read_to_string(&krate.manifest).unwrap();
    let next_version = |krate: &Crate| -> String {
        if CRATES_TO_PUBLISH.contains(&&krate.name[..]) {
            bump(&krate.version, patch)
        } else {
            krate.version.clone()
        }
    };

    let mut new_manifest = String::new();
    let mut is_deps = false;
    for line in contents.lines() {
        let mut rewritten = false;
        if !is_deps && line.starts_with("version =") {
            if CRATES_TO_PUBLISH.contains(&&krate.name[..]) {
                println!(
                    "bump `{}` {} => {}",
                    krate.name,
                    krate.version,
                    next_version(krate),
                );
                new_manifest.push_str(&line.replace(&krate.version, &next_version(krate)));
                rewritten = true;
            }
        }

        is_deps = if line.starts_with("[") {
            line.contains("dependencies")
        } else {
            is_deps
        };

        for other in crates {
            // If `other` isn't a published crate then it's not going to get a
            // bumped version so we don't need to update anything in the
            // manifest.
            if !other.publish {
                continue;
            }
            if !is_deps || !line.starts_with(&format!("{} ", other.name)) {
                continue;
            }
            if !line.contains(&other.version) {
                if !line.contains("version =") || !krate.publish {
                    continue;
                }
                panic!(
                    "{:?} has a dep on {} but doesn't list version {}",
                    krate.manifest, other.name, other.version
                );
            }
            if krate.publish {
                if PUBLIC_CRATES.contains(&other.name.as_str()) {
                    assert!(
                        !line.contains("\"="),
                        "{} should not have an exact version requirement on {}",
                        krate.name,
                        other.name
                    );
                } else {
                    assert!(
                        line.contains("\"="),
                        "{} should have an exact version requirement on {}",
                        krate.name,
                        other.name
                    );
                }
            }
            rewritten = true;
            new_manifest.push_str(&line.replace(&other.version, &next_version(other)));
            break;
        }
        if !rewritten {
            new_manifest.push_str(line);
        }
        new_manifest.push_str("\n");
    }
    fs::write(&krate.manifest, new_manifest).unwrap();
}

/// Performs a major version bump increment on the semver version `version`.
///
/// This function will perform a semver-major-version bump on the `version`
/// specified. This is used to calculate the next version of a crate in this
/// repository since we're currently making major version bumps for all our
/// releases. This may end up getting tweaked as we stabilize crates and start
/// doing more minor/patch releases, but for now this should do the trick.
fn bump(version: &str, patch_bump: bool) -> String {
    let mut iter = version.split('.').map(|s| s.parse::<u32>().unwrap());
    let major = iter.next().expect("major version");
    let minor = iter.next().expect("minor version");
    let patch = iter.next().expect("patch version");

    if patch_bump {
        return format!("{}.{}.{}", major, minor, patch + 1);
    }
    if major != 0 {
        format!("{}.0.0", major + 1)
    } else if minor != 0 {
        format!("0.{}.0", minor + 1)
    } else {
        format!("0.0.{}", patch + 1)
    }
}

fn publish(krate: &Crate) -> bool {
    if !CRATES_TO_PUBLISH.iter().any(|s| *s == krate.name) {
        return true;
    }

    // First make sure the crate isn't already published at this version. This
    // script may be re-run and there's no need to re-attempt previous work.
    let output = Command::new("curl")
        .arg(&format!("https://crates.io/api/v1/crates/{}", krate.name))
        .output()
        .expect("failed to invoke `curl`");
    if output.status.success()
        && String::from_utf8_lossy(&output.stdout)
            .contains(&format!("\"newest_version\":\"{}\"", krate.version))
    {
        println!(
            "skip publish {} because {} is latest version",
            krate.name, krate.version,
        );
        return true;
    }

    let status = Command::new("cargo")
        .arg("publish")
        .current_dir(krate.manifest.parent().unwrap())
        .arg("--no-verify")
        .status()
        .expect("failed to run cargo");
    if !status.success() {
        println!("FAIL: failed to publish `{}`: {}", krate.name, status);
        return false;
    }

    // After we've published then make sure that the `wasmtime-publish` group is
    // added to this crate for future publications. If it's already present
    // though we can skip the `cargo owner` modification.
    let output = Command::new("curl")
        .arg(&format!(
            "https://crates.io/api/v1/crates/{}/owners",
            krate.name
        ))
        .output()
        .expect("failed to invoke `curl`");
    if output.status.success()
        && String::from_utf8_lossy(&output.stdout).contains("wasmtime-publish")
    {
        println!(
            "wasmtime-publish already listed as an owner of {}",
            krate.name
        );
        return true;
    }

    // Note that the status is ignored here. This fails most of the time because
    // the owner is already set and present, so we only want to add this to
    // crates which haven't previously been published.
    let status = Command::new("cargo")
        .arg("owner")
        .arg("-a")
        .arg("github:bytecodealliance:wasmtime-publish")
        .arg(&krate.name)
        .status()
        .expect("failed to run cargo");
    if !status.success() {
        panic!(
            "FAIL: failed to add wasmtime-publish as owner `{}`: {}",
            krate.name, status
        );
    }

    true
}

// Verify the current tree is publish-able to crates.io. The intention here is
// that we'll run `cargo package` on everything which verifies the build as-if
// it were published to crates.io. This requires using an incrementally-built
// directory registry generated from `cargo vendor` because the versions
// referenced from `Cargo.toml` may not exist on crates.io.
fn verify(crates: &[Crate]) {
    drop(fs::remove_dir_all(".cargo"));
    drop(fs::remove_dir_all("vendor"));
    let vendor = Command::new("cargo")
        .arg("vendor")
        .stderr(Stdio::inherit())
        .output()
        .unwrap();
    assert!(vendor.status.success());

    fs::create_dir_all(".cargo").unwrap();
    fs::write(".cargo/config.toml", vendor.stdout).unwrap();

    // Vendor witx which wasn't vendored because it's a path dependency, but
    // it'll need to be in our directory registry for crates that depend on it.
    let witx = crates
        .iter()
        .find(|c| c.name == "witx" && c.manifest.iter().any(|p| p == "wasi-common"))
        .unwrap();
    verify_and_vendor(&witx);

    // Vendor wasi-crypto which is also a path dependency
    let wasi_crypto = crates.iter().find(|c| c.name == "wasi-crypto").unwrap();
    verify_and_vendor(&wasi_crypto);

    for krate in crates {
        if !krate.publish {
            continue;
        }
        verify_and_vendor(&krate);
    }

    fn verify_and_vendor(krate: &Crate) {
        let mut cmd = Command::new("cargo");
        cmd.arg("package")
            .arg("--manifest-path")
            .arg(&krate.manifest)
            .env("CARGO_TARGET_DIR", "./target");
        if krate.name == "witx"
            || krate.name.contains("wasi-nn")
            || krate.name.contains("peepmatic")
        {
            cmd.arg("--no-verify");
        }
        let status = cmd.status().unwrap();
        assert!(status.success(), "failed to verify {:?}", &krate.manifest);
        let tar = Command::new("tar")
            .arg("xf")
            .arg(format!(
                "../target/package/{}-{}.crate",
                krate.name, krate.version
            ))
            .current_dir("./vendor")
            .status()
            .unwrap();
        assert!(tar.success());
        fs::write(
            format!(
                "./vendor/{}-{}/.cargo-checksum.json",
                krate.name, krate.version
            ),
            "{\"files\":{}}",
        )
        .unwrap();
    }
}
