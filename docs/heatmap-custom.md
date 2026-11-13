# Custom HeatMap cells

`heatmap_layout_with_cell_factory` replaces each visible source cell with one checked Scene mark. `heatmap_layout_with_color_profile_and_cell_factory` also selects the shared interpolation profile or named color scheme. Both retain the ordinary HeatMap layout, nullable metadata and controlled-hover behavior.

```rust
use kartis::*;
use glory::render::{GraphicStroke, StrokeCap};

let config = ChartConfig {
    width: 320.,
    height: 180.,
    margin: Margin { left: 20., right: 20., top: 20., bottom: 20. },
    ..Default::default()
};
let cells = [
    HeatCell { row: "row".into(), column: "empty".into(), value: None },
    HeatCell::new("row", "zero", 0.),
    HeatCell::new("row", "value", 7.),
];
let options = HeatmapLayoutOptions {
    axes: HeatmapAxes::none(),
    labels: false,
    hover_target: HeatmapHoverTarget::Cell,
    ..Default::default()
};
let layout = heatmap_layout_with_color_profile_and_cell_factory(
    &config, &cells, &options, ColorScheme::Viridis, |cell| {
        let [x, y] = cell.position;
        let mut ink = cell.inherit(Geometry::Circle { x, y, radius: 14. });
        match cell.datum.value {
            None => ink.fill_paint = Some(FillPaint::Pattern(PatternFill::default())),
            Some(0.) => {
                ink.fill = None;
                ink.stroke = Some(GraphicStroke {
                    color: Color(20, 30, 40, 255), width: 2., cap: StrokeCap::Butt,
                });
            }
            Some(_) => {}
        }
        Ok(ink)
    },
)?;
let active = layout.with_active(Some(HeatmapCellKey::new("row", "empty")))?;
let binding = active.interaction_binding()?;
assert!(binding.sites.iter().any(|site| site.value.is_none()));
assert!(binding.sites.iter().any(|site| site.value == Some(0.)));
let svg = render::to_svg(&active.scene)?;
assert!(svg.contains("<pattern"));
# Ok::<(), kartis::ChartError>(())
```

The callback receives `HeatmapCellContext`: the original datum, typed row/column key, stable mark ID, original input `source_index`, formatted value, resolved band, logical data position, plot, configured geometry, color and border. Callbacks follow the configured row/column order. Hidden cells and sparse absent pairs do not invoke them; an explicit `None` cell does. `None` and `Some(0.)` stay distinct in `ComputedHeatmapCell`, tooltips and interaction bindings, although generic Scene marks use a finite zero compatibility scalar for nulls.

`HeatmapCellGeometry` reuses the Scatter/Line `ScatterNodeGeometry` contract. Return an actual `Geometry` with solid fill, `FillPaint`, stroke, dash and an opacity factor. Rectangles and circles remain centered at `position`. A Path can place visible ink away from that logical origin; its checked anchor supports the original position independently of ink picking. `HeatmapLayout::hit_test` uses the actual painted geometry. Custom marks are clipped to the plot.

`cell.inherit(geometry)` keeps the resolved fill and border and starts with opacity factor 1. The factor multiplies the configured base, active or inactive opacity, including owned-label alpha. Existing labels retain their configured value/empty text, position and row ownership. Per-cell label accessors and arbitrary Glory Widget children are separate capabilities; this factory returns one Scene mark per cell.

Color domains are resolved before callbacks under the existing HeatMap visibility policy. A custom gradient or pattern does not rewrite the resolved `color_scale` or its numeric legend. Custom-cell `color` and binding colors use the effective paint representative. Use `append_color_legend` with the returned scale when a data-color legend is appropriate for the authored encoding.

Custom layouts retain private accepted metadata, marks and owned labels. `set_active`, `with_active` and `interaction_binding` reject altered custom geometry, paint, source data or label ownership before returning another frame. The new binding API also checks original identities, nullability, formatted values, band and logical origin for default layouts. Ordinary default Scene appearance and decorations remain editable; the legacy default hover behavior is preserved. Data, visibility or size changes require a fresh layout. Callback errors, invalid geometry/paint and existing Scene/gradient/dash/anchor budgets reject the candidate.

## Standalone consumer

Run `cargo run --features raster,geojson --example heatmap_custom -- OUTPUT_DIRECTORY`, replacing `OUTPUT_DIRECTORY` with the desired output directory. The example registers the bundled ABeeZee fixture through `FontRegistry::register` and exports `index.html`, two SVGs, two CanvasScripts, four CPU PNGs at DPR 1/2 and `receipt.json`.

The example draws an explicit null as a striped diamond, zero as a hollow circle and a positive value as a gradient diamond. Its second frame activates the null cell and dims the other cells. The same Rust Scenes feed SVG, CanvasScript and CPU rasterization. Chart labels are disabled to isolate geometry and paint; the example does not establish generic-font parity.

[`scripts/heatmap-custom.js`](../scripts/heatmap-custom.js) is an async Playwright page verifier for the generated page. A caller supplies and navigates the page, then evaluates the verifier function. It checks typed source values, actual SVG geometry, decoded SVG/Canvas whole-frame mean-channel error below 0.5, source-position pattern/gradient/hollow pixels and restored DOM/Canvas mutation controls. It returns geometry, pixel witnesses and PNG data URLs for artifact capture. The fixed consumer is exercised at DPR 1/2; browser events, arbitrary Widget factories and physical-device behavior are outside this check.
