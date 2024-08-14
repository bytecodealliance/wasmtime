#!/usr/bin/env node

const child_process = require('child_process');
const stdio = { stdio: 'inherit' };
const fs = require('fs');

function set_env(name, val) {
  fs.appendFileSync(process.env['GITHUB_ENV'], `${name}=${val}\n`)
}

// On OSX all we need to do is configure our deployment target as old as
// possible. For now 10.9 is the limit.
if (process.platform == 'darwin') {
  set_env("MACOSX_DEPLOYMENT_TARGET", "10.9");
  return;
}

// On Windows we build against the static CRT to reduce dll dependencies
if (process.platform == 'win32') {
  set_env("RUSTFLAGS", "-Ctarget-feature=+crt-static");
  return;
}

// Android doesn't use a container as it's controlled by the installation of the
// SDK/NDK.
if (process.env.INPUT_NAME && process.env.INPUT_NAME.indexOf("android") >= 0) {
  return;
}

// ... and on Linux we do fancy things with containers. We'll spawn an old
// CentOS container in the background with a super old glibc, and then we'll run
// commands in there with the `$CENTOS` env var.

if (process.env.CENTOS !== undefined) {
  const args = ['exec', '--workdir', process.cwd(), '--interactive'];
  // Forward any rust-looking env vars from the environment into the container
  // itself.
  for (let key in process.env) {
    if (key.startsWith('CARGO') || key.startsWith('RUST')) {
      args.push('--env');
      args.push(key);
    }
  }
  args.push('build-container')

  // Start the container by appending to `$PATH` with the `/rust/bin` path that
  // is mounted below.
  args.push('bash');
  args.push('-c');
  args.push('export PATH=$PATH:/rust/bin; export RUSTFLAGS="$RUSTFLAGS $EXTRA_RUSTFLAGS"; exec "$@"');
  args.push('bash');

  // Add in whatever we're running which will get executed in the sub-shell with
  // an augmented PATH.
  for (const arg of process.argv.slice(2)) {
    args.push(arg);
  }
  child_process.execFileSync('docker', args, stdio);
  return;
}

const name = process.env.INPUT_NAME.replace(/-min$/, '');

child_process.execFileSync('docker', [
  'build',
  '--tag', 'build-image',
  `${process.cwd()}/ci/docker/${name}`
], stdio);

child_process.execFileSync('docker', [
  'run',
  '--detach',
  '--interactive',
  '--name', 'build-container',
  '-v', `${process.cwd()}:${process.cwd()}`,
  '-v', `${child_process.execSync('rustc --print sysroot').toString().trim()}:/rust:ro`,
  'build-image',
], stdio);

// Use ourselves to run future commands
set_env("CENTOS", __filename);
