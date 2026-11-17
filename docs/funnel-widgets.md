# Funnel functions and live custom-layer roles (Kartis 0.2.0)

[简体中文](funnel-widgets.zh-CN.md)

Available in Kartis 0.2.0.

Nivo Funnel exposes function-valued `layers`, with computed parts, before/after
separators, area/border generators and a current-part setter. It does not expose
`partComponent`, `labelComponent` or `separatorComponent`. Kartis offers explicit
Rust function configuration and a typed live Widget adapter for actual Funnel
marks. The role adapter is a Rust extension to that custom-layer use case.
`funnel_layout_with_stage_factory` changes Scene geometry; it does not create a
Glory Widget.

## Function configuration and timing

`FunnelFactories` borrows callbacks for one layout build:

These adapters cover Nivo's function-valued `colors`, `valueFormat`,
`borderColor` and `labelColor` inputs with explicit Rust types.

| Field | Input and result |
| --- | --- |
| `color` | `FunnelColorContext`: original `datum`, source index, default color and plot; returns a color. It overrides the resolved palette/explicit datum color without changing the raw datum. |
| `formatted_value` | Original `f64` value; returns an owned string. Without it, `FunnelLayoutOptions.value_format` remains authoritative. |
| `border_color` | `FunnelPartStyleContext`: computed part, plot, default color, fill/border opacity and current border width; returns a color. |
| `label_color` | Computed style context including the resolved border color; returns a color. |

Use `funnel_layout_with_factories`, `funnel_layout_with_size_and_factories`, or
`funnel_layout_with_factories_and_stage_factory`. Existing constructors retain
their default output. Source validation and the complete size batch precede
color, formatter and stage-style callbacks. The latter run in source order,
including computed stages whose body, border or label is absent. Explicit style
callbacks can run even when the corresponding paint role is disabled; returning
a label color does not enable labels.

This is not Nivo's callback schedule: Nivo computes size and style per datum,
and its style functions receive a temporary part before profile population.
The Rust style context already has the completed four-sample profile. Do not
use side effects to assume identical callback interleaving.

An explicit label-color callback is not replayed after custom body geometry;
its result is retained. Without that callback, inherited label color follows
replacement body color as in the existing stage factory. An unused disabled
`InheritedColor::Contrast` border keeps its previous admission behavior. An
explicit style callback requests a resolved default context: if that default
cannot be resolved on a transparent surface, construction can fail before the
callback runs. Supplying a callback does not bypass default-context validation.
Errors return no layout; external callback side effects are not rolled back.

## Original profile paths and actual custom ink

`funnel_part_geometry(part, interpolation)` returns the original closed profile
or `None` when its two sides coincide. `funnel_part_border_geometry` returns the
two original open sides, even when border paint is disabled. Both require an
explicit `FunnelInterpolation`; a public part does not retain that option.
All eight profile points must have finite coordinates within the Scene budget.

These geometry tools adapt the area/border-generator role without providing
React generator objects. A custom stage may paint an offset Path or another
silhouette. Its current geometry and paint belong to the actual Scene/host
`Mark`, and can differ from the original profile returned by these helpers.

## Actual roles and complete owned data

`funnel_widgets(&layout, original_source, revision, placement)` returns
`PlotWidgetsData<FunnelWidgetData>`. The synchronous placement context borrows
the full source, layout, optional datum/part/separator, actual Mark, label pool,
clips and series ownership. References are not retained. Returning `None` skips
that role. The selected-role limit is 128 across all four roles:

| Role | Ownership and position |
| --- | --- |
| `Body` | An actual emitted stage body; original `part.center`. |
| `Border` | An actual profile-border Mark; the same original part center. |
| `BeforeSeparator` | An actual before-separator Mark; endpoint midpoint. |
| `AfterSeparator` | An actual after-separator Mark; endpoint midpoint. |

Placement follows actual Scene mark order, including explicit draw order. An
absent body creates no slot. A zero start width can still taper to a nonzero next
stage and produce a body; fully coincident sides do not. Explicit all-zero
pixel sizes produce no bodies, whereas the default all-zero value domain uses
half-width bodies. Opacity-zero and no-ink actual Marks remain eligible. A stage
with no body does not run the body-geometry factory, so its independent border
is not removed by that callback. Host hidden state controls visibility/input
without fabricating or discarding source data.

Closing separators have `source_index: None` and `part_index: None`; their
`separator_index` still identifies the real separator. They are not attached to
an invented last datum. `AtDatum` becomes an explicit rectangle at the original
part center or separator midpoint plus the measured offset. A custom body's
bounding-box center does not replace that anchor.

