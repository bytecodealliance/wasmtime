const child_process = require('child_process');
const toolchain = process.env.INPUT_TOOLCHAIN;

if (process.platform === 'darwin') {
  child_process.execSync(`curl https://sh.rustup.rs | sh -s -- -y --default-toolchain=none --profile=minimal`);
  const bindir = `${process.env.HOME}/.cargo/bin`;
  console.log(`::add-path::${bindir}`);
  process.env.PATH = `${process.env.PATH}:${bindir}`;
}

child_process.execFileSync('rustup', ['set', 'profile', 'minimal']);
child_process.execFileSync('rustup', ['update', toolchain, '--no-self-update']);
child_process.execFileSync('rustup', ['default', toolchain]);

console.log(`::set-env name=CARGO_INCREMENTAL::0`);
