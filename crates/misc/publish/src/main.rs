//! Helper tool to publish the wasmtime and cranelift suites of crates.
//!
//! See documentation in `docs/contributing-release-process.md` for more
//! information, but in a nutshell:
//!
//! * `cargo run -p wasmtime-publish bump-major` - bump versions to the next `-dev`
//! * `cargo run -p wasmtime-publish bump-patch` - bump the patch number
//! * `cargo run -p wasmtime-publish bump-rc` - move to the next release candidate
//! * `cargo run -p wasmtime-publish bump-drop-rc` - strip the pre-release suffix
//! * `cargo run -p wasmtime-publish verify` - verify crates can be published to crates.io
//! * `cargo run -p wasmtime-publish publish` - actually publish crates to crates.io
//! * `cargo run -p wasmtime-publish yank-old-rcs` - yank superseded release candidates
//! * `cargo run -p wasmtime-publish latest-release` - print the most recent release branch's version

use semver::{Prerelease, Version};
use serde_derive::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Output, Stdio};
use std::time::Duration;
use std::{env, fs, thread};
use toml_edit::{DocumentMut, Item};

// note that this list must be topologically sorted by dependencies
const CRATES_TO_PUBLISH: &[&str] = &[
    "wasmtime-internal-core",
    "cranelift-bitset",
    // pulley
    "pulley-macros",
    "pulley-interpreter",
    // cranelift
    "cranelift-srcgen",
    "cranelift-assembler-x64-meta",
    "cranelift-assembler-x64",
    "cranelift-isle",
    "cranelift-entity",
    "cranelift-bforest",
    "cranelift-codegen-shared",
    "cranelift-codegen-meta",
    "cranelift-control",
    "cranelift-codegen",
    "cranelift-reader",
    "cranelift-serde",
    "cranelift-module",
    "cranelift-frontend",
    "cranelift-native",
    "cranelift-object",
    "cranelift-interpreter",
    "wasmtime-internal-component-util",
    "wasmtime-environ",
    "wasmtime-internal-jit-icache-coherence",
    // Wasmtime unwinder, used by both `cranelift-jit` (optionally) and filetests, and by Wasmtime.
    "wasmtime-internal-unwinder",
    // Cranelift crates that use Wasmtime unwinder.
    "cranelift-jit",
    "cranelift",
    // wiggle
    "wiggle-generate",
    "wiggle-macro",
    // wasmtime
    "wasmtime-internal-versioned-export-macros",
    "wasmtime-internal-wit-bindgen",
    "wasmtime-internal-component-macro",
    "wasmtime-internal-jit-debug",
    "wasmtime-internal-fiber",
    "wasmtime-internal-wmemcheck",
    "wasmtime-internal-cranelift",
    "wasmtime-internal-cache",
    "winch-codegen",
    "wasmtime-internal-winch",
    "wasmtime",
    "wiggle",
    // other misc wasmtime crates
    "wasmtime-wasi-io",
    "wasmtime-wasi",
    "wasmtime-wasi-http",
    "wasmtime-wasi-nn",
    "wasmtime-wasi-config",
    "wasmtime-wasi-keyvalue",
    "wasmtime-wasi-threads",
    "wasmtime-wasi-tls",
    "wasmtime-wast",
    "wasmtime-internal-c-api-macros",
    "wasmtime-c-api-impl",
    "wasmtime-wizer",
    "wasmtime-cli-flags",
    "wasmtime-internal-explorer",
    "wasmtime-internal-debugger",
    "wasmtime-internal-gdbstub-component-artifact",
    "wasmtime-cli",
];

