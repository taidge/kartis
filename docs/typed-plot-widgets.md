# Typed live plot Widgets

[简体中文](typed-plot-widgets.zh-CN.md)

`Chart::plot_widgets_with_data` mounts real Glory Widgets in measured slots
above a chart. Each child receives an accepted `TypedPlotWidgetFrame<T>`:
the current mark, placement, clipping and interactivity, together with owned
application metadata. This is useful for interval controls, editors and
role-specific controls whose source cannot be reconstructed from `Mark.value`.
`T` must implement `Clone + PartialEq + 'static`; it need not implement `Debug`.

## One controlled source

Keep Scene and slot metadata in one controlled input, then derive both `Vary`
inputs from it. Read `context.frame` inside child Bonds and event handlers.
Reading the proposed input directly from the child would bypass rejection:
that input can already contain a candidate which the host has not accepted.

This complete example mounts a real Memory-hosted Button. Both data-only and
complete frame updates retain its key and local state.

```rust
use std::collections::BTreeMap;

use glory::render::EventKind;
use glory::{App, Widget};
use kartis::*;

#[derive(Clone, PartialEq)]
struct Payload {
    interval: [f64; 2],
    text: String,
}

fn proposal(value: f64, text: &str) -> Result<(Scene, PlotWidgetsData<Payload>)> {
    let mut scene = ChartConfig::default().scene()?;
    let mut mark = Mark::new(
        "revenue",
        Geometry::Circle { x: 100., y: 80., radius: 10. },
        Color(35, 100, 185, 255),
    );
    mark.series = "sales".into();
    mark.label = "Revenue".into();
    mark.value = value;
    scene.marks.push(mark);
    let slots = PlotWidgetsData {
        layout: PlotWidgetsLayout {
            plot: None,
            slots: vec![PlotWidgetSlot {
                mark_id: "revenue".into(),
                revision: 0,
                placement: PlotWidgetPlacement::AtDatum {
                    offset: [-40., -12.],
                    measured: TextSize { width: 80., height: 24. },
                },
            }],
        },
        data: BTreeMap::from([("revenue".into(), Payload {
            interval: [0., value], text: text.into(),
        })]),
    };
    Ok((scene, slots))
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let app = App::memory();
    let input = app.scope().cage(proposal(10., "Accepted")?);
    let scene = app.scope().bond(move || input.with(|p| p.0.clone()));
    let slots = app.scope().bond(move || input.with(|p| p.1.clone()));
    app.mount(chart(scene).plot_widgets_with_data(slots, |context| {
        let frame = context.frame;
        Ok(glory::ui::widget(move |scope| {
            let count = scope.cage(0usize);
            glory::ui::button()
                .class("revenue-editor")
                .label(scope.bond(move || frame.with(|accepted| {
                    format!("{} {:?}: {} / {} clicks",
                        accepted.data.text,
                        accepted.data.interval,
                        accepted.frame.mark.value,
                        count.get())
                })))
                .on(EventKind::Activate, move |_| count.revise(|n| *n += 1))
                .show_in(scope)
        }))
    }))?;

    // Metadata-only update; the Scene and child revision remain identical.
    input.revise(|p| p.1.data.get_mut("revenue").unwrap().text = "Updated".into());
    app.flush()?;

    // Build successfully before replacing the single controlled input.
    let next = proposal(20., "Next frame")?;
    input.revise(|p| *p = next);
    app.flush()?;
    app.clear()?;
    Ok(())
}
```

`PlotWidgetsData<T>` is owned input. It accepts the same value, `Cage` and `Bond`
forms as other `Vary` APIs. Metadata is compared by `PartialEq` value, rather
than allocation address. Replacing equal metadata does not require a new child;
changing metadata updates the accepted live frame even when the Scene is equal.
The library validates packet structure, not arbitrary business invariants in `T`.

## Identity and rejection

- `layout.slots` contains at most 128 unique real `mark_id`s, and `data` must
  contain exactly those keys. Missing or extra metadata, duplicate slots,
  missing source marks and invalid placements reject the candidate.
