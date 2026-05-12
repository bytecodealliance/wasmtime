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
//
// An entry may be a string (the crate name, producing one bucket) or an
// object `{ crate, sub: [{ name, args }, ...] }` to split a single crate's
// tests across multiple buckets. Sub-bucketing is reserved for crates whose
// integration tests dominate the wall clock and split cleanly along
// `--test` boundaries; the build of the crate's dependencies and test
// binaries is duplicated across the sub-buckets, but the test-execution
// portion (the longer half on slow targets like QEMU) parallelizes.
const SINGLE_CRATE_BUCKETS = [
  "wasmtime",
  {
    "crate": "wasmtime-cli",
    "sub": [
      // The two largest integration suites; each gets its own bucket.
      { "name": "all", "args": "--test all" },
      { "name": "wast", "args": "--test wast" },
      // Everything else in the crate: library, binaries, and the smaller
      // integration tests. Cargo doesn't have an "exclude test" flag, so the
      // remaining `--test` targets are enumerated explicitly.
      { "name": "other", "args": "--lib --bins --test disable_host_trap_handlers --test disas --test rlimited-memory --test wasi" },
    ],
  },
  "wasmtime-wasi",
];

// Helper: get just the crate name from a SINGLE_CRATE_BUCKETS entry.
function singleBucketCrateName(entry) {
  return typeof entry === "string" ? entry : entry.crate;
}

const ubuntu = 'ubuntu-24.04';
const windows = 'windows-2025';
const macos = 'macos-15';

