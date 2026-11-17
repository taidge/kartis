#!/usr/bin/env python3
"""Build the portable static gallery; use the workspace's normal Cargo target."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys

HERE = Path(__file__).resolve().parent
WORKSPACE = HERE / "wasm"
VERSION = "0.2.130"


def run(command, capture=False):
    print("+ " + " ".join(map(str, command)), flush=True)
    return subprocess.run(command, cwd=WORKSPACE, check=True, text=True,
                          stdout=subprocess.PIPE if capture else None).stdout


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--install-tools", action="store_true",
                        help="Install Rust 1.95.0/wasm target and matching CLI through rustup/cargo in their normal locations")
    parser.add_argument("--toolchain", help="Existing rustup toolchain (default: caller's normal toolchain)")
    args = parser.parse_args()
    toolchain = args.toolchain
    if args.install_tools:
        toolchain = toolchain or "1.95.0"
        run(["rustup", "toolchain", "install", toolchain, "--profile", "minimal"])
        run(["rustup", "target", "add", "--toolchain", toolchain, "wasm32-unknown-unknown"])
    cargo = ["cargo"] + (["+" + toolchain] if toolchain else [])
    bindgen = shutil.which("wasm-bindgen")
    installed = subprocess.run([bindgen, "--version"], check=True, text=True,
                               capture_output=True).stdout.strip() if bindgen else ""
    if installed != "wasm-bindgen " + VERSION:
        if not args.install_tools:
            raise RuntimeError(f"Need wasm-bindgen-cli {VERSION}; run with --install-tools or install it with cargo install wasm-bindgen-cli --version {VERSION} --locked")
        run(cargo + ["install", "wasm-bindgen-cli", "--version", VERSION, "--locked", "--force"])
        bindgen = shutil.which("wasm-bindgen")
        if not bindgen:
            raise RuntimeError("Add Cargo's normal bin directory to PATH, then run the build again")
    metadata = json.loads(run(cargo + ["metadata", "--no-deps", "--format-version", "1", "--locked"], capture=True))
    target = Path(metadata["target_directory"])
    run(cargo + ["build", "--locked", "--release", "--target", "wasm32-unknown-unknown"])
    out = HERE / "dist"
    out.mkdir(exist_ok=True)
    shutil.copytree(HERE / "public", out, dirs_exist_ok=True)
    package = out / "pkg"
    package.mkdir(exist_ok=True)
    run([bindgen, str(target / "wasm32-unknown-unknown/release/kartis_site.wasm"),
         "--target", "web", "--out-dir", str(package), "--out-name", "kartis_site"])
    assets = {str(path.relative_to(out)).replace("\\", "/"): {
        "bytes": path.stat().st_size,
        "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
    } for path in sorted(out.rglob("*")) if path.is_file() and path.name != "build.json"}
    (out / "build.json").write_text(json.dumps({
        "version": "0.2.0", "target": "wasm32-unknown-unknown", "profile": "release",
        "wasm_bindgen": VERSION, "assets": assets,
    }, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"\nStatic site ready: {out.resolve()}\nServe: python -m http.server 8080 --directory {out}")


if __name__ == "__main__":
    try:
        main()
    except (subprocess.CalledProcessError, RuntimeError) as error:
        print(f"Site build failed: {error}", file=sys.stderr)
        sys.exit(1)
