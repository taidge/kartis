# Strict live Widget composition

[简体中文](widget-composition.zh-CN.md)

Available in Kartis 0.2.0.

`Chart::plot_widgets_composed` mounts real Glory Widgets and can replace named
Scene mark ink. It is a **terminal builder method**: configure callbacks,
controls and any raster publisher first, then call it last. Existing
`plot_widgets` and `plot_widgets_with_data` remain overlay APIs with their
original local child-revision failure policy.

## One candidate for source, Widgets and paint

`PlotWidgetsComposition<T>` contains `widgets: PlotWidgetsData<T>` and
`replace_ink: BTreeSet<String>`. Every replacement ID must have a measured slot,
and metadata keys must exactly match the unique slot IDs. The 128-slot limit,
real source mark requirement and normal placement/clip validation still apply.
An empty packet retains its `T`. Browser mounts reject a different metadata type
or a switch between overlay and strict composition modes.

The following complete example replaces Bullet measure ink with an actual
Button, retaining ranges, markers, row labels and the original business data.
It derives Scene and composition from one owned controlled source.

```rust
use std::collections::BTreeSet;

use glory::render::EventKind;
use glory::{App, Widget};
use kartis::*;

fn proposal(value: f64)
    -> Result<(Scene, PlotWidgetsComposition<BulletPartWidgetData>)>
{
    let rows = [BulletDatum {
        id: "Revenue".into(), ranges: vec![30., 70., 100.],
        measures: vec![value], markers: vec![80.],
    }];
    let layout = bullet_layout(
        &ChartConfig { width: 600., height: 200., ..Default::default() },
        &rows,
        &BulletLayoutOptions {
            min: Some(0.), max: Some(100.), legend: false,
            ..Default::default()
        },
    )?;
    let widgets = bullet_part_widgets(&layout, 0, |context| {
        if context.part.kind != BulletPartKind::Measure { return Ok(None); }
        let Geometry::Rect { x, y, width, height } = &context.part.geometry
            else { return Err(ChartError("expected a measure rectangle".into())); };
        Ok(Some(PlotWidgetPlacement::Rect(LayoutRect {
            x: *x, y: *y, width: *width, height: *height,
        })))
    })?;
    let replace_ink: BTreeSet<_> = widgets.layout.slots.iter()
        .map(|slot| slot.mark_id.clone()).collect();
    Ok((layout.scene, PlotWidgetsComposition { widgets, replace_ink }))
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let app = App::memory();
    let input = app.scope().cage(proposal(60.)?);
    let scene = app.scope().bond(move || input.with(|p| p.0.clone()));
    let composition = app.scope().bond(move || input.with(|p| p.1.clone()));
    app.mount(chart(scene).legend(false).plot_widgets_composed(
        composition,
        |context| {
            let frame = context.frame;
            Ok(glory::ui::widget(move |scope| {
                let clicks = scope.cage(0usize);
                glory::ui::button()
                    .label(scope.bond(move || frame.with(|current| format!(
                        "Measure {} / {} clicks",
                        current.data.part.raw_value, clicks.get()
                    ))))
                    .on(EventKind::Activate, move |_| clicks.revise(|n| *n += 1))
                    .show_in(scope)
            }))
        },
    ))?;
    let next = proposal(75.)?;
    input.revise(|p| *p = next);
    app.flush()?;
    app.clear()?;
    Ok(())
}
```

Use `context.frame` for live reads. Its `TypedPlotWidgetFrame<T>` contains the
original `Mark`, current slot geometry/visibility and typed source data. The
new `PlotWidgetCompositionContext<T>` is tentative while its factory and child
build execute: a new child is hidden and inert, then suspended until commit.
After commit its Bond follows accepted updates. Rejection disposes that
candidate scope and Bond. Existing children keep reading the old accepted
frame throughout staging; they never observe a rejected proposal.

