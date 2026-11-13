# CPU raster in a Glory image

This independent consumer enables `raster` and Glory Web. Rust draws paths and
explicit font outlines, encodes an actual PNG, then publishes it through the
standard Glory image widget. It does not capture SVG. The fixture registers the
unmodified ABeeZee font; its SIL Open Font License is in
`../../tests/fonts/ABeeZee-OFL.txt`.

```powershell
./build.ps1
python -m http.server 8815 --bind 127.0.0.1
```

The build queries this consumer's Cargo metadata and uses its own target path.
Generated bindings are under ignored `pkg/`. Open <http://127.0.0.1:8815>.
From the Kartis workspace, with the service running:

```powershell
$env:PLAYWRIGHT_CHANNEL = 'chrome'
npm run test:browser -- --raster http://127.0.0.1:8815 target/gallery/raster-browser-report.json
```

The script decodes the real `<img>` PNG and samples geometry and text pixels.
It verifies reactive dimensions/DPR, retained image identity, legend/data/tooltip
controls, publication/option failures keeping the previous frame, and callback
cleanup/remount. The temporary sampling canvas is test tooling.

The default image source is a PNG data URL. Native applications must provide
an explicit image provider and `Chart::raster_image_source` publisher according
to their host's image-source policy; a string source does not grant native I/O.
Native pointer coordinates and device-window acceptance are not claimed here.
Text shaping is explicit left-to-right with registered font fallback, not bidi
layout or cross-font shaping. Missing glyphs follow RasterOptions.
