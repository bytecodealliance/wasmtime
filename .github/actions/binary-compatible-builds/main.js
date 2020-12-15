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
  set_env("python", "python3");
  return;
}

// On Windows we build against the static CRT to reduce dll dependencies
if (process.platform == 'win32') {
  set_env("RUSTFLAGS", "-Ctarget-feature=+crt-static");
  set_env("python", "python");
  return;
}

// ... and on Linux we do fancy things with containers. We'll spawn an old
// CentOS container in the background with a super old glibc, and then we'll run
// commands in there with the `$CENTOS` env var.

if (process.env.CENTOS !== undefined) {
  const args = ['exec', '-w', process.cwd(), '-i', 'centos'];
  for (const arg of process.argv.slice(2)) {
    args.push(arg);
  }
  child_process.execFileSync('docker', args, stdio);
  return;
}

// Add our rust mount onto PATH, but also add some stuff to PATH from
// the packages that we install.
let path = process.env.PATH;
path = `${path}:/rust/bin`;
path = `/opt/rh/devtoolset-8/root/usr/bin:${path}`;

// Spawn a container daemonized in the background which we'll connect to via
// `docker exec`. This'll have access to the current directory.
child_process.execFileSync('docker', [
  'run',
  '-di',
  '--name', 'centos',
  '-v', `${process.cwd()}:${process.cwd()}`,
  '-v', `${child_process.execSync('rustc --print sysroot').toString().trim()}:/rust:ro`,
  '--env', `PATH=${path}`,
  'centos:7',
], stdio);

// Use ourselves to run future commands
set_env("CENTOS", __filename);

// See https://edwards.sdsu.edu/research/c11-on-centos-6/ for where these
const exec = s => {
  child_process.execSync(`docker exec centos ${s}`, stdio);
};
exec('yum install -y centos-release-scl cmake xz epel-release');
exec('yum install -y python3 patchelf unzip');
exec('yum install -y devtoolset-8-gcc devtoolset-8-binutils devtoolset-8-gcc-c++');
exec('yum install -y git');

// Delete `libstdc++.so` to force gcc to link against `libstdc++.a` instead.
// This is a hack and not the right way to do this, but it ends up doing the
// right thing for now.
exec('rm -f /opt/rh/devtoolset-8/root/usr/lib/gcc/x86_64-redhat-linux/8/libstdc++.so');
set_env("python", "python3");
