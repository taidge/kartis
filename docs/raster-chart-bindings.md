# Typed input on CPU image Charts

This API is under development after 0.1.0. A Chart can retain its complete
`SceneInteraction` while drawing a CPU PNG:

```rust,ignore
chart_layout(frame)
    .raster(fonts, options)
    .on_physical_hover(|hit| { /* actual ink identity and source data */ });
```

The frame must have an explicit `PickingProfile::PaintedGeometry` binding.
Canonical business values, including `None` versus `Some(0.0)`, remain separate
from paint scalars and physical aliases. Existing typed overlay Widgets and
`plot_widgets_composed` use the same image publication transaction. A publisher
returning `Err` retains the previous image, binding, camera, controls and accepted
Widget data. Later asynchronous image transport or decode errors remain host
observations and cannot undo an accepted model.

Glory receives finite plot input on a physical View containing the Image.
Pointer coordinates are normalized to image logical dimensions; the accepted
`RasterOptions.view` is inverted once during picking. Device pixel ratio affects
PNG resolution, not input coordinates. Picking checks both the displayed image
viewport and the original source plot. Click movement is measured before camera
inversion. A camera change cancels an active gesture, renews physical-hit frame
identity and repicks a stationary pointer.

Measured tooltips use the typed anchor transformed by the accepted camera.
Widgets remain actual Glory children with their own input; an image pixel is
never converted into a synthetic child click. This is the existing overlay or
strict replacement composition, without arbitrary live children inside a PNG.

Scene-only `raster_chart` does not infer a typed binding. `RasterSurface` retains
its separate model and input APIs. Interactive host support is required: the
Glory web host accepts positive axis-aligned scaling and translation, whereas
its native plot-input host rejects transformed viewport ancestors. Native image
applications must explicitly provide image loading and source publication.
Memory dispatch tests prove the port contract, not operating-system input.
Automatic springs, arbitrary live Widget export and GPU/worker rendering remain
separate work.

[中文](raster-chart-bindings.zh-CN.md)