- `mark_id` retains the keyed wrapper across reordering and source changes.
  A stable `revision` retains the child, its local state and focus eligibility.
  Change `revision` when a different Widget implementation is needed.
- Scene, slot placement and typed source are prepared together. Invalid model,
  slot or packet input preserves the previous accepted geometry and metadata.
  A rejected proposed `Cage` value remains application-owned; the host does not
  rewrite that proposed input to its old value. Correct it and propose again.
- Handle a fallible candidate-builder or placement callback before publication.
  Its `Err` produces no packet to publish. Application callback side effects
  remain application-owned.
- Child factory or child build failure preserves the preceding child at that
  key and reports the normal rejection/error callback. The chart frame and
  packet can already be accepted: the retained child then reads the **new
  accepted data** through its frame Bond. This is child revision retention,
  not a whole-frame rollback. A failed revision is not retried automatically;
  choose a different revision to retry. Returning to the preceding accepted
  revision keeps that child.

Empty typed packets still have a metadata type: use
`PlotWidgetsData::<Payload> { layout: PlotWidgetsLayout::default(), data:
BTreeMap::new() }`. Browser mounts retain that type even when no slots remain.
A typed mount rejects later plain packets or a different `T`; a plain mount
also rejects typed updates. No existing public struct gained a field, and
`Chart::plot_widgets` and the plain browser APIs keep their existing behavior.

## Hosts and placement

Browser SVG and Canvas share real Glory child Widgets through
`BrowserChart::mount_with_plot_widgets_data`. Use `set_plot_widgets_data` for
typed slot updates, or `set_layout_with_plot_widgets_data` to submit Scene,
interaction metadata, slots and typed source together. Both update methods
are also available on `BrowserHandle`. Use the typed interaction frame when a
Path requires its logical datum anchor. Camera transforms are applied once;
slot dimensions are logical and independent of Canvas DPR.

Bound `chart_layout(frame)` Charts now use that same accepted interaction
anchor for `AtDatum`, including custom Paths. This also applies after `.raster`:
the source anchor is resolved before the accepted image camera is applied.
Scene-only Paths continue to require explicit rectangles. See
[Stream dot components](stream-widgets.md) for an adapter that retains source
sampling and actual displaced Path paint separately.

For CPU images with live children, use
`raster_chart(scene, fonts, raster_options).plot_widgets_with_data(slots,
factory)`. The existing raster publication transaction covers image rendering,
encoding, source publication, accepted Scene and typed slots. A synchronous
publisher `Err` preserves them together. Native applications still supply their
own image provider and publisher. This API belongs to the `Chart` returned by
`raster_chart`; it does not add an overlay API to the `RasterSurface` guard.

Slots use explicit measured rectangles or `AtDatum` offsets. Host hiding and
clipping retain child state while the source mark and slot remain present.
To retire a child, remove its slot and same-key metadata in one candidate;
submit any removal of its Scene mark in that candidate as well. Removing only
the mark while retaining its slot rejects the `Chart` candidate. Partially clipped children paint their visible area
but are inert; they cannot receive focus or pointer activation through that
partial slot. Browser path masks use the existing mask host; semantic and
raster slots reject a partial nonrectangular path mask. Child input belongs to
the Widget itself and does not automatically select the underlying chart mark.
Popups follow Glory's separate host placement and input policy.

Live Widgets are overlays: the original Scene ink remains. SVG, Canvas and PNG
exports contain that Scene and omit live child Widgets. This is not a React
component or `animatedProps` execution layer. For typed Bullet range, measure
and marker slots, see [Bullet Widgets](bullet-widgets.md).


Explicit typed packets can also attach to a real decorative Mark with an empty
label, including a Waffle empty cell. Exact metadata keys declare that intent;
the original Mark remains unchanged. Such a slot does not create a source
datum, control or picking site. The untyped slot API keeps its data-Mark rule.
See [HeatMap and Waffle cell adapters](cell-widgets.md).
