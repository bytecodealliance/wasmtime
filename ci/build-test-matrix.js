// Small script used to calculate the matrix of tests that are going to be
// performed for a CI run.
//
// This is invoked by the `determine` step and is written in JS because I
// couldn't figure out how to write it in bash.

const fs = require('fs');
const { spawn } = require('node:child_process');

// Number of generic buckets to shard crates into. Note that we additionally add
// single-crate buckets for our biggest crates.
const GENERIC_BUCKETS = 3;

// Crates which are their own buckets. These are the very slowest to
// compile-and-test crates.
const SINGLE_CRATE_BUCKETS = ["wasmtime", "wasmtime-cli", "wasmtime-wasi"];

// This is the small, fast-to-execute matrix we use for PRs before they enter
// the merge queue. Same schema as `FULL_MATRIX`.
const FAST_MATRIX = [
  {
    "os": "ubuntu-latest",
    "name": "Test Linux x86_64",
    "filter": "linux-x64",
    "isa": "x64",
  },
];

// Returns whether the given package supports a 32-bit architecture, used when
// testing on i686 and armv7 below.
function supports32Bit(pkg) {
  if (pkg.indexOf("pulley") !== -1)
    return true;

  return pkg == 'wasmtime-fiber';
}

// This is the full, unsharded, and unfiltered matrix of what we test on
// CI. This includes a number of platforms and a number of cross-compiled
// targets that are emulated with QEMU. This must be kept tightly in sync with
// the `test` step in `main.yml`.
//
// The supported keys here are:
//
// * `os` - the github-actions name of the runner os
//
// * `name` - the human-readable name of the job
//
// * `filter` - a string which if `prtest:$filter` is in the commit messages
//   it'll force running this test suite on PR CI.
//
// * `isa` - changes to `cranelift/codegen/src/$isa` will automatically run this
//   test suite.
//
// * `target` - used for cross-compiles if present. Effectively Cargo's
//   `--target` option for all its operations.
//
// * `gcc_package`, `gcc`, `qemu`, `qemu_target` - configuration for building
//   QEMU and installing cross compilers to execute a cross-compiled test suite
//   on CI.
//
// * `rust` - the Rust version to install, and if unset this'll be set to
//   `default`
const FULL_MATRIX = [
  ...FAST_MATRIX,
  {
    "os": "ubuntu-latest",
    "name": "Test MSRV on Linux x86_64",
    "filter": "linux-x64",
    "isa": "x64",
    "rust": "msrv",
  },
  {
    "os": "ubuntu-latest",
    "name": "Test Linux x86_64 with MPK",
    "filter": "linux-x64",
    "isa": "x64"
  },
  {
    "os": "macos-13",
    "name": "Test macOS x86_64",
    "filter": "macos-x64",
  },
  {
    "os": "macos-14",
    "name": "Test macOS arm64",
    "filter": "macos-arm64",
    "target": "aarch64-apple-darwin",
  },
  {
    "os": "windows-latest",
    "name": "Test Windows MSVC x86_64",
    "filter": "windows-x64",
  },
  {
    "os": "windows-latest",
    "target": "x86_64-pc-windows-gnu",
    "name": "Test Windows MinGW x86_64",
    "filter": "mingw-x64"
  },
  {
    "os": "ubuntu-latest",
    "target": "aarch64-unknown-linux-gnu",
    "gcc_package": "gcc-aarch64-linux-gnu",
    "gcc": "aarch64-linux-gnu-gcc",
    "qemu": "qemu-aarch64 -L /usr/aarch64-linux-gnu",
    "qemu_target": "aarch64-linux-user",
    "name": "Test Linux arm64",
    "filter": "linux-arm64",
    "isa": "aarch64",
  },
  {
    "os": "ubuntu-latest",
    "target": "s390x-unknown-linux-gnu",
    "gcc_package": "gcc-s390x-linux-gnu",
    "gcc": "s390x-linux-gnu-gcc",
    "qemu": "qemu-s390x -L /usr/s390x-linux-gnu",
    "qemu_target": "s390x-linux-user",
    "name": "Test Linux s390x",
    "filter": "linux-s390x",
    "isa": "s390x"
  },
  {
    "os": "ubuntu-latest",
    "target": "riscv64gc-unknown-linux-gnu",
    "gcc_package": "gcc-riscv64-linux-gnu",
    "gcc": "riscv64-linux-gnu-gcc",
    "qemu": "qemu-riscv64 -cpu rv64,v=true,vlen=256,vext_spec=v1.0,Zfa=true,zba=true,zbb=true,zbc=true,zbs=true,zbkb=true,zcb=true,x-zicond=true -L /usr/riscv64-linux-gnu",
    "qemu_target": "riscv64-linux-user",
    "name": "Test Linux riscv64",
    "filter": "linux-riscv64",
    "isa": "riscv64",
  },
  {
    "name": "Tests on i686-unknown-linux-gnu",
    "32-bit": true,
    "os": "ubuntu-latest",
    "target": "i686-unknown-linux-gnu",
    "gcc_package": "gcc-i686-linux-gnu",
    "gcc": "i686-linux-gnu-gcc",
  },
  {
    "name": "Tests on armv7-unknown-linux-gnueabihf",
    "32-bit": true,
    "os": "ubuntu-latest",
    "target": "armv7-unknown-linux-gnueabihf",
    "gcc_package": "gcc-arm-linux-gnueabihf",
    "gcc": "arm-linux-gnueabihf-gcc",
    "qemu": "qemu-arm -L /usr/arm-linux-gnueabihf -E LD_LIBRARY_PATH=/usr/arm-linux-gnueabihf/lib",
    "qemu_target": "arm-linux-user",
  },
];