// Anything **not** mentioned in this array is required to have an `=a.b.c`
// dependency requirement on it to enable breaking api changes even in "patch"
// releases since everything not mentioned here is just an organizational detail
// that no one else should rely on.
const PUBLIC_CRATES: &[&str] = &[
    // These are actually public crates which we cannot break the API of in
    // patch releases.
    "wasmtime",
    "wasmtime-wasi-io",
    "wasmtime-wasi",
    "wasmtime-wasi-tls",
    "wasmtime-wasi-http",
    "wasmtime-wasi-nn",
    "wasmtime-wasi-config",
    "wasmtime-wasi-keyvalue",
    "wasmtime-wasi-threads",
    "wasmtime-cli",
    "wasmtime-wizer",
    // All cranelift crates are considered "public" in that they can't have
    // breaking API changes in patch releases.
    "cranelift-srcgen",
    "cranelift-assembler-x64-meta",
    "cranelift-assembler-x64",
    "cranelift-entity",
    "cranelift-bforest",
    "cranelift-bitset",
    "cranelift-codegen-shared",
    "cranelift-codegen-meta",
    "cranelift-egraph",
    "cranelift-control",
    "cranelift-codegen",
    "cranelift-reader",
    "cranelift-serde",
    "cranelift-module",
    "cranelift-frontend",
    "cranelift-native",
    "cranelift-object",
    "cranelift-interpreter",
    "cranelift",
    "cranelift-jit",
    // This is a dependency of cranelift crates and as a result can't break in
    // patch releases as well
    "wasmtime-types",
];

const C_HEADER_PATH: &str = "./crates/c-api/include/wasmtime.h";

/// A single first-party crate discovered in the tree.
struct Crate {
    manifest: PathBuf,
    name: String,
    version: Version,
    publish: bool,
}

/// The version-rewriting operations that this tool knows how to perform.
#[derive(Copy, Clone)]
enum BumpOp {
    Major,
    Patch,
    Rc,
    DropRc,
}

fn main() {
    let root = read_doc(Path::new("./Cargo.toml"));
    let ws_version = workspace_version(&root);

    let mut crates = Vec::new();
    crates.push(read_crate(&ws_version, Path::new("./Cargo.toml")).expect("root is a crate"));
    find_crates(Path::new("crates"), &ws_version, &mut crates);
    find_crates(Path::new("cranelift"), &ws_version, &mut crates);
    find_crates(Path::new("pulley"), &ws_version, &mut crates);
    find_crates(Path::new("winch"), &ws_version, &mut crates);

    let pos = CRATES_TO_PUBLISH
        .iter()
        .enumerate()
        .map(|(i, c)| (*c, i))
        .collect::<HashMap<_, _>>();
    crates.sort_by_key(|krate| pos.get(&krate.name[..]));

    match &env::args().nth(1).expect("must have one argument")[..] {
        name @ ("bump-major" | "bump-patch" | "bump-rc" | "bump-drop-rc") => {
            let op = match name {
                "bump-major" => BumpOp::Major,
                "bump-patch" => BumpOp::Patch,
                "bump-rc" => BumpOp::Rc,
                "bump-drop-rc" => BumpOp::DropRc,
                _ => unreachable!(),
            };

            let next: HashMap<&str, Version> = crates
                .iter()
                .filter(|k| k.publish)
                .map(|k| (k.name.as_str(), bump(&k.version, op)))
                .collect();

            for krate in crates.iter() {
                bump_version(krate, &crates, &next);
            }
            // update C API version in wasmtime.h
            update_capi_version();
            // update the lock file
            run_cmd(Command::new("cargo").arg("fetch"));
        }

        "publish" => {
            // We have so many crates to publish we're frequently either
            // rate-limited or we run into issues where crates can't publish
            // successfully because they're waiting on the index entries of
            // previously-published crates to propagate. This means we try to
            // publish in a loop and we remove crates once they're successfully
            // published. Failed-to-publish crates get enqueued for another try
            // later on.
            for _ in 0..10 {
                crates.retain(|krate| !publish(krate));

                if crates.is_empty() {
                    break;
                }

                println!(
                    "{} crates failed to publish, waiting for a bit to retry",
                    crates.len(),
                );
                thread::sleep(Duration::from_secs(40));
            }

            assert!(crates.is_empty(), "failed to publish all crates");

            println!();
            println!("===================================================================");
            println!();
            println!("Don't forget to push a git tag for this release!");
            println!();
            println!("    $ git tag vX.Y.Z");
            println!("    $ git push git@github.com:bytecodealliance/wasmtime.git vX.Y.Z");
        }

        "yank-old-rcs" => {
            assert!(
                yank_old_rcs(&crates),
                "failed to yank all superseded release candidates"
            );
        }

        "verify" => {
            verify(&crates);
        }

        "latest-release" => {
            println!("{}", latest_release_branch());
        }

        s => panic!("unknown command: {s}"),
    }
}

