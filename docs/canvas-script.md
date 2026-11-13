# Explicit CanvasScript surfaces

`render::to_canvas_script` and `render::to_canvas_script_with_blends` keep their existing Direct replay. `CanvasScriptOptions` adds explicit main-thread Offscreen2D staging through `to_canvas_script_with_options` and `to_canvas_script_with_blends_and_options`. This does not create a worker, transfer ownership of the destination, install events or introduce a GPU backend.

```rust
use kartis::{render::*, *};

let mut scene = line(
    &ChartConfig::default(),
    &[Series::new("values", vec![Point::new(0., 1.), Point::new(1., 2.)])],
    &LineOptions::default(),
)?;
scene.typography.default.family = Some(vec![FontFamily::System]);
let options = CanvasScriptOptions {
    surface: CanvasScriptSurface::Offscreen2d {
        fallback: CanvasScriptFallback::Error,
    },
    ..Default::default()
};
let script = to_canvas_script_with_options(&scene, "chart", &options)?;
# Ok::<(), kartis::ChartError>(())
```

The application creates the named HTML canvas and executes the returned synchronous script after loading fonts and establishing the intended document layout. The example explicitly chooses the authored System family; that choice is independent of the surface option. Sans remains `sans-serif`, System remains `system-ui`, and named families remain quoted CSS names. No family is substituted. Offscreen contexts do not have a DOM language ancestor; generic-family, locale and fallback-font resolution can differ from HTML Canvas or SVG across engines. The option does not resolve every existing default-Sans comparison or promise identical fonts on all platforms.

`Offscreen2d` draws the complete Scene into a staging canvas, then copies physical pixels to the same HTML canvas. Paths, ordered drawing, gradients, patterns, clips, rotated labels and isolated mark blends use the existing replay. DPR calculation stays the same. An existing destination with a reported non-sRGB or alpha-disabled 2D context is rejected for staging. With `CanvasScriptFallback::Error`, missing OffscreenCanvas, constructor failure or missing 2D context throws a JavaScript error. `Direct` permits the existing Direct replay for those availability failures. Allocation-budget, painting or publication errors do not silently select another surface.

`max_extra_buffer_pixels` applies to Offscreen-mode planning, including availability fallback: one offscreen frame when its constructor is available, all distinct pattern tiles, and the maximum shared blend scratch. The destination is excluded. The checked range is 1..=134217728 pixels and the default is 67108864. Existing individual pattern, work and blend limits still apply. A pixel budget counts backing pixels rather than certified process memory; browser overhead and pixel formats are separate. Direct mode preserves its existing budgets and output.

Aggregate admission precedes extra-buffer allocation or destination resizing. A created staging object is explicitly resized to zero on success, paint failure, publication failure, and a failed 2D-context acquisition. Existing pattern and blend scratch lifecycle is unchanged. This is staging, not the BrowserChart transaction protocol: before publication a rejected proposal can preserve the old bitmap, but publication resizes the visible destination before `drawImage`. If that call fails, the old bitmap can already be cleared. No rollback or old-frame restoration is promised for publication failure. Direct fallback retains the existing Direct behavior.

Capability checks are explicit. An engine without Offscreen2D can exercise Error or Direct fallback; that result is not evidence that it rendered through Offscreen2D. Frame comparisons and font profiles must report which path actually ran.