/// Get the workspace's full list of member crates.
async function getWorkspaceMembers() {
  // Spawn a `cargo metadata` subprocess, accumulate its JSON output from
  // `stdout`, and wait for it to exit.
  const child = spawn("cargo", ["metadata"], { encoding: "utf8" });
  let data = "";
  child.stdout.on("data", chunk => data += chunk);
  await new Promise((resolve, reject) => {
    child.on("close", resolve);
    child.on("error", reject);
  });

  // Get the names of the crates in the workspace from the JSON metadata by
  // building a package-id to name map and then translating the package-ids
  // listed as workspace members.
  const metadata = JSON.parse(data);
  const id_to_name = {};
  for (const pkg of metadata.packages) {
    id_to_name[pkg.id] = pkg.name;
  }
  return metadata.workspace_members.map(m => id_to_name[m]);
}

/// For each given target configuration, shard the workspace's crates into
/// buckets across that config.
///
/// This is essentially a `flat_map` where each config that logically tests all
/// crates in the workspace is mapped to N sharded configs that each test only a
/// subset of crates in the workspace. Each sharded config's subset of crates to
/// test are disjoint from all its siblings, and the union of all these siblings'
/// crates to test is the full workspace members set.
///
/// With some poetic license around a `crates_to_test` key that doesn't actually
/// exist, logically each element of the input `configs` list gets transformed
/// like this:
///
///     { os: "ubuntu-latest", isa: "x64", ..., crates: "all" }
///
///     ==>
///
///     [
///       { os: "ubuntu-latest", isa: "x64", ..., crates: ["wasmtime"] },
///       { os: "ubuntu-latest", isa: "x64", ..., crates: ["wasmtime-cli"] },
///       { os: "ubuntu-latest", isa: "x64", ..., crates: ["wasmtime-wasi"] },
///       { os: "ubuntu-latest", isa: "x64", ..., crates: ["cranelift", "cranelift-codegen", ...] },
///       { os: "ubuntu-latest", isa: "x64", ..., crates: ["wasmtime-slab", "cranelift-entity", ...] },
///       { os: "ubuntu-latest", isa: "x64", ..., crates: ["cranelift-environ", "wasmtime-cli-flags", ...] },
///       ...
///     ]
///
/// Note that `crates: "all"` is implicit in the input and omitted. Similarly,
/// `crates: [...]` in each output config is actually implemented via adding a
/// `bucket` key, which contains the CLI flags we must pass to `cargo` to run
/// tests for just this config's subset of crates.
async function shard(configs) {
  const members = await getWorkspaceMembers();

  // Divide the workspace crates into N disjoint subsets. Crates that are
  // particularly expensive to compile and test form their own singleton subset.
  const buckets = Array.from({ length: GENERIC_BUCKETS }, _ => new Set());
  let i = 0;
  for (const crate of members) {
    if (SINGLE_CRATE_BUCKETS.indexOf(crate) != -1) continue;
    buckets[i].add(crate);
    i = (i + 1) % GENERIC_BUCKETS;
  }
  for (crate of SINGLE_CRATE_BUCKETS) {
    buckets.push(new Set([crate]));
  }

  // For each config, expand it into N configs, one for each disjoint set we
  // created above.
  const sharded = [];
  for (const config of configs) {
    // Special case 32-bit configs. Only some crates, according to
    // `supports32Bit`, run on this target. At this time the set of supported
    // crates is small enough that they're not sharded.
    if (config["32-bit"] === true) {
      sharded.push(Object.assign(
        {},
        config,
        {
          bucket: members
            .map(c => supports32Bit(c) ? `--package ${c}` : `--exclude ${c}`)
            .join(" "),
        }
      ));
      continue;
    }

    for (const bucket of buckets) {
      sharded.push(Object.assign(
        {},
        config,
        {
          name: `${config.name} (${Array.from(bucket).join(', ')})`,
          // We run tests via `cargo test --workspace`, so exclude crates that
          // aren't in this bucket, rather than naming only the crates that are
          // in this bucket.
          bucket: members
            .map(c => bucket.has(c) ? `--package ${c}` : `--exclude ${c}`)
            .join(" "),
        }
      ));
    }
  }
  return sharded;
}