Every selected role shares one complete source `Arc`, one parts `Arc`, one
separators `Arc`, and one `FunnelWidgetLabels` `Arc`, allocated on first selection.
The label pool contains current Scene labels, indexed clips and series-owner
maps; an application can find a part's labels by `part.series`. Reading these
labels does not suppress or duplicate their independent Scene paint. Empty text,
multiple labels owned by one stage, and absent labels remain distinct cases.

The full original values, optional sizes and numeric category keys preserve raw
signed-zero bits in payload equality, including unselected siblings and the raw
datums inside computed parts. Current geometry keeps ordinary numeric equality.
A resolved part color can differ from `datum.color` or raw paint. Actual current
Mark paint remains available through the accepted host frame.

Complete source, profile/bounds/center, separator, Mark-role, label-owner and
private binding checks happen before the first placement callback. This is
current consistency validation, not immutable authentication of the original
constructor: public layout fields and an explicitly supplied coherent source
can be edited together. `funnel_widget_frame(&layout)` has no external original
source to authenticate; it validates the current layout, clones the Scene and
canonical body binding, retains the plot, and selects `PaintedGeometry`.
Borders and separators do not create canonical data controls or sites.

## One controlled source and real child state

The following complete program is identical to
[the runnable example](../examples/funnel_widgets.rs). It derives the frame and
packet from one owned controlled input, mounts actual Glory Buttons, updates the
source and checks retained local click state. Memory dispatch verifies the
Widget protocol; it is not operating-system input evidence.

```rust
//! A complete application-owned Funnel layer with retained Glory controls.
use glory::render::{Event, EventKind, NodeId, Props};
use glory::{App, Widget};
use kartis::*;
#[derive(Clone, Debug)]
struct Source {
    frame: SceneInteraction,
    composition: PlotWidgetsComposition<FunnelWidgetData>,
}
fn source(data: &[FunnelDatum]) -> Result<Source> {
    let config = ChartConfig::default();
    let options = FunnelLayoutOptions {
        labels: None,
        before: Some(FunnelSeparatorOptions {
            length: 100.,
            offset: 10.,
            stroke: None,
        }),
        after: None,
        ..Default::default()
    };
    let mut color = |ctx: &FunnelColorContext<'_>| {
        Ok(if ctx.datum.value >= 60. {
            Color(25, 140, 80, 255)
        } else {
            Color(20, 100, 180, 255)
        })
    };
    let mut format = |value: f64| Ok(format!("{value:.0} tickets"));
    let layout = funnel_layout_with_factories(
        &config,
        data,
        &options,
        &mut FunnelFactories {
            color: Some(&mut color),
            formatted_value: Some(&mut format),
            ..Default::default()
        },
    )?;
    // Explicit interpolation turns an original part profile into geometry.
    // Actual custom-stage ink remains available on its host Mark instead.
    for part in &layout.parts {
        let _ = funnel_part_geometry(part, options.interpolation)?;
    }
    let widgets = funnel_widgets(&layout, data, 0, |ctx| {
        let selected = match ctx.role {
            FunnelWidgetRole::Body => ctx.source_index == Some(0),
            FunnelWidgetRole::BeforeSeparator => ctx.source_index == Some(1),
            _ => false,
        };
        Ok(selected.then_some(PlotWidgetPlacement::AtDatum {
            offset: [-40., -12.],
            measured: TextSize {
                width: 80.,
                height: 24.,
            },
        }))
    })?;
    let replace_ink = widgets
        .layout
        .slots
        .iter()
        .map(|slot| slot.mark_id.clone())
        .collect();
    Ok(Source {
        frame: funnel_widget_frame(&layout)?,
        composition: PlotWidgetsComposition {
            widgets,
            replace_ink,
        },
    })
}
fn children(app: &App) -> Vec<NodeId> {
    let tree = app.tree();
    let mut pending = tree.roots().to_vec();
    let mut result = vec![];
    while let Some(id) = pending.pop() {
        let node = tree.node(id).unwrap();
        if node
            .descriptor
            .view()
            .classes
            .iter()
            .any(|c| c == "funnel-demo")
        {
            result.push(id);
        }
        pending.extend(node.children.iter().copied());
    }
    result
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut data = vec![
        FunnelDatum::new("visits", 80.),
        FunnelDatum::new("sales", 40.),
    ];
    let app = App::memory();
    let input = app.scope().cage(source(&data)?);
    let frame = app.scope().bond(move || input.with(|s| s.frame.clone()));
    let packet = app
        .scope()
        .bond(move || input.with(|s| s.composition.clone()));
    app.mount(chart_layout(frame).plot_widgets_composed(packet, |ctx| {
        let frame = ctx.frame;
        Ok(glory::ui::widget(move |scope| {
            let count = scope.cage(0u32);
            glory::ui::button()
                .class("funnel-demo")
                .label(scope.bond(move || {
                    frame.with(|f| {
                        let part = &f.data.parts[f.data.part_index.unwrap()];
                        format!(
                            "{:?}: {} {}, clicks={}",
                            f.data.role,
                            part.datum.label,
                            part.formatted_value,
                            count.get()
                        )
                    })
                }))
                .on(EventKind::Activate, move |_| count.revise(|n| *n += 1))
                .show_in(scope)
        }))
    }))?;
    let before = children(&app);
    assert_eq!(before.len(), 2);
    for id in &before {
        app.dispatch(*id, Event::Activate)?;
    }
    app.flush()?;
    data[0].value = 60.;
    let next = source(&data)?;
    input.revise(|old| *old = next);
    app.flush()?;
    assert_eq!(children(&app), before);
    for id in before {
        let tree = app.tree();
        let Props::Button(button) = tree.node(id).unwrap().descriptor.view().props else {
            panic!("not a Button")
        };
        assert!(button.label.contains("clicks=1"));
    }
    app.clear()?;
    Ok(())
}
```