fn cmd_output(cmd: &mut Command) -> Output {
    eprintln!("Running: `{cmd:?}`");
    match cmd.output() {
        Ok(o) => o,
        Err(e) => panic!("Failed to run `{cmd:?}`: {e}"),
    }
}

fn cmd_status(cmd: &mut Command) -> ExitStatus {
    eprintln!("Running: `{cmd:?}`");
    match cmd.status() {
        Ok(s) => s,
        Err(e) => panic!("Failed to run `{cmd:?}`: {e}"),
    }
}

fn run_cmd(cmd: &mut Command) {
    let status = cmd_status(cmd);
    assert!(
        status.success(),
        "Command `{cmd:?}` exited with failure status: {status}"
    );
}

/// Parses the `Cargo.toml` at `path` into an editable document.
fn read_doc(path: &Path) -> DocumentMut {
    fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
        .parse::<DocumentMut>()
        .unwrap_or_else(|e| panic!("failed to parse {}: {e}", path.display()))
}

/// Reads the `[workspace.package] version` from the root manifest.
fn workspace_version(root: &DocumentMut) -> Version {
    let version = root["workspace"]["package"]["version"]
        .as_str()
        .expect("workspace version");
    parse_version(version)
}

fn parse_version(version: &str) -> Version {
    Version::parse(version).unwrap_or_else(|e| panic!("failed to parse version `{version}`: {e}"))
}

fn find_crates(dir: &Path, ws_version: &Version, dst: &mut Vec<Crate>) {
    if dir.join("Cargo.toml").exists() {
        if let Some(krate) = read_crate(ws_version, &dir.join("Cargo.toml")) {
            dst.push(krate);
        }
    }

    for entry in dir.read_dir().unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            find_crates(&entry.path(), ws_version, dst);
        }
    }
}

/// Reads the package name, version, and publish flag from a single manifest.
///
/// Returns `None` for virtual manifests (those without a `[package]` section),
/// which have nothing to publish. The version is taken from the manifest's own
/// `[package] version` when it's a literal (as with the cranelift crates on
/// their `0.x` version line) and otherwise inherited from the workspace.
fn read_crate(ws_version: &Version, manifest: &Path) -> Option<Crate> {
    let doc = read_doc(manifest);
    let package = doc.get("package")?.as_table_like()?;
    let name = package
        .get("name")
        .and_then(|i| i.as_str())
        .unwrap_or_else(|| panic!("{} has no package name", manifest.display()))
        .to_string();
    let version = match package.get("version").and_then(|i| i.as_str()) {
        Some(v) => parse_version(v),
        None => ws_version.clone(),
    };
    let publish = package
        .get("publish")
        .and_then(|i| i.as_bool())
        .unwrap_or(true);

    assert!(
        !publish || CRATES_TO_PUBLISH.contains(&name.as_str()),
        "{name} must either be listed in `CRATES_TO_PUBLISH` or set `publish = false`",
    );

    Some(Crate {
        manifest: manifest.to_path_buf(),
        name,
        version,
        publish,
    })
}

