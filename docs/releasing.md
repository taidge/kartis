# Publishing Kartis

[简体中文](releasing.zh-CN.md)

The package version lives in `Cargo.toml`. Releases use an annotated `vX.Y.Z`
tag on the exact commit that passed CI. The GitHub repository is
[`taidge/kartis`](https://github.com/taidge/kartis).

Before tagging, update the version, lockfile, installation examples, and
`CHANGELOG.md`. Push the commit and wait for all CI checks to pass. CI checks
spelling, Rust formatting, native tests and Clippy, WebAssembly, the minimum
supported Rust version, documentation, and crates.io package verification.
The package uses published dependencies, with no sibling repositories required.

Publishing uses Rust/Cargo 1.95.0 so local and CI packages use the same Cargo
version. Keep this toolchain pin aligned with the release workflow.

The first publication can use `cargo +1.95.0 publish --locked --all-features` with an authenticated
crates.io account after CI passes. Configure a crates.io Trusted Publisher for
owner `taidge`, repository `kartis`, workflow `release.yml` to enable subsequent
publishing from GitHub Actions without storing a long-lived registry token.
The publishing workflow requires this configuration for an unpublished version.

Create and push the version tag:

```sh
git tag -a v0.1.0 -m "Kartis 0.1.0"
git push origin v0.1.0
```

The release workflow runs CI again on that tag, verifies that its version matches
the manifest, and packages the crate. If the version is already published, its
SHA-256 checksum must exactly match the local package. Otherwise the workflow
authenticates with crates.io through OIDC and publishes it. Only after registry
verification does it create the GitHub Release and attach the crate and
`SHA256SUMS`. An existing mismatched or yanked version fails the release.

If a run fails after publication, rerun the failed jobs or dispatch the release
workflow with the same tag. Keep the tag unchanged: crates.io versions are
immutable. The release check is also available as:

```sh
cargo +1.95.0 package --locked --all-features
python scripts/release.py verify --tag v0.1.0 --require-published
```

The registry archive contains library sources, examples, documentation, fonts,
and license notices. Large development reference datasets stay in GitHub; CI
runs the complete test suite from the repository.
