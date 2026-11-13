# Typed HeatMap and Bar labels

Use a typed, fallible text callback to replace an owned value label while retaining the chart's source data, geometry and interaction metadata. Text and geometry callbacks can be supplied together. Existing entry points and default options keep their previous behavior.

| Chart | Borrowed callbacks | Entry points |
| --- | --- | --- |
| HeatMap | `HeatmapFactories { cell, label }` | `heatmap_layout_with_factories`; `heatmap_layout_with_color_profile_and_factories` |
| Bar | `BarFactories { bar, label }` | `bar_layout_with_factories`; `bar_layout_with_scale_options_and_factories` |

Each field is optional. The HeatMap profile variant accepts the shared `ColorScaleProfile` conversions; the Bar scale variant accepts `ContinuousScaleOptions`. Callbacks return `Result<String>` for text or the existing checked geometry type for ink. They are borrowed only during construction and are never retained by the returned layout. In the example, both closures leave scope before hover updates begin.

`HeatmapLabelContext` borrows the original `HeatCell`, typed row/column key, mark ID and original `source_index`. It also provides nullable `formatted_value`, `default_text`, the band, logical `position`, ordinary `label_position` and plot. `BarLabelContext` provides the original series/category and source indices, mark ID, raw/displayed values, interval, default text, logical position, label position and plot. These fields let equal values in different rows, series or categories receive distinct text without changing their identities.

The complete example uses explicit null and zero HeatMap cells, different row labels for equal values, an empty owned label, a zero-extent Bar label, inherited label-color configuration, custom geometry opacity and controlled-hover restoration.

```rust
use kartis::*;
use std::cell::RefCell;

fn main() -> kartis::Result<()> {
    let config = ChartConfig::default();
    let heat_data = [
        HeatCell { row: "north".into(), column: "same".into(), value: None },
        HeatCell::new("south", "same", 0.),
        HeatCell::new("north", "other", 0.),
        HeatCell::new("east", "empty", 1.),
    ];
    let heat_events = RefCell::new(Vec::new());
    let mut heat = {
        let mut text = |context: &HeatmapLabelContext<'_>| {
            heat_events.borrow_mut().push(("label", context.source_index));
            Ok(if context.datum.row == "east" {
                String::new()
            } else {
                format!("{} / {}: {}", context.datum.row, context.datum.column, context.default_text)
            })
        };
        let mut cell = |context: &HeatmapCellContext<'_>| {
            heat_events.borrow_mut().push(("cell", context.source_index));
            let mut mark = context.inherit(context.geometry.clone());
            mark.opacity = 0.5;
            Ok(mark)
        };
        heatmap_layout_with_factories(
            &config,
            &heat_data,
            &HeatmapLayoutOptions {
                axes: HeatmapAxes::none(),
                hover_target: HeatmapHoverTarget::Cell,
                empty_label: "NA".into(),
                label_color_profile: Some(InheritedColor::Fixed(Color(40, 50, 60, 101))),
                opacity: 0.8,
                ..Default::default()
            },
            HeatmapFactories { cell: Some(&mut cell), label: Some(&mut text) },
        )
    }?;
    assert_eq!(*heat_events.borrow(), vec![
        ("label", 3), ("cell", 3), ("label", 2), ("cell", 2),
        ("label", 0), ("cell", 0), ("label", 1), ("cell", 1),
    ]);
    assert_eq!(heat.scene.labels.iter().map(|label| label.text.as_str()).collect::<Vec<_>>(),
        vec!["", "north / other: 0", "north / same: NA", "south / same: 0"]);
    assert!(heat.scene.labels.iter().all(|label| label.color == Color(40, 50, 60, 40)));
    let binding = heat.interaction_binding()?;
    assert!(binding.sites.iter().any(|site| site.value.is_none()));
    assert!(binding.sites.iter().any(|site| site.value == Some(0.)));
    let base = heat.scene.clone();
    heat.set_active(Some(HeatmapCellKey::new("north", "same")))?;
    heat.set_active(None)?;
    assert_eq!(heat.scene, base);
    assert_eq!(heat_events.borrow().len(), 8);

    let categories = ["A", "B", "C"].map(str::to_string);
    let data = [BarSeries::new("values", vec![Some(0.), None, Some(5.)])];
    let bar_events = RefCell::new(Vec::new());
    let mut bar = {
        let mut text = |context: &BarLabelContext<'_>| {
            bar_events.borrow_mut().push(("label", context.source_category));
            Ok(format!("{}={}", context.category, context.default_text))
        };
        let mut geometry = |context: &BarDatumContext<'_>| {
            bar_events.borrow_mut().push(("bar", context.source_category));
            let mut mark = context.inherit(context.geometry.clone());
            mark.opacity = 0.5;
            Ok(mark)
        };
        bar_layout_with_factories(
            &config,
            &categories,
            &data,
            &BarOptions { value_domain: Some([0., 10.]), ..Default::default() },
            &BarStyleOptions {
                labels: BarLabels {
                    enabled: true,
                    zero_extent: BarZeroExtentLabels::ValueAxis,
                    color_profile: Some(InheritedColor::Fixed(Color(40, 50, 60, 101))),
                    ..Default::default()
                },
                ..Default::default()
            },
            BarFactories { bar: Some(&mut geometry), label: Some(&mut text) },
        )
    }?;
    assert_eq!(*bar_events.borrow(), vec![("label", 0), ("bar", 0), ("label", 2), ("bar", 2)]);
    let zero = bar.layout.bars.iter().find(|datum| datum.raw_value == 0.).unwrap();
    let label = zero.label_index.unwrap();
    assert_eq!(bar.layout.scene.labels[label].text, "A=0");
    assert_eq!(bar.layout.scene.labels[label].color, Color(40, 50, 60, 51));
    assert_eq!(bar.interaction_binding()?.sites.len(), 2);
    let base = bar.layout.scene.clone();
    let active = bar.layout.bars.iter().find(|datum| datum.raw_value == 5.).unwrap().id.clone();
    bar.set_active(Some(&active), 0.2)?;
    assert_eq!(bar.layout.scene.labels[label].color, Color(40, 50, 60, 10));
    bar.set_active(None, 0.2)?;
    assert_eq!(bar.layout.scene, base);
    assert_eq!(bar_events.borrow().len(), 4);
    let _svg = render::to_svg(&bar.layout.scene)?;
    let _script = render::to_canvas_script(&heat.scene, "heatmap")?;
    Ok(())
}
```