Run it with `cargo run --example funnel_widgets`.

Read `context.frame` in child Bonds and event handlers so rejected proposals do
not leak through a captured proposed input. Same mark ID/revision retains the
child; a new revision requests a new child scope. Strict composition rejects
invalid model/packet/placement, child construction and paint-publication
candidates together. Callback-owned external effects remain outside rollback.
Configure the Chart before the terminal `.plot_widgets_composed(...)` call.

`FunnelLayoutOptions.current`, current-size extension and current border width
are caller-controlled. A host selection or hover does not automatically rebuild
that layout. Map the accepted body identity back to its datum, update the
controlled source/current option, rebuild the layout and packet, then submit
them together. Borrowed layout factories are not retained for future events.

## Host order, replacement and scope

The example uses native strict composition above Scene paint. Overlay
`plot_widgets_with_data` retains source ink and its existing local child-failure
policy. Strict `PlotWidgetsComposition.replace_ink` explicitly names measured
source Marks whose physical ink is replaced; an empty replacement set keeps it.
Source Scene and canonical logical controls are retained, while replaced source
ink is excluded from physical picking.

Browser `mount_with_plot_widgets_in_scene` and
`set_layout_with_plot_widgets_in_scene` place Widgets at authored Scene mark
positions; a kept mark paints before its Widget, and later Scene items can cover
it. `mount_with_plot_widgets_composed` and its matching setter use a top layer.
Use the same metadata type and composition mode for a mount. See
[typed packets](typed-plot-widgets.md), [strict composition](widget-composition.md)
and [Scene-order Widgets](ordered-widgets.md) for admission, placement, clipping,
rejection and resource budgets. This role adapter does not add an arbitrary
Nivo-style layer list or a label-Widget role.

CPU image Charts can use `chart_layout(frame).raster(fonts, options)` with the
same packet and strict composition; fonts and a publisher are application
inputs. Scene exports contain Scene paint, not live Widgets, their state or a
browser DOM. Source-order behavior is not a claim of native operating-system
input, React spring/animated-prop equivalence, arbitrary datum-field recovery,
GPU rendering, or complete Nivo parity.

## Evidence and rendering boundaries

Focused native tests cover source-bit preservation, source/profile/role
conflicts before callbacks, shared pools, zero/collapsed profiles, custom offset
anchors, explicit paths, accepted child state, rejected candidates, and CPU
image publication. Browser cases exercise actual SVG/Canvas carriers, measured
DOM bounds, trusted clicks and keyboard activation, source selection, hidden
state, controlled current changes, reorder, resize, camera and revision updates.
Final counts and preserved failures are in the [verification record](verification.md#development-funnel-function-and-live-layer-roles).

The browser fixture declares source labels at 20px and Widget content at 0px:
Widgets paint solid measured Buttons while text/metadata and local state stay
observable through the DOM. It does not certify Widget glyph rasterization.
Foreground and retained source-ink probes use a per-channel RGBA tolerance of
one; DOM geometry is checked within `1/32` Scene pixel. Unique label/border source colors
are counted globally within the same RGBA tolerance, with exact-color counts
also retained and an actual pixel/expected/delta witness recorded. That census
proves visible unique source paint, not each glyph contour or each stage's
stroke coverage. Whole-viewport PNG hashes are diagnostic, not a strict
bit-identical screenshot pass; rollback separately checks accepted state, DOM,
computed SVG paint, Canvas backing PNG bytes and capture geometry.
