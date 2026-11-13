# Glory browser Widget acceptance fixture

This independent consumer workspace mounts the real Glory web host. It exercises
Kartis legend proposals, datum-button hover/focus/tooltips, selection, navigation,
clipping, accepted/rejected reactive updates, teardown and remount.
The page also provides a rotated-font-outline example: actual semantic Graphic
glyph paths with explicit font bytes, safe text and stable label slots. This
consumer opts into `raster` for font-outline support; the glyph example itself
does not use a PNG image as its text renderer.

From this directory run:

```powershell
./build.ps1
python -m http.server 8813 --bind 127.0.0.1
```

The build resolves `cargo metadata --no-deps --format-version 1` and uses the
returned `target_directory`. It inherits the caller's Cargo and temporary paths.
Generated browser files live in ignored `pkg/`. Open `http://localhost:8813`.
`window.fixture` exposes acceptance hooks; fixture
startup sets `window.fixtureReady`. The repository's
`scripts/widget-browser-acceptance.js` is a Playwright-style acceptance function
that accepts a connected `page`.

The chart uses fixed logical pixels. Owned labels follow legend visibility;
unowned axis labels remain visible. The mixed paint-order hook verifies dynamic
geometry/text interleaving while preserving the existing DOM nodes. Plot-pointer
geometry and automatic Canvas Widget mounting are not supplied by this fixture.
