#!/usr/bin/env node

import { fileURLToPath } from 'node:url';
import { dirname, join, parse } from 'node:path';
import { fetch } from 'zx'
import { mkdir, writeFile } from "node:fs/promises";
import decompress from 'decompress';
import decompressUnzip from 'decompress-unzip';
import decompressTarxz from '@felipecrs/decompress-tarxz';

const __dirname = dirname(fileURLToPath(import.meta.url));
const tag = 'dev';
let response = await fetch(`https://api.github.com/repos/bytecodealliance/wizer/releases/tags/${tag}`)
response = await response.json()
const id = response.id
let packages = {
    'wizer-darwin-arm64': {
        releaseAsset: `wizer-${tag}-x86_64-macos.tar.xz`,
        binaryAsset: 'wizer',
        description: 'The macOS 64-bit binary for Wizer, the WebAssembly Pre-Initializer',
        os: 'darwin',
        cpu: 'arm64',
    },
    'wizer-darwin-x64': {
        releaseAsset: `wizer-${tag}-x86_64-macos.tar.xz`,
        binaryAsset: 'wizer',
        description: 'The macOS 64-bit binary for Wizer, the WebAssembly Pre-Initializer',
        os: 'darwin',
        cpu: 'x64',
    },
    'wizer-linux-x64': {
        releaseAsset: `wizer-${tag}-x86_64-linux.tar.xz`,
        binaryAsset: 'wizer',
        description: 'The Linux 64-bit binary for Wizer, the WebAssembly Pre-Initializer',
        os: 'darwin',
        cpu: 'x64',
    },
    'wizer-win32-x64': {
        releaseAsset: `wizer-${tag}-x86_64-windows.zip`,
        binaryAsset: 'wizer.exe',
        description: 'The Windows 64-bit binary for Wizer, the WebAssembly Pre-Initializer',
        os: 'win32',
        cpu: 'x64',
    },
}
let assets = await fetch(`https://api.github.com/repos/bytecodealliance/wizer/releases/${id}/assets`)
assets = await assets.json()

// console.log({id,assets})
// 'arm', 'arm64', 'ia32', 'mips','mipsel', 'ppc', 'ppc64', 's390', 's390x', and 'x64'
// 'aix', 'android', 'darwin', 'freebsd','linux', 'openbsd', 'sunos', and 'win32'.
for (const [packageName, info] of Object.entries(packages)) {
    const asset = assets.find(asset => asset.name === info.releaseAsset)
    if (!asset) {
        throw new Error(`Can't find an asset named ${info.releaseAsset} for the release https://github.com/bytecodealliance/wizer/releases/tag/${tag}`)
    }
    const packageDirectory = join(__dirname, '../', packageName.split('/').pop())
    await mkdir(packageDirectory, { recursive: true })
    await writeFile(join(packageDirectory, 'package.json'), packageJson(packageName, tag, info.description, info.os, info.cpu))
    await writeFile(join(packageDirectory, 'index.js'), indexJs(info.binaryAsset))
    const browser_download_url = asset.browser_download_url;
    let archive = await fetch(browser_download_url)
    await decompress(Buffer.from(await archive.arrayBuffer()), packageDirectory, {
        strip:1,
        plugins: [
            decompressTarxz(),
            decompressUnzip()
        ],
        filter: file => parse(file.path).base === info.binaryAsset
    })
}

function indexJs(binaryAsset) {
    return `
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'
const __dirname = dirname(fileURLToPath(import.meta.url))
let location = join(__dirname, '${binaryAsset}')
export default location
`
}
function packageJson(name, version, description, os, cpu) {
    return JSON.stringify({
        name: `@bytecode-alliance/${name}`,
        bin: {
            [name]: "wizer"
        },
        type: "module",
        version,
        description,
        repository: "https://github.com/evanw/esbuild",
        license: "Apache-2.0",
        preferUnplugged: false,
        os: [os],
        cpu: [cpu],
    }, null, 4);
}
