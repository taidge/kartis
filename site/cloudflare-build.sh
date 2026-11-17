#!/usr/bin/env bash
# Cloudflare Pages may have neither Rust nor rustup in its build image.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

# Locate the normal user installation; never redirect Cargo or build caches.
if [[ -f "$HOME/.cargo/env" ]]; then
  source "$HOME/.cargo/env"
fi
if ! command -v rustup >/dev/null 2>&1; then
  installer="$(mktemp)"
  trap 'rm -f "$installer"' EXIT
  curl --proto '=https' --tlsv1.2 --fail --silent --show-error \
    https://sh.rustup.rs --output "$installer"
  sh "$installer" -y --profile minimal --default-toolchain 1.95.0 --no-modify-path
  source "$HOME/.cargo/env"
fi

python3 site/build.py --install-tools