/// Rewrites a single crate's manifest: its own version and any dependency
/// requirements it places on other first-party crates.
fn bump_version(krate: &Crate, crates: &[Crate], next: &HashMap<&str, Version>) {
    println!("bumping `{}`...", krate.name);
    let mut doc = read_doc(&krate.manifest);

    // Rewrite the crate's own version. Note that `next` only contains published
    // crates, so `publish = false` crates are left alone.
    if let Some(new) = next.get(krate.name.as_str()) {
        println!("  {} => {new}", krate.version);
        set_own_version(&mut doc, new);
    }

    // Rewrite dependency requirements on other first-party crates across every
    // dependency table in the manifest (including `[workspace.dependencies]`).
    for_each_dep_table(&mut doc, |table| {
        let keys = table.iter().map(|(k, _)| k.to_string()).collect::<Vec<_>>();
        for key in keys {
            let item = table.get_mut(&key).unwrap();
            let Some(dep) = item.as_table_like_mut() else {
                continue;
            };
            let dep_name = dep
                .get("package")
                .and_then(|i| i.as_str())
                .unwrap_or(&key)
                .to_string();
            let Some(new) = next.get(dep_name.as_str()) else {
                continue;
            };
            let Some(other) = crates.iter().find(|c| c.name == dep_name) else {
                continue;
            };
            let Some(req_item) = dep.get_mut("version") else {
                continue;
            };
            let req = req_item.as_str().unwrap().to_string();
            let exact = req.strip_prefix('=');
            let base = exact.unwrap_or(&req);
            assert_eq!(
                base,
                other.version.to_string(),
                "{}: dependency on {dep_name} lists {base} but that crate is at {}",
                krate.manifest.display(),
                other.version,
            );

            // Public crates must use a caret requirement (so patch releases can
            // include breaking changes for everything else); internal crates
            // must be pinned exactly.
            if PUBLIC_CRATES.contains(&dep_name.as_str()) {
                assert!(
                    exact.is_none(),
                    "{} should not have an exact version requirement on {dep_name}",
                    krate.name,
                );
            } else {
                assert!(
                    exact.is_some(),
                    "{} should have an exact version requirement on {dep_name}",
                    krate.name,
                );
            }

            let new_req = if exact.is_some() {
                format!("={new}")
            } else {
                new.to_string()
            };
            set_version_str(req_item, &new_req);
        }
    });

    fs::write(&krate.manifest, doc.to_string()).unwrap();
}

fn set_own_version(doc: &mut DocumentMut, new: &Version) {
    if let Some(item) = doc
        .get_mut("package")
        .and_then(Item::as_table_like_mut)
        .and_then(|t| t.get_mut("version"))
    {
        if item.is_str() {
            set_version_str(item, &new.to_string());
            return;
        }
    }

    if let Some(item) = doc
        .get_mut("workspace")
        .and_then(Item::as_table_like_mut)
        .and_then(|t| t.get_mut("package"))
        .and_then(Item::as_table_like_mut)
        .and_then(|t| t.get_mut("version"))
    {
        set_version_str(item, &new.to_string());
    }
}

fn set_version_str(item: &mut Item, new: &str) {
    let decor = item.as_value().map(|v| v.decor().clone());
    *item = toml_edit::value(new.to_string());
    if let (Some(decor), Some(value)) = (decor, item.as_value_mut()) {
        *value.decor_mut() = decor;
    }
}

fn for_each_dep_table(doc: &mut DocumentMut, mut f: impl FnMut(&mut dyn toml_edit::TableLike)) {
    const KINDS: &[&str] = &["dependencies", "dev-dependencies", "build-dependencies"];

    for kind in KINDS {
        if let Some(table) = doc.get_mut(kind).and_then(Item::as_table_like_mut) {
            f(table);
        }
    }

    // `[workspace.dependencies]`
    if let Some(deps) = doc
        .get_mut("workspace")
        .and_then(Item::as_table_like_mut)
        .and_then(|t| t.get_mut("dependencies"))
        .and_then(Item::as_table_like_mut)
    {
        f(deps);
    }

    // `[target.'cfg(..)'.dependencies]` and friends
    if let Some(target) = doc.get_mut("target").and_then(Item::as_table_like_mut) {
        let cfgs = target
            .iter()
            .map(|(k, _)| k.to_string())
            .collect::<Vec<_>>();
        for cfg in cfgs {
            let Some(cfg) = target.get_mut(&cfg).and_then(Item::as_table_like_mut) else {
                continue;
            };
            for kind in KINDS {
                if let Some(table) = cfg.get_mut(kind).and_then(Item::as_table_like_mut) {
                    f(table);
                }
            }
        }
    }
}

