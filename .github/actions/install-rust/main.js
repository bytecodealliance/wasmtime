const child_process = require('child_process');
const toolchain = process.env.INPUT_TOOLCHAIN;
const fs = require('fs');

function set_env(name, val) {
  fs.appendFileSync(process.env['GITHUB_ENV'], `${name}=${val}\n`)
}

if (process.platform === 'darwin') {
  child_process.execSync(`curl https://sh.rustup.rs | sh -s -- -y --default-toolchain=none --profile=minimal`);
  const bindir = `${process.env.HOME}/.cargo/bin`;
  fs.appendFileSync(process.env['GITHUB_PATH'], `${bindir}\n`);
  process.env.PATH = `${process.env.PATH}:${bindir}`;
}

child_process.execFileSync('rustup', ['set', 'profile', 'minimal']);
child_process.execFileSync('rustup', ['update', toolchain, '--no-self-update']);
child_process.execFileSync('rustup', ['default', toolchain]);

// Deny warnings on CI to keep our code warning-free as it lands in-tree. Don't
// do this on nightly though since there's a fair amount of warning churn there.
if (!toolchain.startsWith('nightly')) {
  set_env("RUSTFLAGS", "-D warnings");
}

// Save disk space by avoiding incremental compilation, and also we don't use
// any caching so incremental wouldn't help anyway.
set_env("CARGO_INCREMENTAL", "0");

// Turn down debuginfo from 2 to 1 to help save disk space
set_env("CARGO_PROFILE_DEV_DEBUG", "1");
set_env("CARGO_PROFILE_TEST_DEBUG", "1");

if (process.platform === 'darwin') {
  set_env("CARGO_PROFILE_DEV_SPLIT_DEBUGINFO", "unpacked");
  set_env("CARGO_PROFILE_TEST_SPLIT_DEBUGINFO", "unpacked");
}
