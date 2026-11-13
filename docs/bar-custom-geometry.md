# Custom Bar geometry and paint

The typed Bar factory produces one renderer-independent `Scene` mark per computed bar. It accepts the same `Geometry`, `FillPaint`, stroke, dash and anchor contract as custom Scatter/Line points. Existing Bar entry points retain their default behavior; the factory interface and its integration status are tracked separately in [verification](verification.md).

```rust
use kartis::*;
use kartis::scales::BandScaleOptions;

fn main() -> kartis::Result<()> {
    let config = ChartConfig {
        width: 420.,
        height: 260.,
        ..Default::default()
    };
    let categories = ["A", "B", "C"].map(str::to_string);
    let series = [BarSeries::new("sales", vec![Some(4.), None, Some(0.)])];
    let style = BarStyleOptions {
        category_scale: Some(BandScaleOptions {
            padding_inner: 0.2,
            padding_outer: 0.2,
            round: true,
            ..Default::default()
        }),
        labels: BarLabels {
            enabled: true,
            clip_to_plot: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut frame = bar_layout_with_bar_factory(
        &config,
        &categories,
        &series,
        &BarOptions::default(),
        &style,
        |context| {
            let [x, y] = context.position;
            let glyph = glyph_geometry(
                [x + 18., y],
                &GlyphOptions {
                    shape: GlyphShape::Filled(FilledGlyph::Diamond),
                    size: 280.,
                    ..Default::default()
                },
            )?;
            let mut node = context.inherit(glyph.geometry);
            node.anchor_commands = Some(glyph.anchor_indices);
            node.opacity = 0.8;
            node.fill_paint = Some(FillPaint::Linear {
                start: [x + 8., y],
                end: [x + 28., y],
                stops: vec![
                    GradientStop::new(0., Color(230, 60, 20, 255)),
                    GradientStop::new(1., Color(20, 80, 220, 255)),
                ],
            });
            Ok(node)
        },
    )?;
    let binding = frame.interaction_binding()?;
    assert_eq!(binding.sites.len(), 2);
    assert!(binding.sites.iter().any(|site| site.value == Some(0.)));
    if let Some(id) = binding.sites.first().map(|site| site.mark_id.clone()) {
        frame.set_active(Some(&id), 0.25)?;
        frame.set_active(None, 0.25)?;
    }
    let _svg = render::to_svg(&frame.layout.scene)?;
    let _canvas_script = render::to_canvas_script(&frame.layout.scene, "bars")?;
    Ok(())
}
```

This example moves the painted diamond 18 logical pixels to the right of its data site. The generated diamond is a Path, so the logical site stays at `context.position`. The `Some(0.)` source still invokes the factory and remains a real zero in the binding. The `None` source creates no bar, callback or site. The ordinary value-label eligibility rules still apply; a zero-height default bar is not forced to display a value label by replacing its geometry.

## Source and geometry contract

`bar_layout_with_bar_factory` returns `CustomBarLayout`; its public `layout` is the ordinary `StyledBarLayout`. The scale-options variant, `bar_layout_with_scale_options_and_bar_factory`, returns `ScaledCustomBarLayout` with the resolved value scale. Both use the same numeric layout, category mapping, clipping, source identity and interval calculation as the existing Bar functions. [Category band options](bar-category-scale.md) continue to work with either entry point.

The context borrows the original `BarSeries` and category and supplies stable mark ID, original series/category indices, raw value, displayed value, interval, category slot/breadth, visible rectangle, original ordinary geometry, inherited color/stroke and plot. `original_rect` is optional when an uncropped rectangle cannot be represented; `endpoints` contains optional finite unclamped value projections. These fields are separate from the visible rectangle. Factory callbacks follow the existing lexical visible-series order and authored category order, retaining the original indices. Hidden series, missing values, fully cropped bars and zero-bandwidth slots do not invoke callbacks. Inheritance copies the ordinary solid fill/border with opacity 1; returning a custom paint does not change the numeric domains or raw values.

The logical anchor is fixed before custom ink exists: it is the ordinary Rect's center, or the visible rectangle's center when the ordinary geometry is a Path. A zero-area rounded Rect placeholder retains its authored center through category cropping. This anchor is not claimed to be the unclipped source midpoint; use `original_rect` and `endpoints` for uncropped information. `context.position` is not recomputed from the custom mark's bounding box.

Custom Rects and Circles must remain centered on that logical position, including the shared checked center-rounding rule. Use a Path for offset ink, a hollow shape, multiple contours or an asymmetric outline. Optional `anchor_commands` select the same checked Path endpoint supports as Scatter/Line; compatible animation follows their mean displacement. The accepted frame uses the exact supplied logical position. Actual `Scene` hit testing uses the painted geometry, stroke/dash and plot clip, while closest-point bindings use logical sites. A logical site is not an invisible filled hit rectangle.

## Paint, labels and controlled hover

The factory owns one mark's geometry, solid fill, linear/radial gradient or pattern, stroke, dash, opacity and optional Path anchor supports. The layout assigns the mark ID, series, original raw value, tooltip text and plot clipping. The existing numeric label/total layout stays in charge of positioning, formatting, skip conditions and source ownership. Bar value-label alpha is its configured original alpha multiplied by the factory opacity. Axis, legend and total labels remain independently owned. Returned `StyledBarDatum.color` retains the numeric palette color; typed interaction sites use the prepared paint's representative color.

`set_active(Some(mark_id), inactive_factor)` multiplies each inactive bar's original opacity by the supplied factor. The active bar keeps its original opacity. The factor must be finite in [0,1], and unknown IDs return `Err`. `set_active(None, factor)` restores the base frame. Owned value-label alpha is rebuilt from the original configured color and base opacity on every update, avoiding repeated multiplication or rounding. Paint, geometry, source values and clips remain unchanged.

A custom layout privately retains its accepted complete frame, source sites and anchors. Both `interaction_binding()` and `set_active()` validate against that private frame. Mutating public geometry, paint, source metadata, opacity, labels, `label_series`, `label_clips` or draw order is not a supported update and returns an error before a new controlled frame is published. Hover updates validate a complete candidate before replacing the current frame. Rebuild through the factory for resize, hidden-series changes, new data or a different style; this recalculates slots, intervals, totals and sites with stable source identities. Generic Scene-only legend filtering cannot perform that numeric relayout.

## Scope and limits

This is a typed single-mark geometry/paint callback, not an arbitrary Glory Widget slot, React `barComponent`, Canvas context callback or child-widget lifecycle. It does not store application callbacks for later invocation. Applications connect accepted scenes and bindings to their existing hosts and drive controlled hover explicitly.

Factory errors or invalid outputs reject the candidate layout. Existing finite-coordinate and paint rules remain in force. Admission counts existing axes and all custom outputs: at most 200,000 source Path commands, 16,384 gradient stops, and the shared dash limits of 200,000 generated commands/flattened segments and 4,000,000 work units. Dash admission happens before retaining another callback result. Final `Scene` validation remains authoritative for the shared geometry, clip and rendering contracts. These limits bound returned resources; they do not sandbox caller-owned Rust code or intercept callback panics.
