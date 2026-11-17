# Kartis gallery

A static Rust/WebAssembly showcase for `kartis.taidge.org`. Six live examples
use application-owned `BrowserChart` SVG and Canvas surfaces, real native input,
Glory legends/tooltips and responsive Rust layout callbacks. The surrounding
HTML/CSS and small JavaScript module supply navigation and controls. No CDN,
React runtime, remote dataset, tracking script or API service is required.

## Build

Requirements: Python 3, Rust 1.95 or later with the `wasm32-unknown-unknown` target,
and `wasm-bindgen-cli` **0.2.130**, matching this workspace's locked dependency.
Run from the repository root:

```sh
python3 site/build.py
python3 -m http.server 8080 --directory site/dist
```

Open `http://localhost:8080`. The deployable output is **`site/dist`**. The build
queries `cargo metadata` from `site/wasm` and uses its actual `target_directory`;
it does not redirect Cargo home, target, temporary directories or global caches.
`site/wasm/Cargo.lock` is committed and builds use `--locked`.

For a fresh Linux builder with rustup already installed, opt into installing the
official toolchain/target/CLI in their normal user locations:

```sh
python3 site/build.py --install-tools
```

Alternatively provision Rust/wasm-bindgen in CI and use the default build command.
`--toolchain 1.95.0` selects an existing toolchain explicitly. The script never
changes the process environment or introduces a replacement cache directory.

## Cloudflare Pages

Use the repository root as the build root, **`bash site/cloudflare-build.sh`**
as the build command (or `python3 site/build.py` on a provisioned runner), and
`site/dist` as the output directory. No environment variables, tokens or worker
bindings are needed by the site. Connect the custom domain through Pages after
the project exists. Git deployment and domain setup are separate from this build.
The Cloudflare wrapper first locates the normal user Rust installation. If
rustup is absent, it downloads the official `https://sh.rustup.rs` installer and
installs the minimal Rust 1.95.0 toolchain in its normal user location, then
sources `$HOME/.cargo/env` for tool lookup. It does not redirect HOME, CARGO_HOME,
target or temporary directories. Existing rustup installations skip bootstrap.
The static `_headers` file declares MIME safety and conservative asset caching.
The host must serve `.wasm` as `application/wasm`.

## Verify

From the repository root, using the existing Node/Playwright dependencies:

```sh
npm ci
npx playwright install chromium
node site/smoke.mjs
```

The smoke script serves `site/dist` itself, opens real Chromium and checks six
live charts, both renderers, changed geometry after a dataset update, tooltip,
desktop/mobile resize, SVG download, and JavaScript/network errors. It writes
JSON and screenshots to ignored `site/verification`. `--url=https://example.com`
instead checks an already deployed site. `--out=<directory>` changes the evidence
directory. No deployment or remote mutation is performed by the smoke test.

The sample values are illustrative. “Download SVG” exports the current author
dataset at the responsive chart dimensions; transient hover, selection and
hidden-legend presentation are not exported. Renderer switching creates fresh
owners and resets transient interaction. Dataset changes are computed in Rust;
the page does not simulate a server feed. Page hiding disposes chart owners each
time; BFCache restoration remounts them with the retained Rust author dataset.
The smoke report separates deterministic lifecycle-handler coverage from actual
trusted BFCache navigations when the browser permits them.
The gallery is a selected introduction,
not a compatibility or performance claim for every Kartis API.
