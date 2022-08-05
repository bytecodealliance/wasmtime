const platformPackages = [
  "@bytecode-alliance/wizer-win32-x64",
  "@bytecode-alliance/wizer-linux-x64",
  "@bytecode-alliance/wizer-darwin-x64",
  "@bytecode-alliance/wizer-darwin-arm64",
]

let location;
for (const pkg of platformPackages) {
  try {
    location = await import(pkg);
  } catch {}
}
throw new Error(
  "@bytecode-alliance/wizer does not have a precompiled binary for the platform/architecture you are using. You can open an issue on https://github.com/bytecodealliance/wizer/issues to request for your platform/architecture to be included."
);

export default location;