For each eligible label, its text callback runs once before that source's geometry callback. HeatMap uses configured row/column order; Bar uses its ordinary source order. Text callbacks skip hidden or absent sparse sources and disabled labels. Bar also skips `None` values, fully cropped bars, and labels rejected by the ordinary visibility, position or size thresholds. A geometry callback can still run for a visible mark whose label is disabled or skipped. These are callback counts for accepted Rust label slots; they do not promise Nivo's accessor invocation counts for labels that are not rendered.

Explicit HeatMap `None` and `Some(0.)` remain distinct and both can receive text. A null cell's `default_text` comes from `empty_label`; even an empty default can be replaced. Returning an empty string keeps the owned `Label`, row association and slot, while drawing no glyphs. The same empty-result rule applies to Bar text.

`BarLabels.zero_extent` defaults to `BarZeroExtentLabels::Skip`. Opt into `ValueAxis` to permit a zero visible extent along the value axis when the visible category breadth is positive and the original, unclamped endpoints project to the same finite pixel. This includes a genuine zero interval or collapse caused by scale rounding. Cropping a nonzero interval to the plot edge does not qualify. Existing `skip_width`, `skip_height`, ordinary anchor placement, plot containment and clipping rules still apply; a custom geometry callback does not create eligibility for an otherwise skipped label.

Custom text changes only the owned label's `text`. Raw/null values, numeric `formatted_value`, tooltips, source indices, IDs and interaction metadata retain their original meanings. `default_text` is computed by the existing value/empty-label formatting. The text callback does not replace tooltip formatting or parse a dynamic property path.

Label colors continue to use the configured [inherited label-color roles](inherited-label-colors.md), resolved from the original palette and theme before custom geometry. Text does not change that context, and a custom fill's representative color does not silently become the inherited label color. Existing alpha composition remains in effect: the role's color alpha is multiplied by the geometry factory's factor and the chart's ordinary base/active/inactive factors. The example's HeatMap label has alpha `round(101 × 0.5 × 0.8) = 40`; the Bar label has `round(101 × 0.5) = 51`, and becomes `10` when inactive at `0.2`.

`set_active` and `interaction_binding` use the accepted source and owned-label frame. Hover changes opacity without running either callback again; clearing hover restores the accepted frame. After a factory layout is accepted, tampering with owned text, row/series ownership, source data or the accepted geometry/paint makes checked binding or hover operations fail. Controlled hover rejects atomically. Default layouts created without factories keep their previous Scene-decoration behavior.

When a text factory is supplied, configuration and the planned Scene budget are checked before callbacks: count the actual labels that will be emitted, including existing axes and Bar totals, rather than a dense row-by-column product. The Scene limit remains 20,000 labels, and each returned string is checked immediately against the existing 1,024-byte UTF-8 limit. Total bounded label storage follows those limits. An error or oversized result rejects construction without changing borrowed input or an already accepted frame. Side effects performed by a caller's closure, such as logging or incrementing a counter, are not rolled back. With `label: None`, the existing geometry-only callback and error order is preserved.

These APIs produce ordinary Scene labels and renderer-independent marks for SVG, CanvasScript and CPU rendering. They do not provide an arbitrary Glory Widget slot, and this guide makes no cross-browser pixel-acceptance claim. See [custom HeatMap cells](heatmap-custom.md) and [custom Bar geometry](bar-custom-geometry.md) for the geometry contracts.
