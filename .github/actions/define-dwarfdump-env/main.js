#!/usr/bin/env node

// On OSX pointing to brew's LLVM location.
if (process.platform == 'darwin') {
  console.log("::set-env name=DWARFDUMP::/usr/local/opt/llvm/bin/llvm-dwarfdump");
}

// On Linux pointing to specific version
if (process.platform == 'linux') {
  console.log("::set-env name=DWARFDUMP::/usr/bin/llvm-dwarfdump-9");
}
