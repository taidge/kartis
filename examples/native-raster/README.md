# Glory native chart examples

## Headless Apace raster

This independent consumer runs a real Glory Apace document, image decoder,
native input bridge and CPU document renderer. It opens no OS window and grants
no network or filesystem resource access. PNG and TTF bytes are explicitly
published through a bounded application-owned `asset://raster/` provider.

From this workspace:

```powershell
$metadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
$output = Join-Path $metadata.target_directory 'native-raster-apace.png'
cargo run --locked -- $output
```

The program writes the specified PNG and an adjacent JSON acceptance report.
The output path is the only application filesystem write.
`cargo clippy --locked --all-targets -- -D warnings`
checks this consumer separately from the browser/WASM fixtures.

`RasterSurface::mount_published(scope, scene, fonts, options, publisher)` produces
one checked CPU frame before committing its model and image source. Retain its
guard for the mounted lifetime, render `surface.widget()` once, and use the weak
`surface.handle()` in application callbacks. The widget displays an actual
Glory image plus controlled legend, data and tooltip controls. Those controls
feed the same `SurfaceModel`; they do not rerender through a second raster widget.
`on_event` observes accepted interaction results. `on_error` observes rejected
control proposals. Explicit handle methods return their errors directly.

The sample publisher encodes the supplied RGBA frame as PNG and returns a unique
asset URL. It retains at most 64 assets and 32 MiB, accepts only its own asset
host, bounds pending jobs to 32, and polls `NetHandler::prepare` before delivery.
Cancel and close revoke queued jobs; closing the document releases retained
assets. The chart font and the Apace `@font-face` resource both use the explicitly
embedded, unmodified ABeeZee font under `tests/fonts/ABeeZee-OFL.txt`. No font file
is discovered or opened by this application. Apace owns its normal font fallback
engine; this fixture verifies successful registration of the supplied face.

Glory's finite semantic pointer observations do not carry coordinates or press
phases. The sample's `host_pointer` is therefore the native host entry point:
it sends the actual `UiEvent` to `WindowApp`, checks Apace's image hit, queries the
real image bounds, and forwards the input through `handle.dispatch_host`.
`RasterSurfaceBounds` and host coordinates must use the same units and origin;
the controller converts offset/scaled coordinates to logical chart pixels.
Applications must also forward captured release/cancel events outside the image.
Keyboard and legend controls are tested through Apace's actual hit/focus and
`UiEvent::KeyDown` paths, without fabricating Glory callbacks.

The executable checks decoded PNG/color/font pixels, nonzero host offsets,
selection, tooltip, legend pixel hiding, keyboard navigation/activation, resize
and DPR decode, transactional invalid-scene rejection, weak-handle retirement,
and provider close cleanup. These are headless native document/input-protocol
checks, **not physical mouse/keyboard, OS window, accessibility or device
acceptance**. No `Shell::run`, event-loop creation or visible native window is
invoked. An application may host the same widget in a manually launched Glory
window, but that is outside this acceptance result.

A separate document in the same executable first mounts the original
`raster_chart(...).raster_image_source(...)` API directly, and verifies its real
Apace PNG pixels and native legend redraw. The controller acceptance then uses
its own single-render image/control bridge. The report records both paths.

Rendering/encoding/source-publication failures retain the accepted model and
frame. A provider's later asynchronous transport/decode failure is a host
observation and cannot roll back an already accepted application transaction.

## Desktop window

Run the same reactive Glory chart with either desktop host from this example's
workspace.

```sh
cargo run --locked --features window --bin kartis-native-window -- apace apace-window.json
cargo run --locked --features wry-window --bin kartis-native-window -- wry wry-window.json
```

Toggle a region using the legend, navigate to a data button with Tab, and press
Enter to select it. Update the values, then submit an invalid frame to observe
the last valid plot stay visible. Closing the window writes the observed Rust
events to the supplied JSON path. Apace draws Glory's semantic vector graphics.
Wry uses the CPU raster widget and `Chart::basic_data_controls` for Tab focus and
button activation. The headless example also exercises PNG publication and CPU
image decoding through an explicit Apace asset provider.
