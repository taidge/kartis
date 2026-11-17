# Bump point components

This API is available in Kartis 0.2.0.
`bump_point_widgets` adapts actual `BumpLayout` point marks into measured, typed
live Glory components. It composes with ordinary point geometry, custom Paths,
endpoint text factories and the existing SVG, Canvas and CPU-image hosts.

Each placement callback borrows the current layout, computed series, original
series and datum, computed point and actual Mark. Returned data owns one shared
`Arc<BumpSeries>` per selected series, the original source index and typed X,
nullable rank, mapped position, representative point color and plot. Numeric
`1` and string `"1"` remain distinct; original numeric sign bits survive. Current
geometry and paint come from the accepted host Mark, independently of source data.

The entire frame is validated before the first callback: source/computed data,
point-scale domains/ranges, mapped positions, line runs, point identity and
actual sites/anchors must agree. Hidden series and gaps do not invent roles;
actual zero-size and transparent marks still have roles. Returning `None` skips
a selected component, and callback errors return no packet. Arbitrary callback
side effects are not transactional. The public `BumpLayout` structure is unchanged;
this checks a self-consistent current frame, not private historical provenance.
A fully consistent public edit can be accepted. The original style/accessor or
point factory cannot be recovered from that frame.

`AtDatum` uses the original data anchor even for a custom Path whose ink is
offset. `bump_widget_frame` retains the exact plot, including an empty layout,
and explicitly selects painted picking. It does not change the older
`into_scene_interaction` behavior. Shared-series snapshots do not follow later
source mutations. Mark/site representative color, series line color and actual
paint have separate meanings.

```rust
//! Live Glory components for Bump ranking points.
use glory::{App, Widget};
use kartis::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let series = [BumpSeries::new(
        "north",
        vec![
            BumpDatum::new(1_f64, 2.),
            BumpDatum::gap("middle"),
            BumpDatum::new("tail", 0.),
        ],
    )];
    let layout = bump_layout(
        &ChartConfig::default(),
        &series,
        &BumpLayoutOptions::default(),
    )?;
    let widgets = bump_point_widgets(&layout, 0, |_| {
        Ok(Some(PlotWidgetPlacement::AtDatum {
            offset: [-20., -10.],
            measured: TextSize {
                width: 40.,
                height: 20.,
            },
        }))
    })?;
    let replace_ink = widgets
        .layout
        .slots
        .iter()
        .map(|slot| slot.mark_id.clone())
        .collect();
    let frame = bump_widget_frame(&layout)?;
    let app = App::memory();
    app.mount(chart_layout(frame).plot_widgets_composed(
        PlotWidgetsComposition {
            widgets,
            replace_ink,
        },
        |context: PlotWidgetCompositionContext<BumpPointWidgetData>| {
            let frame = context.frame;
            Ok(glory::ui::widget(move |scope| {
                let count = scope.cage(0u32);
                glory::ui::button()
                    .label(scope.bond(move || {
                        frame.with(|f| {
                            format!(
                                "{}: {:?}, rank {:?} ({} clicks)",
                                f.data.series.label,
                                f.data.point.raw_x,
                                f.data.point.rank,
                                count.get()
                            )
                        })
                    }))
                    .on(glory::render::EventKind::Activate, move |_| {
                        count.revise(|v| *v += 1)
                    })
                    .show_in(scope)
            }))
        },
    ))?;
    app.flush()?;
    app.clear()?;
    Ok(())
}
```

Nivo's normal compute pipeline filters null-rank points before creating
`pointComponent` elements, retains raw source indices and keeps line gaps.
Zero values and size0 points remain actual components. Its default Point reads
only position, size/color and border properties, animates them through React
Spring, clamps a negative animated radius to zero, and uses pointer-events:none;
line/mesh layers own business input. Kartis uses stable typed-category business
IDs instead of Nivo's series-plus-source-index IDs. Live Glory components may
receive their own input, and Rust's controlled Mark opacity remains distinct
from Nivo's default Point behavior. This adapter does not reproduce React
reconciliation, SpringValues or automatic spring timing.

Use the [complete application](../examples/bump_widgets.rs) or shared host model
for updates and custom geometry. [Transactional composition](widget-composition.md)
controls replacement, revisions, rejection and scope cleanup;
[Scene-order browser composition](ordered-widgets.md) supplies paint interleaving.
Plain overlay mode remains available through `plot_widgets_with_data`.
Scene exports contain chart geometry and labels, not arbitrary live Widget
snapshots. Native OS input, automatic springs and GPU/worker remain separate work.
The solid-color browser fixture preserves JSON data in real Buttons with font0;
it validates role/layout/input and does not claim arbitrary glyph equivalence.

The focused SVG/Canvas matrix covers four browsers at DPR1/2: 192 frames,
11,856 gated RGBA samples, trusted child/source input, revisions and rollback.
Four independent mutated-driver controls reject at their intended gates.
Run the portable checks after installing the pinned project Node dependencies:

```sh
node scripts/bump-point-widget-reference.mjs --frozen
node scripts/bump-widgets-acceptance.mjs --chromium-only
```

See [verification](verification.md#development-bump-live-point-components) for
exact scope and retained failures; this is not a complete catalog acceptance.