async function main() {
  // Our first argument is a file that is a giant json blob which contains at
  // least all the messages for all of the commits that were a part of this PR.
  // This is used to test if any commit message includes a string.
  const commits = fs.readFileSync(process.argv[2]).toString();

  // The second argument is a file that contains the names of all files modified
  // for a PR, used for file-based filters.
  const names = fs.readFileSync(process.argv[3]).toString();

  for (let config of FULL_MATRIX) {
    if (config.rust === undefined) {
      config.rust = 'default';
    }
  }

  // If the optional third argument to this script is `true` then that means all
  // tests are being run and no filtering should happen.
  if (process.argv[4] == 'true') {
    console.log(JSON.stringify(await shard(FULL_MATRIX), undefined, 2));
    return;
  }

  // When we aren't running the full CI matrix, filter configs down to just the
  // relevant bits based on files changed in this commit or if the commit asks
  // for a certain config to run.
  const filtered = FULL_MATRIX.filter(config => {
    // If an ISA-specific test was modified, then include that ISA config.
    if (config.isa && names.includes(`cranelift/codegen/src/isa/${config.isa}`)) {
      return true;
    }

    // If any runtest was modified, include all ISA configs as runtests can
    // target any backend.
    if (names.includes(`cranelift/filetests/filetests/runtests`)) {
      if (config.isa !== undefined)
        return true;
    }

    // For matrix entries that represent 32-bit only some crates support that,
    // so whenever the crates are changed be sure to run 32-bit tests on PRs
    // too.
    if (config["32-bit"] === true) {
      if (names.includes("pulley"))
        return true;
      if (names.includes("fiber"))
        return true;
    }

    // If the commit explicitly asks for this test config, then include it.
    if (config.filter && commits.includes(`prtest:${config.filter}`)) {
      return true;
    }

    return false;
  });

  // If at least one test is being run via our filters then run those tests.
  if (filtered.length > 0) {
    console.log(JSON.stringify(await shard(filtered), undefined, 2));
    return;
  }

  // Otherwise if nothing else is being run, run the fast subset of the matrix.
  console.log(JSON.stringify(await shard(FAST_MATRIX), undefined, 2));
}

main()
