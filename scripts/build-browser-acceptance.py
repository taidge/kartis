#!/usr/bin/env python3
"""Build and archive a source-bound browser-host package using normal Cargo config."""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import zipfile


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", nargs="?", help="new evidence directory (legacy positional form)")
    parser.add_argument("--out", help="new evidence directory; must not already exist")
    parser.add_argument("--toolchain", default="1.95.0", help="Rust toolchain (default: 1.95.0)")
    parser.add_argument("--jobs", type=int, default=4, help="parallel Cargo jobs (default: 4)")
    args = parser.parse_args()
    if bool(args.output) == bool(args.out):
        parser.error("provide exactly one output directory: --out=PATH or positional PATH")
    if args.jobs < 1:
        parser.error("--jobs must be positive")

    root = Path.cwd().resolve()
    consumer = root / "examples/browser-host"
    script = Path(__file__).resolve()
    try:
        script.relative_to(root)
    except ValueError:
        parser.error("run from the Kartis workspace containing this script")
    manifests = [root / name for name in (
        "Cargo.toml", "Cargo.lock", "examples/browser-host/Cargo.toml",
        "examples/browser-host/Cargo.lock",
    )]
    if not all(path.is_file() for path in manifests):
        parser.error("workspace root and independent browser-host Cargo manifests/locks are required")
    out = Path(args.out or args.output).resolve()
    out.mkdir(parents=True, exist_ok=False)

    def source_paths():
        return sorted(set([
            *root.glob("src/**/*.rs"),
            *consumer.glob("src/**/*.rs"),
            *root.glob("examples/shared/**/*.rs"),
            *[path for path in root.glob("tests/fonts/**/*") if path.is_file()],
            root / "README.md",
            *manifests,
            script,
        ]))

    def fence():
        return {path.relative_to(root).as_posix(): sha(path) if path.is_file() else None
                for path in source_paths()}

    report = {
        "passed": False,
        "featureAccepted": False,
        "broadGoalComplete": False,
        "scope": "Exact browser-host source build; runtime acceptance is separate.",
        "toolchain": args.toolchain,
        "sourceStart": fence(),
        "commands": [],
    }

    def save():
        (out / "report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")

    def run(name, command, capture=False):
        print("START " + name, flush=True)
        log = out / (name + ".log")
        record = {
            "name": name, "command": command,
            "cwd": consumer.relative_to(root).as_posix(),
            "actualTerminalExit": None,
        }
        report["commands"].append(record)
        try:
            if capture:
                result = subprocess.run(command, cwd=consumer, capture_output=True, check=False)
                log.write_bytes(result.stdout + b"\n--- stderr ---\n" + result.stderr)
            else:
                with log.open("xb") as stream:
                    result = subprocess.run(command, cwd=consumer, stdout=stream,
                                            stderr=subprocess.STDOUT, check=False)
            record["actualTerminalExit"] = result.returncode
        except OSError as error:
            record["launchError"] = str(error)
            if not log.exists():
                log.write_text(str(error) + "\n", encoding="utf-8")
            raise
        finally:
            if log.exists():
                record["logSha256"] = sha(log)
            save()
        print("FINISH " + name + " " + str(result.returncode), flush=True)
        if result.returncode:
            print(log.read_text(encoding="utf-8", errors="replace")[-18000:], flush=True)
            return None
        return result.stdout if capture else True

    exit_code = 1
    try:
        (out / "executed-producer.py").write_bytes(script.read_bytes())
        with zipfile.ZipFile(out / "source.zip", "x", zipfile.ZIP_DEFLATED) as archive:
            for path in source_paths():
                archive.write(path, path.relative_to(root).as_posix())
        report["sourceArchiveSha256"] = sha(out / "source.zip")
        report["executedProducerSha256"] = sha(out / "executed-producer.py")
        save()

        cargo = ["cargo", "+" + args.toolchain]
        raw = run("metadata", cargo + ["metadata", "--locked", "--no-deps",
                                      "--format-version", "1"], capture=True)
        if raw is None:
            return 1
        metadata = json.loads(raw)
        (out / "cargo-metadata.json").write_bytes(raw)
        target = Path(metadata["target_directory"])
        if not target.is_absolute():
            raise ValueError("cargo metadata returned a non-absolute target_directory")
        report["targetDirectory"] = str(target)
        report["metadataSha256"] = sha(out / "cargo-metadata.json")
        # Never set Cargo home/target or TEMP/TMP. Resolve the binary from the
        # actual consumer workspace metadata, including caller configuration.
        common = ["--locked", "--target", "wasm32-unknown-unknown", "--jobs", str(args.jobs)]
        if run("strict", cargo + ["clippy", *common, "--", "-D", "warnings"]) is None:
            return 1
        if run("release", cargo + ["build", *common, "--release"]) is None:
            return 1
        wasm = target / "wasm32-unknown-unknown/release/kartis_host_fixture.wasm"
        report["inputWasmSha256"] = sha(wasm)
        if run("bindgen", ["wasm-bindgen", str(wasm), "--target", "web", "--out-dir",
                           str(out / "pkg"), "--out-name", "host"]) is None:
            return 1
        required = {"host.js", "host_bg.wasm", "host.d.ts", "host_bg.wasm.d.ts"}
        report["package"] = {
            path.name: sha(path) for path in sorted((out / "pkg").glob("host*")) if path.is_file()
        }
        if not required <= report["package"].keys():
            raise ValueError("wasm-bindgen did not produce the complete host web package")
        report["passed"] = True
        exit_code = 0
    except Exception as error:
        report["error"] = type(error).__name__ + ": " + str(error)
        print(report["error"], file=sys.stderr, flush=True)
    finally:
        report["sourceEnd"] = fence()
        report["sourceUnchanged"] = report["sourceStart"] == report["sourceEnd"]
        report["passed"] = report["passed"] and report["sourceUnchanged"]
        save()
        print(json.dumps({key: report.get(key) for key in (
            "passed", "sourceUnchanged", "package", "targetDirectory",
        )}), flush=True)
    return exit_code if report["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