fn update_capi_version() {
    let version = workspace_version(&read_doc(Path::new("./Cargo.toml")));

    // Note that the `WASMTIME_VERSION` string carries the full version,
    // pre-release suffix included, while the numeric macros only describe the
    // `major.minor.patch` triple.
    let Version {
        major,
        minor,
        patch,
        ..
    } = version;

    let mut new_header = String::new();
    let contents = fs::read_to_string(C_HEADER_PATH).unwrap();
    for line in contents.lines() {
        if line.starts_with("#define WASMTIME_VERSION \"") {
            new_header.push_str(&format!("#define WASMTIME_VERSION \"{version}\""));
        } else if line.starts_with("#define WASMTIME_VERSION_MAJOR") {
            new_header.push_str(&format!("#define WASMTIME_VERSION_MAJOR {major}"));
        } else if line.starts_with("#define WASMTIME_VERSION_MINOR") {
            new_header.push_str(&format!("#define WASMTIME_VERSION_MINOR {minor}"));
        } else if line.starts_with("#define WASMTIME_VERSION_PATCH") {
            new_header.push_str(&format!("#define WASMTIME_VERSION_PATCH {patch}"));
        } else {
            new_header.push_str(line);
        }
        new_header.push('\n');
    }

    fs::write(C_HEADER_PATH, new_header).unwrap();
}

/// Computes the next version of `version` under the operation `op`.
fn bump(version: &Version, op: BumpOp) -> Version {
    let mut next = version.clone();
    match op {
        // A patch bump leaves any pre-release suffix alone. This keeps the
        // operation monotonic and means that CI's "does `cargo vet` still work
        // if versions are bumped" check behaves sensibly on `main` (`-dev`) and
        // on a release branch sitting at `-rc.N`.
        BumpOp::Patch => next.patch += 1,

        // Development on `main` always happens on a `-dev` version so that it's
        // clear the version is unreleased and unreleasable.
        BumpOp::Major => {
            if next.major != 0 {
                next.major += 1;
                next.minor = 0;
                next.patch = 0;
            } else {
                assert!(next.minor != 0);
                next.minor += 1;
                next.patch = 0;
            }
            next.pre = Prerelease::new("dev").unwrap();
        }

        // `-dev` (and a plain release) start the release-candidate sequence over
        // at 1, and each subsequent candidate increments from there.
        BumpOp::Rc => {
            let n = rc_number(version).map_or(1, |n| n + 1);
            next.pre = Prerelease::new(&format!("rc.{n}")).unwrap();
        }

        // Releasing simply drops whatever pre-release suffix is present, and is
        // a no-op if there isn't one.
        BumpOp::DropRc => next.pre = Prerelease::EMPTY,
    }
    next
}

/// Returns the release-candidate number if `version` is an `-rc.N` pre-release.
fn rc_number(version: &Version) -> Option<u64> {
    version.pre.as_str().strip_prefix("rc.")?.parse().ok()
}

fn publish(krate: &Crate) -> bool {
    if !krate.publish {
        return true;
    }

    // First make sure the crate isn't already published at this version. This
    // script may be re-run and there's no need to re-attempt previous work.
    let Some(versions) = published_versions(&krate.name) else {
        return false;
    };
    if versions.iter().any(|(v, _)| *v == krate.version) {
        println!(
            "skip publish {} because {} is already published",
            krate.name, krate.version,
        );
        return true;
    }

    let status = cmd_status(
        Command::new("cargo")
            .arg("publish")
            .current_dir(krate.manifest.parent().unwrap())
            .arg("--no-verify"),
    );
    if !status.success() {
        println!("FAIL: failed to publish `{}`: {status}", krate.name);
        return false;
    }

    true
}

/// Yanks every release candidate which the version being released supersedes,
/// returning whether everything was yanked successfully.
fn yank_old_rcs(crates: &[Crate]) -> bool {
    let Some(versions) = old_rcs(crates) else {
        return false;
    };

    let mut todo = Vec::new();
    for version in versions {
        for krate in crates.iter().filter(|k| k.publish) {
            todo.push((krate, version.clone()));
        }
    }

    // Just like publishing there are enough crates here to run into crates.io
    // rate limits, so work through the list in a loop and drop each yank from
    // it once it succeeds. Failed yanks get another try on the next round.
    for _ in 0..10 {
        todo.retain(|(krate, version)| !yank(krate, version));

        if todo.is_empty() {
            break;
        }

        println!("{} yanks failed, waiting for a bit to retry", todo.len());
        thread::sleep(Duration::from_secs(40));
    }

    todo.is_empty()
}

