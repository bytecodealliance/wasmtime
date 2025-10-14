// Small script used to calculate the matrix of tests that are going to be
// performed for a CI run.
//
// This is invoked by the `determine` step and is written in JS because I
// couldn't figure out how to write it in bash.

const fs = require('fs');
const { spawn } = require('node:child_process');

// TODO
const SHARDS = 4;

const ubuntu = 'ubuntu-24.04';
const windows = 'windows-2025';
const macos = 'macos-15';

// This is the small, fast-to-execute matrix we use for PRs before they enter
// the merge queue. Same schema as `FULL_MATRIX`.
const FAST_MATRIX = [
  {
    "os": ubuntu,
    "name": "Test Linux x86_64",
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
// * `rust` - the Rust version to install, and if unset this'll be set to
//   `default`
const FULL_MATRIX = [
  ...FAST_MATRIX,
  {
    "os": ubuntu,
    "name": "Test MSRV on Linux x86_64",
    "filter": "linux-x64",
    "isa": "x64",
    "rust": "msrv",
  },
  {
    "os": ubuntu,
    "name": "Test Linux x86_64 with MPK",
    "filter": "linux-x64",
    "isa": "x64"
  },
  {
    "os": ubuntu,
    "name": "Test Linux x86_64 with ASAN",
    "filter": "asan",
    "rust": "wasmtime-ci-pinned-nightly",
    "target": "x86_64-unknown-linux-gnu",
  },
  {
    "os": ubuntu,
    "name": "Test Linux x86_64 with SDE",
    "filter": "sde",
    "isa": "x64",
    "sde": true,
    "crates": "cranelift-tools",
  },
  {
    "os": macos,
    "name": "Test macOS x86_64",
    "filter": "macos-x64",
    "target": "x86_64-apple-darwin",
  },
  {
    "os": macos,
    "name": "Test macOS arm64",
    "filter": "macos-arm64",
    "target": "aarch64-apple-darwin",
  },
  {
    "os": windows,
    "name": "Test Windows MSVC x86_64",
    "filter": "windows-x64",
  },
  {
    "os": windows,
    "target": "x86_64-pc-windows-gnu",
    "name": "Test Windows MinGW x86_64",
    "filter": "mingw-x64"
  },
  {
    "os": ubuntu + '-arm',
    "target": "aarch64-unknown-linux-gnu",
    "name": "Test Linux arm64",
    "filter": "linux-arm64",
    "isa": "aarch64",
  },
  {
    "os": 'ubuntu-24.04-s390x',
    "target": "s390x-unknown-linux-gnu",
    "name": "Test Linux s390x",
    "filter": "linux-s390x",
    "isa": "s390x"
    // These are no longer necessary now that this runner is using an
    // IBM-provided native runner. If that needs to change, however,
    // uncommenting these and switching back to `ubuntu` for the os will switch
    // us back to QEMU emulation.
    // "gcc_package": "gcc-s390x-linux-gnu",
    // "gcc": "s390x-linux-gnu-gcc",
    // "qemu": "qemu-s390x -L /usr/s390x-linux-gnu",
    // "qemu_target": "s390x-linux-user",
  },
  {
    "os": ubuntu,
    "target": "riscv64gc-unknown-linux-gnu",
    "gcc_package": "gcc-riscv64-linux-gnu",
    "gcc": "riscv64-linux-gnu-gcc",
    "qemu": "qemu-riscv64 -cpu rv64,v=true,vlen=256,vext_spec=v1.0,zfa=true,zfh=true,zba=true,zbb=true,zbc=true,zbs=true,zbkb=true,zcb=true,zicond=true,zvfh=true -L /usr/riscv64-linux-gnu",
    "qemu_target": "riscv64-linux-user",
    "name": "Test Linux riscv64",
    "filter": "linux-riscv64",
    "isa": "riscv64",
  },
  {
    "name": "Tests on i686-unknown-linux-gnu",
    "os": ubuntu,
    "target": "i686-unknown-linux-gnu",
    "gcc_package": "gcc-i686-linux-gnu",
    "gcc": "i686-linux-gnu-gcc",
  },
  {
    "name": "Tests on armv7-unknown-linux-gnueabihf",
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
/// TODO
async function shard(configs) {
  // For each config, expand it into N configs, one for each disjoint set we
  // created above.
  const sharded = [];
  for (const config of configs) {
    // If crates is specified, don't shard, just use the specified crates
    if (config.crates) {
      sharded.push(Object.assign(
        {},
        config,
        {
          name: `${config.name} (${config.crates})`,
        }
      ));
      continue;
    }
    for (let i = 0; i < SHARDS; i++) {
      sharded.push(Object.assign(
        {},
        config,
        {
          name: `${config.name} (${i}/${SHARDS})`,
          bucket: `--partition hash:${i}/${SHARDS}`,
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
