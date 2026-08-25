#!/usr/bin/env bash
#
# Download the latest ISLE verifier SMT query cache from the `dev` release on
# github.com/bytecodealliance/wasmtime and install it as
# cranelift/isle/veri/cache, so a local run of verify.sh starts from the same
# state CI is currently using.
#
# The `dev` release is refreshed on every push to `main` by the
# "ISLE Verifier" CI workflow (see .github/workflows/isle-veri.yml).
#
# Usage:
#   ./cranelift/isle/veri/setup/download-cache.sh
#
# Requires: curl, tar.
# Optionally set GH_TOKEN to avoid GitHub API rate limits (unauthenticated
# requests are limited to 60/hour per IP).

set -euo pipefail

repo="bytecodealliance/wasmtime"
release="dev"
asset="isle-veri-cache.tar.gz"
cache_dir="cranelift/isle/veri/cache"

cd "$(git rev-parse --show-toplevel)"

if [ -n "${GH_TOKEN:-}" ]; then
    curl_args=(-fsSL -H "Authorization: Bearer ${GH_TOKEN}")
else
    curl_args=(-fsSL)
fi

json=$(curl "${curl_args[@]}" \
    "https://api.github.com/repos/${repo}/releases/tags/${release}")

# Extract the browser_download_url of our asset from the release JSON.
url=$(printf '%s\n' "$json" \
    | { grep -o "\"browser_download_url\"[[:space:]]*:[[:space:]]*\"[^\"]*/${asset}\"" || true; } \
    | sed 's/.*"browser_download_url"[[:space:]]*:[[:space:]]*"//; s/"$//')
if [ -z "$url" ]; then
    echo "ERROR: asset '${asset}' not found on release '${release}' of ${repo}" >&2
    exit 1
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
curl -fL --progress-bar -o "${tmp}/${asset}" "$url"

rm -rf "$cache_dir"
tar xzf "${tmp}/${asset}" -C cranelift/isle/veri
echo "Installed verifier cache to ${cache_dir}/"
echo "You can now run ./cranelift/isle/veri/verify.sh (or cache-only / rebuild-cache)."