/// Yanks `version` of `krate`, returning whether it succeeded.
fn yank(krate: &Crate, version: &Version) -> bool {
    let status = cmd_status(
        Command::new("cargo")
            .arg("yank")
            .arg("--version")
            .arg(version.to_string())
            .arg(&krate.name),
    );
    if !status.success() {
        println!("FAIL: failed to yank `{} {version}`: {status}", krate.name);
        return false;
    }
    true
}

/// Returns the release candidates which the version being released has
/// superseded and which aren't yanked already, or `None` if that couldn't be
/// determined.
fn old_rcs(crates: &[Crate]) -> Option<Vec<Version>> {
    let wasmtime = crates
        .iter()
        .find(|k| k.name == "wasmtime")
        .expect("failed to find the `wasmtime` crate");
    let current = &wasmtime.version;

    let Some(versions) = published_versions(&wasmtime.name) else {
        println!("FAIL: could not list versions of `{}`", wasmtime.name);
        return None;
    };

    let mut ret = Vec::new();
    for (version, already_yanked) in versions {
        // Skip if already yanked
        if already_yanked {
            continue;
        }

        // Skip if this is for some other version track
        if version.major != current.major
            || version.minor != current.minor
            || version.patch != current.patch
        {
            continue;
        }

        // Skip if this isn't actually a rc
        if rc_number(&version).is_none() {
            continue;
        }
        ret.push(version);
    }
    Some(ret)
}

/// Returns the version of the most recent `release-*` branch on `origin`.
///
/// Release branches are named after the version they're releasing and never
/// carry a pre-release suffix, so the largest version named by a branch is the
/// most recent release branch. Note that this only sees branches which have
/// been fetched, so `git fetch` needs to have happened first.
fn latest_release_branch() -> Version {
    let output = cmd_output(
        Command::new("git")
            .arg("for-each-ref")
            .arg("refs/remotes/origin")
            .arg("--format")
            .arg("%(refname)"),
    );
    assert!(output.status.success(), "failed to list remote branches");
    let refs = String::from_utf8_lossy(&output.stdout);

    let mut releases = refs
        .lines()
        .filter_map(|l| l.strip_prefix("refs/remotes/origin/release-"))
        .filter_map(|l| Version::parse(l).ok())
        // Skip anything with a suffix, which isn't a release branch name and
        // would additionally sort before the release it's a candidate for.
        .filter(|v| v.pre.is_empty() && v.build.is_empty())
        .collect::<Vec<_>>();
    releases.sort();
    releases.pop().expect("no `release-*` branches found")
}

/// The subset of a crates.io version listing that's used here.
#[derive(Deserialize)]
struct VersionList {
    versions: Vec<VersionEntry>,
}

#[derive(Deserialize)]
struct VersionEntry {
    num: String,
    yanked: bool,
}

/// Returns every version of `name` published to crates.io along with whether
/// each one is yanked, or `None` if the list couldn't be fetched.
fn published_versions(name: &str) -> Option<Vec<(Version, bool)>> {
    let body = curl(&format!("https://crates.io/api/v1/crates/{name}/versions"))?;
    let list = match serde_json::from_str::<VersionList>(&body) {
        Ok(list) => list,
        Err(e) => {
            println!("failed to parse the version list of `{name}`: {e}");
            return None;
        }
    };
    Some(
        list.versions
            .into_iter()
            .filter_map(|v| Some((Version::parse(&v.num).ok()?, v.yanked)))
            .collect(),
    )
}

fn curl(url: &str) -> Option<String> {
    // Transient failures talking to crates.io are common enough that it's worth
    // retrying a few times before giving up on this request entirely.
    for i in 0..5 {
        if i > 0 {
            thread::sleep(Duration::from_secs(10));
        }
        let output = cmd_output(
            Command::new("curl")
                .arg("--user-agent")
                .arg("bytecodealliance/wasmtime auto-publish script")
                .arg(url),
        );
        if output.status.success() {
            return Some(String::from_utf8_lossy(&output.stdout).into());
        }
        println!("failed to curl: {}", output.status);
        println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    }
    None
}

