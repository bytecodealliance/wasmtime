// Small script used to calculate the matrix of builds that are going to be
// done if CI decides to do a release build.
//
// This is a separate script primarily to write out all the release
// targets/platforms once and then duplicate them all with a "min" build.

const ubuntu = 'ubuntu-24.04';
const windows = 'windows-2025';
const macos = 'macos-14';

const array = [
  {
    // The name of the build which shows up in the name of the artifact for
    // Wasmtime's github releases.
    "build": "x86_64-linux",
    // The GitHub Actions platform that this build runs on
    "os": ubuntu,
    // The Rust target that will be used for the build.
    "target": "x86_64-unknown-linux-gnu",
    "env": { "DOCKER_IMAGE": "./ci/docker/x86_64-linux/Dockerfile" },
  },
  {
    "build": "aarch64-linux",
    "os": ubuntu,
    "target": "aarch64-unknown-linux-gnu",
    "env": { "DOCKER_IMAGE": "./ci/docker/aarch64-linux/Dockerfile" },
  },
  {
    "build": "s390x-linux",
    "os": ubuntu,
    "target": "s390x-unknown-linux-gnu",
    "env": { "DOCKER_IMAGE": "./ci/docker/s390x-linux/Dockerfile" },
  },
  {
    "build": "riscv64gc-linux",
    "os": ubuntu,
    "target": "riscv64gc-unknown-linux-gnu",
    "env": { "DOCKER_IMAGE": "./ci/docker/riscv64gc-linux/Dockerfile" },
  },
  {
    "build": "x86_64-macos",
    "os": macos,
    "target": "x86_64-apple-darwin",
    // On OSX all we need to do is configure our deployment target as old as
    // possible. For now 10.9 is the limit.
    "env": { "MACOSX_DEPLOYMENT_TARGET": "10.9" },
  },
  {
    "build": "aarch64-macos",
    "os": macos,
    "target": "aarch64-apple-darwin",
    "env": { "MACOSX_DEPLOYMENT_TARGET": "10.9" },
  },
  {
    "build": "x86_64-windows",
    "os": windows,
    "target": "x86_64-pc-windows-msvc",
    // On Windows we build against the static CRT to reduce dll dependencies
    "env": { "RUSTFLAGS": "-Ctarget-feature=+crt-static" },
  },
  {
    "build": "x86_64-mingw",
    "os": windows,
    "target": "x86_64-pc-windows-gnu",
    "env": { "RUSTFLAGS": "-Ctarget-feature=+crt-static" },
  },
  {
    "build": "aarch64-android",
    "os": ubuntu,
    "target": "aarch64-linux-android",
  },
  {
    "build": "x86_64-android",
    "os": ubuntu,
    "target": "x86_64-linux-android",
  },
  {
    "build": "x86_64-musl",
    "os": ubuntu,
    "target": "x86_64-unknown-linux-musl",
    "env": { "DOCKER_IMAGE": "./ci/docker/x86_64-musl/Dockerfile" },
  },
  {
    "build": "aarch64-musl",
    "os": ubuntu,
    "target": "aarch64-unknown-linux-musl",
    "env": { "DOCKER_IMAGE": "./ci/docker/aarch64-musl/Dockerfile" },
  },
  {
    "build": "aarch64-windows",
    "os": windows,
    "target": "aarch64-pc-windows-msvc",
    "env": { "RUSTFLAGS": "-Ctarget-feature=+crt-static" },
  },
  {
    "build": "i686-linux",
    "os": ubuntu,
    "target": "i686-unknown-linux-gnu",
    "env": { "DOCKER_IMAGE": "./ci/docker/i686-linux/Dockerfile" },
  },
  {
    "build": "armv7-linux",
    "os": ubuntu,
    "target": "armv7-unknown-linux-gnueabihf",
    "env": { "DOCKER_IMAGE": "./ci/docker/armv7-linux/Dockerfile" },
  },
  {
    "build": "i686-windows",
    "os": windows,
    "target": "i686-pc-windows-msvc",
    "env": { "RUSTFLAGS": "-Ctarget-feature=+crt-static" },
  },
];

const builds = [];
for (let build of array) {
  // Perform a "deep clone" roundtripping through JSON for a copy of the build
  // that's normal
  build.rust = 'default';
  builds.push(JSON.parse(JSON.stringify(build)));

  // Next generate a "min" build and add it to the builds list. Min builds
  // require Nightly rust due to some nightly build options that are configured.
  build.build += '-min';
  build.rust = 'wasmtime-ci-pinned-nightly';
  builds.push(build);
}

console.log(JSON.stringify(builds));
