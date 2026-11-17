# Stream dot Widgets

This development API is not included in the published 0.1.0 package.
`stream_dot_widgets` supplies the role of Nivo's Stream `dotComponent` using
real Glory children and owned Rust data. It works with typed overlays, strict
replacement composition, CPU image Charts and browser Scene-order composition.
The [complete runnable example](../examples/stream_widgets.rs) uses Memory;
applications select their actual Glory host.

```rust
use glory::{App, Widget};
use kartis::*;
# fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
let series = [BarSeries::new("orders", vec![Some(8.), Some(0.), Some(13.)])];
let layout = stream_layout(
    &ChartConfig::default(),
    &["first".into(), "second".into(), "third".into()],
    &series,
    &StreamLayoutOptions { dots: Some(StreamDotOptions::default()), ..Default::default() },
)?;
let packet = stream_dot_widgets(&layout, 0, |_| Ok(Some(
    PlotWidgetPlacement::AtDatum {
        offset: [-18., -12.], measured: TextSize { width: 36., height: 24. },
    }
)))?;
let mut frame = layout.into_slice_interaction()?;
frame.interaction.as_mut().unwrap().picking_profile = PickingProfile::PaintedGeometry;
let app = App::memory();
app.mount(chart_layout(frame).plot_widgets_with_data(packet, |context| {
    let frame = context.frame;
    Ok(glory::ui::widget(move |scope| {
        glory::ui::button().label(scope.bond(move || frame.with(|f|
            format!("{}: {:?}", f.data.layer_label, f.data.sample.value)
        ))).show_in(scope)
    }))
}))?;
app.clear()?;
# Ok(()) }
```

Each `StreamDotWidgetData` retains an `Arc<BarSeries>`, original series index,
layer ID/label/color, stack order, `StreamSample`, and plot bounds. The source
layer color remains separate from the current custom dot's paint. The sample contains
the typed category, source column index, raw value, formatted value, unscaled
stack interval, projected boundaries and configured dot position. Slots from
the same source series share one snapshot. Read the accepted host frame's Mark
for actual current geometry, paint and opacity. No compatibility paint scalar
is promoted into a missing source value.

Placement receives a borrowed `StreamDotWidgetContext` and may return `None` to
skip a role. The entire accepted source, computed layout, full scale configuration,
actual Stream marks and bindings are checked before the first placement call.
To change source styles or geometry, build a new layout, optionally with
`stream_layout_with_dot_factory`; editing the generated Stream marks or source
metadata after construction is rejected. Other source errors fail atomically.
External callback side effects remain application-owned. At most 128 selected
slots may be admitted by a host.

Enabled non-null dots are component roles, including `Some(0.0)`, size zero and
custom opacity zero. Hidden layers are omitted. `dots: None` and null samples
retain sampling placeholders and slice data, but do not create dot components.
The shared original series still contains its nulls. Changing a null into zero
adds that previously absent child. This is the explicit Rust nullable contract;
Nivo's public Stream datum type is numeric. Start/Center/End use the configured
first boundary, midpoint or second boundary, including reversed screen ranges.

Stable mark ID plus revision retains a child's state while its accepted data
updates. A new revision remounts; disabled or removed roles retire their owners.
This identity contract differs from Nivo's React column-index keys. Host-level
series hiding retains existing scopes but hides and disables their children.
Partially clipped children remain visible and inert under the shared slot policy.

Bound Charts now resolve `AtDatum` through the accepted `InteractionBinding`,
including Paths with displaced ink. The original source anchor determines the
slot; an accepted image camera is applied afterwards. A Scene-only Path without
an explicit anchor still requires `Rect` placement. Binding-only changes are
validated with the Scene and packet. Strict composition and PNG publication
errors retain accepted source data, controls, child identity, focus and pixels.

Nivo reference checks execute the original StreamDots call arguments and enable
gates; they do not prove React lifecycle or pixel equivalence. Memory protocol
injection is not OS input evidence. Automatic springs, React `animatedProps`,
arbitrary live Widget exports and GPU/worker rendering remain separate work.

[中文](stream-widgets.zh-CN.md)
