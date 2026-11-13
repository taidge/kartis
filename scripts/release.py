"""Check a release tag and compare the packaged crate with crates.io."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tomllib
import urllib.error
import urllib.request


ROOT = Path(__file__).resolve().parent.parent


def command(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()


def verify(tag, require_published=False):
    manifest = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    package = manifest["package"]
    version = package["version"]
    if not re.fullmatch(r"v\d+\.\d+\.\d+", tag) or tag != f"v{version}":
        raise ValueError(f"Tag {tag!r} must match package version v{version}")
    head = command("git", "rev-parse", "HEAD")
    tagged = command("git", "rev-parse", f"refs/tags/{tag}^{{commit}}")
    if head != tagged:
        raise ValueError("The release tag must point to the checked-out commit")
    if command("git", "status", "--porcelain", "--untracked-files=normal"):
        raise ValueError("The release checkout must be clean")
    metadata = json.loads(command("cargo", "metadata", "--no-deps", "--format-version", "1", "--locked"))
    crate = Path(metadata["target_directory"]) / "package" / f"{package['name']}-{version}.crate"
    checksum = hashlib.sha256(crate.read_bytes()).hexdigest()
    url = f"https://crates.io/api/v1/crates/{package['name']}/{version}"
    request = urllib.request.Request(url, headers={"User-Agent": "kartis-release-check/0.1.0"})
    try:
        with urllib.request.urlopen(request, timeout=60) as response:
            published = json.load(response)["version"]
    except urllib.error.HTTPError as error:
        if error.code != 404:
            raise
        published = None
    if published is None and require_published:
        raise ValueError(f"{package['name']} {version} is not published on crates.io")
    if published is not None:
        if published["checksum"] != checksum:
            raise ValueError("The crates.io checksum differs from the release package")
        if published["yanked"]:
            raise ValueError("The published version is yanked")
    result = {
        "exists": str(published is not None).lower(),
        "crate_path": str(crate),
        "version": version,
        "checksum": checksum,
        "checksum_path": str(crate.parent / "SHA256SUMS"),
    }
    (crate.parent / "SHA256SUMS").write_text(f"{checksum}  {crate.name}\n", encoding="utf-8")
    output = os.environ.get("GITHUB_OUTPUT")
    if output:
        with open(output, "a", encoding="utf-8") as stream:
            for key, value in result.items():
                stream.write(f"{key}={value}\n")
    print(json.dumps(result, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    check = commands.add_parser("verify")
    check.add_argument("--tag", required=True)
    check.add_argument("--require-published", action="store_true")
    args = parser.parse_args()
    try:
        verify(args.tag, args.require_published)
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError) as error:
        print(f"Release verification failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
