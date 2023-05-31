import { endianness } from "node:os";
import { platform, arch } from "node:process";

const knownPackages = {
    "win32 x64 LE": "@bytecodealliance/wizer-win32-x64",
    "darwin arm64 LE": "@bytecodealliance/wizer-darwin-arm64",
    "darwin x64 LE": "@bytecodealliance/wizer-darwin-x64",
    "linux arm64 LE": "@bytecodealliance/wizer-linux-arm64",
    "linux s390x BE": "@bytecodealliance/wizer-linux-s390x",
    "linux x64 LE": "@bytecodealliance/wizer-linux-x64",
};

export function pkgForCurrentPlatform() {
    let platformKey = `${platform} ${arch} ${endianness()}`;
    if (platformKey in knownPackages) {
        return knownPackages[platformKey];
    }
    throw new Error(`Unsupported platform: "${platformKey}". "@bytecodealliance/wizer does not have a precompiled binary for the platform/architecture you are using. You can open an issue on https://github.com/bytecodealliance/wizer/issues to request for your platform/architecture to be included."`);
}