// This is the small, fast-to-execute matrix we use for PRs before they enter
// the merge queue. Same schema as `FULL_MATRIX`.
const FAST_MATRIX = [
  {
    "name": "Test Linux x86_64",
    "os": ubuntu,
    "filter": "linux-x64",
    "isa": "x64",
  },
];

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
// * `sde` - if `true`, indicates this test should use Intel SDE for instruction
//   emulation. SDE will be set up and configured as the test runner.
//
// * `crates` - if a string, this config is not sharded across the workspace
//   and instead runs against only the named crate. If an array of strings,
//   the config produces one job per named crate (each job's name suffixed
//   with the crate name) and skips the generic buckets entirely. Used to
//   restrict env-var-toggled test variants to only the crates that observe
//   that env var (e.g. MPK).
//
// * `rust` - the Rust version to install, and if unset this'll be set to
//   `default`
const FULL_MATRIX = [
  ...FAST_MATRIX,
  {
    // MPK is only observed at test time by code that reads
    // WASMTIME_TEST_FORCE_MPK (the `wasmtime` runtime crate and the
    // root-level integration tests under `wasmtime-cli`). Restricting MPK
    // testing to those two crates eliminates four redundant shards
    // (3 generic + wasmtime-wasi) that produce identical results to
    // `Test Linux x86_64`.
    "name": "Test MPK",
    "os": ubuntu,
    "filter": "linux-x64",
    "isa": "x64",
    "crates": ["wasmtime", "wasmtime-cli"],
  },
  {
    "name": "Test ASAN",
    "os": ubuntu,
    "filter": "asan",
    "rust": "wasmtime-ci-pinned-nightly",
    "target": "x86_64-unknown-linux-gnu",
  },
  {
    "name": "Test Intel SDE",
    "os": ubuntu,
    "filter": "sde",
    "isa": "x64",
    "sde": true,
    "crates": "cranelift-tools",
  },
  {
    "name": "Test macOS x86_64",
    "os": macos,
    "filter": "macos-x64",
    "target": "x86_64-apple-darwin",
  },
  {
    "name": "Test macOS arm64",
    "os": macos,
    "filter": "macos-arm64",
    "target": "aarch64-apple-darwin",
  },
  {
    "name": "Test MSVC x86_64",
    "os": windows,
    "filter": "windows-x64",
  },
  {
    "name": "Test MinGW x86_64",
    "os": windows,
    "target": "x86_64-pc-windows-gnu",
    "filter": "mingw-x64"
  },
  {
    "name": "Test Linux arm64",
    "os": ubuntu + '-arm',
    "target": "aarch64-unknown-linux-gnu",
    "filter": "linux-arm64",
    "isa": "aarch64",
  },
  {
    "name": "Test Linux s390x",
    // "os": 'ubuntu-24.04-s390x',
    "os": ubuntu,
    "target": "s390x-unknown-linux-gnu",
    "filter": "linux-s390x",
    "isa": "s390x",
    "gcc_package": "gcc-s390x-linux-gnu",
    "gcc": "s390x-linux-gnu-gcc",
    "qemu": "qemu-s390x -L /usr/s390x-linux-gnu",
    "qemu_target": "s390x-linux-user",
  },
  {
    "name": "Test Linux riscv64",
    "os": ubuntu,
    "target": "riscv64gc-unknown-linux-gnu",
    "gcc_package": "gcc-riscv64-linux-gnu",
    "gcc": "riscv64-linux-gnu-gcc",
    "qemu": "qemu-riscv64 -cpu rv64,v=true,vlen=256,vext_spec=v1.0,zfa=true,zfh=true,zba=true,zbb=true,zbc=true,zbs=true,zbkb=true,zcb=true,zicond=true,zvfh=true -L /usr/riscv64-linux-gnu",
    "qemu_target": "riscv64-linux-user",
    "filter": "linux-riscv64",
    "isa": "riscv64",
  },
  {
    "name": "Tests Linux i686",
    "os": ubuntu,
    "target": "i686-unknown-linux-gnu",
    "gcc_package": "gcc-i686-linux-gnu",
    "gcc": "i686-linux-gnu-gcc",
  },
  {
    "name": "Tests Linux armv7",
    "os": ubuntu,
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
  // A bucket is either a Set of crate names (used as-is for `cargo test
  // --workspace --exclude ...`) or an object `{ crate, sub }` describing a
  // single-crate bucket that should be further split by `--test` filter.
  const singleBucketCrateNames = new Set(SINGLE_CRATE_BUCKETS.map(singleBucketCrateName));
  const buckets = Array.from({ length: GENERIC_BUCKETS }, _ => new Set());
  let i = 0;
  for (const crate of members) {
    if (singleBucketCrateNames.has(crate)) continue;
    buckets[i].add(crate);
    i = (i + 1) % GENERIC_BUCKETS;
  }
  for (const entry of SINGLE_CRATE_BUCKETS) {
    if (typeof entry === "string") {
      buckets.push(new Set([entry]));
    } else {
      // A crate with sub-buckets: push one bucket per `sub` entry, retaining
      // the crate name and extra-args for naming and bucket-arg expansion.
      for (const sub of entry.sub) {
        buckets.push({ crate: entry.crate, name: sub.name, args: sub.args });
      }
    }
  }

  // For each config, expand it into N configs, one for each disjoint set we
  // created above.
  const sharded = [];
  for (const config of configs) {
    // If `crates` is specified, don't shard against the generic buckets.
    // A string value produces a single job for that crate; an array value
    // produces one job per crate with the crate name appended to `name`.
    if (config.crates) {
      const cratesList = Array.isArray(config.crates) ? config.crates : [config.crates];
      const useSuffix = Array.isArray(config.crates);
      for (const crate of cratesList) {
        sharded.push(Object.assign(
          {},
          config,
          {
            name: useSuffix ? `${config.name} (${crate})` : config.name,
            bucket: members
              .map(c => c === crate ? `--package ${c}` : `--exclude ${c}`)
              .join(" "),
          }
        ));
      }
      continue;
    }

    let nbucket = 1;
    for (const bucket of buckets) {
      let bucket_name;
      let bucket_args;
      if (bucket instanceof Set) {
        bucket_name = bucket.size === 1
          ? Array.from(bucket)[0]
          : `${nbucket}/${buckets.length}`;
        bucket_args = members
          .map(c => bucket.has(c) ? `--package ${c}` : `--exclude ${c}`)
          .join(" ");
      } else {
        // Sub-bucket of a single crate.
        bucket_name = `${bucket.crate}-${bucket.name}`;
        bucket_args = members
          .map(c => c === bucket.crate ? `--package ${c}` : `--exclude ${c}`)
          .join(" ") + " " + bucket.args;
      }

      sharded.push(Object.assign(
        {},
        config,
        {
          name: `${config.name} (${bucket_name})`,
          // We run tests via `cargo test --workspace`, so exclude crates that
          // aren't in this bucket, rather than naming only the crates that are
          // in this bucket.
          bucket: bucket_args,
        }
      ));
      nbucket += 1;
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
