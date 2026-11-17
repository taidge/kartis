# Original values in retained plot Widgets

Available in Kartis 0.2.0. Bump, Scatter, Bar, Stream, Heatmap and Bullet typed packets distinguish observable original `-0.0` and `+0.0` metadata. A data-only update can refresh a mounted child while its Scene, measured placement, business identity and revision remain equal. Ordinary chart geometry and category-key equality keep their numeric semantics.

| Role | Original values compared exactly |
| --- | --- |
| Bump | Complete owned series: numeric categories and nullable ranks; current point category/rank |
| Scatter | Complete owned series: numeric X/Y and nullable sizes; current site's X/Y/size |
| Bar | Complete owned nullable series values and current raw bar value |
| Stream | Complete owned nullable series values and current sample category/value |
| Heatmap | Original nullable datum value and computed cell's source value |
| Bullet | Current part raw value and original interval endpoints |

Shared source arrays use an `Arc` identity fast path. Independently allocated equal inputs still compare equal by content. Exact original-number comparison retains ordinary NaN inequality; valid layout inputs are finite. This does not change public `CategoryKey`, `Mark`, scale or layout geometry equality. Bullet exposes the current part, not every original range/measure/marker array. A zero-sized Waffle group owns no cell, so it cannot be selected as that cell's source payload.

Use a stable Widget revision for value updates; change it when child construction must run again. Custom payload type `T` remains responsible for its own `PartialEq`: derive-based numeric equality treats signed zeros as equal. Keep original-source values in the payload if your child observes them.

This complete Memory application updates an actual Button label from negative-zero bits to positive-zero bits:

```rust
//! Observe original-value updates independently of equal chart paint.
use glory::render::Props;
use glory::{App, Widget};
use kartis::*;
fn source(value: f64) -> Result<(SceneInteraction, PlotWidgetsData<BumpPointWidgetData>)> {
    let layout = bump_layout(
        &ChartConfig::default(),
        &[BumpSeries::new("row", vec![BumpDatum::new("point", value)])],
        &BumpLayoutOptions {
            axes: BumpAxes::none(),
            legend: false,
            labels: BumpEndpointLabels {
                start: false,
                end: false,
                ..Default::default()
            },
            ..Default::default()
        },
    )?;
    let packet = bump_point_widgets(&layout, 0, |_| {
        Ok(Some(PlotWidgetPlacement::AtDatum {
            offset: [-40., -12.],
            measured: TextSize {
                width: 80.,
                height: 24.,
            },
        }))
    })?;
    Ok((bump_widget_frame(&layout)?, packet))
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let before = source(-0.)?;
    let after = source(0.)?;
    assert_eq!(before.0, after.0); // ordinary numeric Scene equality
    assert_ne!(before.1, after.1); // observable original-source bits
    let app = App::memory();
    let input = app.scope().cage(before);
    let frame = app.scope().bond(move || input.with(|v| v.0.clone()));
    let packet = app.scope().bond(move || input.with(|v| v.1.clone()));
    app.mount(
        chart_layout(frame).plot_widgets_with_data(packet, |context| {
            let frame = context.frame;
            Ok(glory::ui::widget(move |scope| {
                glory::ui::button()
                    .class("raw-example")
                    .label(scope.bond(move || {
                        frame.with(|f| format!("{:016x}", f.data.point.rank.unwrap().to_bits()))
                    }))
                    .show_in(scope)
            }))
        }),
    )?;
    input.revise(|old| *old = after);
    app.flush()?;
    {
        let tree = app.tree();
        let mut pending = tree.roots().to_vec();
        let mut label = None;
        while let Some(id) = pending.pop() {
            let node = tree.node(id).unwrap();
            let view = node.descriptor.view();
            if view.classes.iter().any(|c| c == "raw-example")
                && let Props::Button(p) = view.props
            {
                label = Some(p.label.clone());
            }
            pending.extend(node.children.iter().copied());
        }
        assert_eq!(label.as_deref(), Some("0000000000000000"));
    }
    app.clear()?;
    Ok(())
}
```

Run it with `cargo run --example widget_source_values`. Source provenance checks also reject changed Stream numeric-category bits and Heatmap values before placement callbacks. These checks protect the original snapshot without broadly redefining presentation equality.

## Browser verification boundary

The six-role browser fixture wraps only real role payloads; it adds no phase or cached-bit field to bypass their equality. It explicitly authors each zero-valued compatibility `Mark.value` as positive zero in both phases. Typed original values stay untouched. This makes the fixture's source SVG byte-equal; ordinary unmodified SVG metadata can spell `-0` differently from `0`.

A stable reference rectangle keeps an actual source SVG/Canvas plane present even if the child replaces the role's sole mark. The driver requires the requested renderer and nonempty source planes. Independent exact bit strings, real retained-child state/focus, actual DOM rectangles within 1/32 logical pixel and nine solid-color foreground samples per frame within ±1 per channel establish the functional update. Rejected candidates must preserve accepted state, complete root DOM, computed SVG paint styles, native Canvas backing bytes and viewport geometry. Expected business errors are checked; JavaScript TypeError and WebAssembly traps fail validation.

Whole-viewport PNG hashes are recorded separately. The earlier strict whole-viewport gate remains failed: a script-free native SVG carrier reproduced one changing antialias pixel without WASM or a candidate update; the prescribed integer alignment did not resolve it. Functional acceptance does not establish strict whole-viewport stability. Solid font-zero Buttons do not establish arbitrary glyph equivalence; the earlier Bar font8 compositor failure also remains open.

See [typed packets](typed-plot-widgets.md), [sampling](viewport-sampling.md), the [runnable example](../examples/widget_source_values.rs), and [中文指南](widget-source-values.zh-CN.md). Automatic sampled-motion components, arbitrary Widget exports, native same-layer input and GPU/worker work remain separate gaps.
