#!/usr/bin/env node
import { endianness } from "node:os";
import { platform, arch } from "node:process";
import { execFileSync } from "node:child_process";
const knownPackages = {
    "win32 x64 LE": "@bytecode-alliance/wizer-win32-x64",
    "darwin arm64 LE": "@bytecode-alliance/wizer-darwin-arm64",
    "darwin x64 LE": "@bytecode-alliance/wizer-darwin-x64",
    "linux x64 LE": "@bytecode-alliance/wizer-linux-x64",
};

function pkgForCurrentPlatform() {
    let platformKey = `${platform} ${arch} ${endianness()}`;

    if (platformKey in knownPackages) {
        return knownPackages[platformKey];
    }
    throw new Error(`Unsupported platform: "${platformKey}". "@bytecode-alliance/wizer does not have a precompiled binary for the platform/architecture you are using. You can open an issue on https://github.com/bytecodealliance/wizer/issues to request for your platform/architecture to be included."`);
}

const pkg = pkgForCurrentPlatform();

let location;
try {
    // Check for the binary package from our "optionalDependencies". This
    // package should have been installed alongside this package at install time.
    location = (await import(pkg)).default;
} catch (e) {
    throw new Error(`The package "${pkg}" could not be found, and is needed by @bytecode-alliance/wizer.
If you are installing @bytecode-alliance/wizer with npm, make sure that you don't specify the
"--no-optional" flag. The "optionalDependencies" package.json feature is used
by @bytecode-alliance/wizer to install the correct binary executable for your current platform.`);
}

execFileSync(location, process.argv.slice(2), { stdio: "inherit" });
