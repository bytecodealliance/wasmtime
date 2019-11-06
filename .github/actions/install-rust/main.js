const child_process = require('child_process');
const toolchain = process.env.INPUT_TOOLCHAIN;

for (var i = 0, keys = Object.keys(process.env), ii = keys.length; i < ii; i++) {
  console.log(keys[i] + '=' + process.env[keys[i]]);
}

if (process.platform === 'darwin') {
  child_process.execSync(`curl https://sh.rustup.rs | sh -s -- -y --default-toolchain=none --profile=minimal`);
  const bindir = `${process.env.HOME}/.cargo/bin`;
  console.log(`::add-path::${bindir}`);
  process.env.PATH = `${process.env.PATH}:${bindir}`;
  child_process.execFileSync('rustup', ['set', 'profile', 'minimal']);
}

child_process.execFileSync('rustup', ['update', toolchain, '--no-self-update']);
child_process.execFileSync('rustup', ['default', toolchain]);
