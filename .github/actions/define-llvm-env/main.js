#!/usr/bin/env node

const fs = require('fs');

function set_env(name, val) {
  fs.appendFileSync(process.env['GITHUB_ENV'], `${name}=${val}\n`)
}

// On OSX pointing to brew's LLVM location.
if (process.platform == 'darwin') {
  set_env("DWARFDUMP", "/usr/local/opt/llvm/bin/llvm-dwarfdump");
  set_env("LLDB", "/usr/local/opt/llvm/bin/lldb");
}

// On Linux pointing to specific version
if (process.platform == 'linux') {
  set_env("DWARFDUMP", "/usr/bin/llvm-dwarfdump-9");
  set_env("LLDB", "/usr/bin/lldb-9");
}