// Verify the current tree is publish-able to crates.io. The intention here is
// that we'll run `cargo package` on everything which verifies the build as-if
// it were published to crates.io. This requires using an incrementally-built
// directory registry generated from `cargo vendor` because the versions
// referenced from `Cargo.toml` may not exist on crates.io.
fn verify(crates: &[Crate]) {
    verify_capi();

    if Path::new(".cargo").exists() {
        panic!(
            "`.cargo` already exists on the file system, remove it and then run the script again"
        );
    }
    if Path::new("vendor").exists() {
        panic!(
            "`vendor` already exists on the file system, remove it and then run the script again"
        );
    }

    let vendor = cmd_output(Command::new("cargo").arg("vendor").stderr(Stdio::inherit()));
    assert!(vendor.status.success());

    fs::create_dir_all(".cargo").unwrap();
    fs::write(".cargo/config.toml", vendor.stdout).unwrap();

    for krate in crates {
        if !krate.publish {
            continue;
        }
        verify_and_vendor(krate);
    }

    fn verify_and_vendor(krate: &Crate) {
        verify_crates_io(krate);

        let mut cmd = Command::new("cargo");
        cmd.arg("package")
            .arg("--manifest-path")
            .arg(&krate.manifest)
            .env("CARGO_TARGET_DIR", "./target");
        if krate.name.contains("wasi-nn") {
            cmd.arg("--no-verify");
        }
        run_cmd(&mut cmd);
        run_cmd(
            Command::new("tar")
                .arg("xf")
                .arg(format!(
                    "../target/package/{}-{}.crate",
                    krate.name, krate.version
                ))
                .current_dir("./vendor"),
        );
        fs::write(
            format!(
                "./vendor/{}-{}/.cargo-checksum.json",
                krate.name, krate.version
            ),
            "{\"files\":{}}",
        )
        .unwrap();
    }

    fn verify_capi() {
        let version = workspace_version(&read_doc(Path::new("./Cargo.toml")));
        let Version {
            major,
            minor,
            patch,
            ..
        } = version;

        let mut count = 0;
        let contents = fs::read_to_string(C_HEADER_PATH).unwrap();
        for line in contents.lines() {
            if line.starts_with(&format!("#define WASMTIME_VERSION \"{version}\"")) {
                count += 1;
            } else if line.starts_with(&format!("#define WASMTIME_VERSION_MAJOR {major}")) {
                count += 1;
            } else if line.starts_with(&format!("#define WASMTIME_VERSION_MINOR {minor}")) {
                count += 1;
            } else if line.starts_with(&format!("#define WASMTIME_VERSION_PATCH {patch}")) {
                count += 1;
            }
        }

        assert!(
            count == 4,
            "invalid version macros in {C_HEADER_PATH}, should match \"{version}\"",
        );
    }

    fn verify_crates_io(krate: &Crate) {
        let name = &krate.name;
        let Some(owners) = curl(&format!("https://crates.io/api/v1/crates/{name}/owners")) else {
            panic!(
                "
failed to get owners for {name}

If this crate does not exist on crates.io yet please visit

  https://docs.wasmtime.dev/contributing-coding-guidelines.html#adding-crates

and follow the instructions there
"
            );
        };

        // This is the id of the `wasmtime-publish` user on crates.io
        if !owners.contains("\"id\":73222,") {
            panic!(
                "
crate {name} is not owned by wasmtime-publish, please visit:

  https://docs.wasmtime.dev/contributing-coding-guidelines.html#adding-crates

and follow the instructions there
"
            );
        }

        // TODO: waiting for trusted publishing to be proven to work before
        // activating this.
        if false && owners.split("\"id\"").count() != 2 {
            panic!(
                "
crate {name} is not exclusively owned by wasmtime-publish, please visit:

  https://docs.wasmtime.dev/contributing-coding-guidelines.html#adding-crates

and follow the instructions there
"
            );
        }
    }
}
