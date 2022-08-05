import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { copyFile } from "node:fs/promises";

const __dirname = dirname(fileURLToPath(import.meta.url));

const platformPackages = [
  "@bytecode-alliance/wizer-win32-x64",
  "@bytecode-alliance/wizer-linux-x64",
  "@bytecode-alliance/wizer-darwin-x64",
  "@bytecode-alliance/wizer-darwin-arm64",
]


for (const pkg of platformPackages) {
  try {
    const location = await import(pkg);
    await copyFile(location, join(__dirname, 'wizer'))
    process.exit()
  } catch {}
}
throw new Error(
  "@bytecode-alliance/wizer does not have a precompiled binary for the platform/architecture you are using. You can open an issue on https://github.com/bytecodealliance/wizer/issues to request for your platform/architecture to be included."
);