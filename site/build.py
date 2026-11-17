#!/usr/bin/env python3
"""Build the portable static gallery; use the workspace's normal Cargo target."""
import argparse
import hashlib
import json
import re
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


def fresh_output():
    """Remove only the verified, generated site/dist directory."""
    requested = HERE / "dist"
    if requested.is_symlink() or getattr(requested, "is_junction", lambda: False)():
        raise RuntimeError("Refusing to clean a linked site/dist")
    out = requested.resolve()
    if out.parent != HERE.resolve() or out.name != "dist":
        raise RuntimeError("Generated output must resolve directly beneath site/")
    if out.exists():
        if not out.is_dir():
            raise RuntimeError("Expected site/dist to be a generated directory")
        shutil.rmtree(out)
    out.mkdir()
    return out


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
    out = fresh_output()
    shutil.copytree(HERE / "public", out, dirs_exist_ok=True)
    runtime = out / "_runtime"
    runtime.mkdir()
    for entry in ("app.js", "style.css"):
        (out / entry).replace(runtime / entry)
    package = runtime / "pkg"
    package.mkdir()
    run([bindgen, str(target / "wasm32-unknown-unknown/release/kartis_site.wasm"),
         "--target", "web", "--out-dir", str(package), "--out-name", "kartis_site"])
    # Include path boundaries and lengths as well as every actual file byte.
    # Bindgen's JS, WASM and snippets always move together with the app and CSS.
    digest = hashlib.sha256()
    for path in sorted(runtime.rglob("*")):
        if path.is_file():
            name = path.relative_to(runtime).as_posix().encode("utf-8")
            content = path.read_bytes()
            digest.update(name + b"\0")
            digest.update(len(content).to_bytes(8, "big"))
            digest.update(content)
    asset_revision = digest.hexdigest()
    versioned = out / "assets" / asset_revision
    versioned.parent.mkdir()
    runtime.rename(versioned)
    index_path = out / "index.html"
    index = index_path.read_text(encoding="utf-8")
    for attribute, entry in (("href", "style.css"), ("src", "app.js")):
        original = f'{attribute}="/{entry}"'
        if index.count(original) != 1:
            raise RuntimeError(f"Expected exactly one HTML entry for {entry}")
        index = index.replace(original, f'{attribute}="/assets/{asset_revision}/{entry}"')
    index_path.write_text(index, encoding="utf-8", newline="\n")
    assets = {str(path.relative_to(out)).replace("\\", "/"): {
        "bytes": path.stat().st_size,
        "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
    } for path in sorted(out.rglob("*")) if path.is_file() and path.name != "build.json"}
    try:
        revision = subprocess.run(["git", "rev-parse", "HEAD"], cwd=HERE.parent,
                                  capture_output=True, text=True, check=False)
        commit = revision.stdout.strip() if revision.returncode == 0 else ""
    except OSError:
        commit = ""
    (out / "build.json").write_text(json.dumps({
        "version": "0.2.0", "target": "wasm32-unknown-unknown", "profile": "release",
        "source_commit": commit if re.fullmatch(r"[0-9a-fA-F]{40}", commit) else None,
        "asset_revision": asset_revision,
        "wasm_bindgen": VERSION, "assets": assets,
    }, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"\nStatic site ready: {out.resolve()}\nServe: python -m http.server 8080 --directory {out}")


if __name__ == "__main__":
    try:
        main()
    except (subprocess.CalledProcessError, RuntimeError) as error:
        print(f"Site build failed: {error}", file=sys.stderr)
        sys.exit(1)
