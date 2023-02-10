const fs = require('fs');

const commits = fs.readFileSync(process.argv[2]).toString();
const names = fs.readFileSync(process.argv[3]).toString();

const array = [
  {
    "os": "ubuntu-latest",
    "name": "Test Linux x86_64",
    "filter": "linux-x64"
  },
  {
    "os": "macos-latest",
    "name": "Test macOS x86_64",
    "filter": "macos-x64"
  },
  {
    "os": "windows-latest",
    "name": "Test Windows MSVC x86_64",
    "filter": "windows-x64"
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
    "isa": "aarch64"
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
    "qemu": "qemu-riscv64 -L /usr/riscv64-linux-gnu",
    "qemu_target": "riscv64-linux-user",
    "name": "Test Linux riscv64",
    "filter": "linux-riscv64",
    "isa": "riscv64"
  }
];

function myFilter(item) {
  if (item.isa && names.includes(`cranelift/codegen/src/isa/${item.isa}`)) {
    return true;
  }
  if (item.filter && commits.includes(`prtest:${item.filter}`)) {
    return true;
  }
  return false;
}

const filtered = array.filter(myFilter);

if (process.argv[4] == 'true') {
  console.log(JSON.stringify(array));
} else if (filtered.length > 0) {
  console.log(JSON.stringify(filtered));
} else {
  console.log(JSON.stringify([array[0]]));
}