Same ID and revision retain the child and local state; data and geometry update
on commit. Metadata equality uses `T::PartialEq`, not allocation identity.
Changing a revision may replace its whole wrapper. New-key factory errors,
revision factory errors and child build errors all reject the **entire strict
candidate**. They do not accept new source data into an old child, unlike the
existing overlay revision-retention protocol. Initial child construction also
finishes before the initial PNG publisher is invoked.

Model, slot, metadata, build and synchronous paint/publication failures preserve
the old accepted Scene, typed data, children and base image. Incoming proposed
application sources are not rewritten; correct them and propose again. A
replacement-set change repaints and refreshes picking even if the authored
Scene is unchanged. Retrying a rejected strict revision in a later proposal is
allowed. Side effects that application callbacks perform outside candidate
scopes cannot be rolled back.

## Rendering and input

The authored Scene is retained. Named physical mark ink is removed from the
host's paint projection, without setting `Mark.opacity = 0` or modifying the
mark delivered to a Widget. All Scene labels, axes, titles, legends and canonical
logical controls remain; normal host eligibility still governs their display.
There is no inferred per-mark label ownership: a Bullet row title is not removed
merely because one measure was replaced.

The original named physical geometry is excluded from pointer picking,
including nearest-site pointer queries. Other physical alias IDs are not
implicitly replaced. Logical datums, programmatic selection, explicitly
selected interaction helpers and crosshair annotations keep their existing
policies. Child Widgets handle their own pointer and keyboard actions; their
activation does not automatically select the underlying datum.

Hidden or wholly clipped source slots retain child state while their source
mark and slot remain. Partial slots are visible but inert. A zero effective
mark/group opacity hides and inerts a slot without modifying its source Mark.
Positive group opacity does not automatically reproduce group alpha, blending
or original Scene layering inside an arbitrary Widget. Its appearance belongs
to the Widget. Remove a slot and same-key metadata together; remove its Scene
mark in that same candidate when it is no longer needed.

Widgets occupy a top layer. They do not reproduce Nivo's same-layer React
component order, `animatedProps`, spring trajectory or Scene group blending.
Scene exports do not include live Widgets. Exporting the authored Scene still
draws its original ink; the host's filtered base PNG omits replaced ink and
also omits the live Widget above it.

## Other hosts and bounded staging

For CPU images, configure
`raster_chart(scene, fonts, raster_options).raster_image_source(publisher)`
before the terminal `plot_widgets_composed(composition, factory)` call.
This uses the existing image publication gate. Native applications provide
their own image provider and publisher. It does not add a composition API to
the `RasterSurface` guard. Semantic Graphic and clipping restrictions still
apply to the actual rendered source profile.

Browser SVG and Canvas use `BrowserChart::mount_with_plot_widgets_composed`.
`set_plot_widgets_composed` updates composition alone;
`set_layout_with_plot_widgets_composed` submits Scene, interaction and
composition together. Both setters also exist on `BrowserHandle`. They stage
children before admitting a paint and can reject stale, stopped or reentrant
proposals. Queued work is applied when the host paints or is explicitly flushed.

These rejection guarantees apply before commit. Once all typed rows have been
adopted, ordinary dynamic child bindings run as post-commit UI observations.
Their errors are reported on the accepted frame; already observed data and
external effects are not rolled back. Browser overlay and source-order hosts
use the same distinction.

One host permits at most 256 newly constructed, uncommitted children across
pending stages, in addition to its accepted children. The check occurs before
an extra factory call. Same-revision reuse costs no pending child; retargeted
stages share one reservation. Commit or dropping a rejected/retired stage
releases that reservation once. Flush or retire pending work before queuing
more new revisions; a budget rejection preserves the accepted frame.

See [typed overlay Widgets](typed-plot-widgets.md) for the unchanged overlay
protocol and [Bullet Widgets](bullet-widgets.md) for role-specific metadata.

For browser source-order interleaving and once-only group opacity across actual Widgets, use the separate [Scene-order API](ordered-widgets.md). The top-layer entry described here keeps its existing behavior.
