// Small script used to calculate the matrix of tests that are going to be
// performed for a CI run.
//
// This is invoked by the `determine` step and is written in JS because I
// couldn't figure out how to write it in bash.

const fs = require('fs');

// Our first argument is a file that is a giant json blob which contains at
// least all the messages for all of the commits that were a part of this PR.
// This is used to test if any commit message includes a string.
const commits = fs.readFileSync(process.argv[2]).toString();

// The second argument is a file that contains the names of all files modified
// for a PR, used for file-based filters.
const names = fs.readFileSync(process.argv[3]).toString();

// This is the full matrix of what we test on CI. This includes a number of
// platforms and a number of cross-compiled targets that are emulated with QEMU.
// This must be kept tightly in sync with the `test` step in `main.yml`.
//
// The supported keys here are:
//
// * `os` - the github-actions name of the runner os
// * `name` - the human-readable name of the job
// * `filter` - a string which if `prtest:$filter` is in the commit messages
//   it'll force running this test suite on PR CI.
// * `target` - used for cross-compiles if present. Effectively Cargo's
//   `--target` option for all its operations.
// * `gcc_package`, `gcc`, `qemu`, `qemu_target` - configuration for building
//   QEMU and installing cross compilers to execute a cross-compiled test suite
//   on CI.
// * `isa` - changes to `cranelift/codegen/src/$isa` will automatically run this
//   test suite.
// * `rust` - the Rust version to install, and if unset this'll be set to
//   `default`
const array = [
  {
    "os": "ubuntu-latest",
    "name": "Test Linux x86_64",
    "filter": "linux-x64",
    "isa": "x64",
    "extra_features": "--features wasmtime-wasi-nn/onnx"
  },
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
    "extra_features": "--features wasmtime-wasi-nn/onnx"
  },
  {
    "os": "macos-14",
    "name": "Test macOS arm64",
    "filter": "macos-arm64",
    "target": "aarch64-apple-darwin",
    "extra_features": "--features wasmtime-wasi-nn/onnx"
  },
  {
    "os": "windows-latest",
    "name": "Test Windows MSVC x86_64",
    "filter": "windows-x64",
    "extra_features": "--features wasmtime-wasi-nn/onnx"
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
    "qemu": "qemu-riscv64 -cpu rv64,v=true,vlen=256,vext_spec=v1.0,zba=true,zbb=true,zbc=true,zbs=true,zbkb=true,zcb=true -L /usr/riscv64-linux-gnu",
    "qemu_target": "riscv64-linux-user",
    "name": "Test Linux riscv64",
    "filter": "linux-riscv64",
    "isa": "riscv64",
  }
];

for (let config of array) {
  if (config.rust === undefined) {
    config.rust = 'default';
  }
}

function myFilter(item) {
  if (item.isa && names.includes(`cranelift/codegen/src/isa/${item.isa}`)) {
    return true;
  }
  if (item.filter && commits.includes(`prtest:${item.filter}`)) {
    return true;
  }

  // If any runtest was modified, re-run the whole test suite as those can
  // target any backend.
  if (names.includes(`cranelift/filetests/filetests/runtests`)) {
    return true;
  }

  return false;
}

const filtered = array.filter(myFilter);

// If the optional third argument to this script is `true` then that means all
// tests are being run and no filtering should happen.
if (process.argv[4] == 'true') {
  console.log(JSON.stringify(array));
  return;
}

// If at least one test is being run via our filters then run those tests.
if (filtered.length > 0) {
  console.log(JSON.stringify(filtered));
  return;
}

// Otherwise if nothing else is being run, run the first one which is Ubuntu
// Linux which should be the fastest for now.
console.log(JSON.stringify([array[0]]));
