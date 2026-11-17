# Live Tree node and link Widgets (Unreleased)

[简体中文](tree-widgets.zh-CN.md)

Development API, not included in the published 0.1.0 package.

Nivo Tree has real SVG `nodeComponent` and `linkComponent` props, plus Canvas
`renderNode` and `renderLink` functions. Kartis supplies a typed Tree layout and
actual Glory node/link Widgets for this customization use case. The Rust APIs
do not execute React components or JavaScript Canvas renderers. A later additive [label component API](tree-label-components.md) adds real Label
targets alongside node/link Widgets. The separate Canvas `renderLabel` callback
and exported arbitrary controls remain outside these Widget APIs.

## Captured layout and source

`tree_layout(config, data, options)` returns `TreeLayout` with public `scene`,
`plot`, `nodes` and `links`. `source()`, `options()` and `root_index()` expose the
captured flat input, constructor options and root source index. Node metadata
is indexed in original source order; link metadata follows the original
breadth-first edge order. `tree_with_options` uses the same authoritative
calculation and returns its Scene. Existing default Scene/SVG output is retained.

Each `TreeNode` includes source/parent/children indices, depth and height, raw
value, aggregate `value`, original position, resolved radius, constructor style
and fill color, and an optional actual Mark ID. Each `TreeLink` carries its
source/target source indices, original endpoints, their true midpoint, link
index and optional Mark ID. `style` and `color` describe constructor metadata;
read the accepted host frame's actual Mark for later custom geometry and paint.

Rust input is a flat `HierarchyDatum` array with globally unique string IDs,
one root and checked parents/cycles. Internal raw values must be zero;
`node.value` is the subtree aggregate used by compatible Scene scalars. Nivo's
inspected Tree hooks use nested data and dot-joined path UIDs, retain original
`node.data`, and do not call `hierarchy.sum`. Rust aggregates are a separate
existing model, not a reconstructed Nivo raw datum. Node Mark IDs remain
`node:<id>` and link IDs `link:<target-id>`. Reparenting a target can retain that
link's key; this is not Nivo path-UID identity.

Tidy uses the existing Buchheim calculation, and Cluster uses the existing Rust
cluster convention. Neither is certified numerically identical to every Nivo
Tree/dendogram layout. The four Cartesian orientations have a fifth, Rust radial
extension. The shared `TreeOptions.hierarchy.include_root` field has no effect
for Tree: the root is always emitted, preserving previous behavior.

Exactly zero is now accepted for `TreeOptions.node_radius` and
`TreeNodeStyle.radius`. The existing positive style-radius exclusion
`(0, 1e-6)` remains; this restriction concerns the style override. Zero radius
creates actual zero-size Circle/Rect/Path roles without creating a paint
footprint. A border can independently contribute extent and ink.

## One typed packet for both roles

`tree_node_widgets`, `tree_link_widgets` and `tree_widgets` take the layout,
revision and a borrowed synchronous placement callback. The mixed callback sees
`TreeWidgetContext::Node` or `::Link`; the mixed payload uses
`TreeWidgetData::Node` or `::Link`. Node context borrows its original datum,
computed node and actual Mark. Link context borrows both endpoint datums,
computed link and actual Mark. Each context also exposes the layout and full
captured source. Returning `None` skips a role. References and callbacks are not
retained; callback-owned external effects cannot be rolled back.
Nivo event/tooltip props, `setCurrentNode`, `linkGenerator` and Spring
`animatedProps` are not injected into Glory factories; applications use their
Glory child events and accepted frame data.

Selected roles share the complete source, nodes, links and current label pools
through `Arc`. The source is captured once at construction; computed/label
packet pools are allocated once on first selection, rather than cloned per
slot. `TreeWidgetLabels` retains current labels, indexed clips and series-owner
maps, including moved or measured labels. The shared selected-slot budget is
128 across both roles, not 128 for each role.

Only actual Marks create roles. Disabled links retain complete computed metadata
with `mark_id: None`, and create no link callback/Widget. Actual zero-radius,
opacity-zero and no-fill/no-stroke nodes remain eligible; host clipping, hidden
state, opacity and capabilities determine visible paint and physical input.
Hidden host state retains the underlying source and child state.

`AtDatum` becomes an explicit rectangle using the original node center or true
endpoint midpoint plus the measured offset. It does not use a curved link's or
custom Path's bounding-box center. Make measured placements fit the plot when
interactive controls are required: a partially clipped slot can be inert. The
example places its leaf control inward rather than crossing the plot boundary.

Before any callback, validation checks the complete privately captured
source/topology/positions/constructor eligibility against current computed
metadata, all managed actual Marks and label ownership. Managed computed fields
cannot be edited into a new topology; construct a new layout instead. Legal
current Scene paint, offset Path geometry and consistently updated label
text/placement/clips remain supported. Circle/Rect shapes keep the original
logical center; offset silhouettes use Paths. Current extra decoration does not
become a node/link source role.

`tree_widget_frame(&layout)` clones the current Scene with the accurate plot
and explicit `PaintedGeometry` binding. Canonical business sites belong only to
labeled nodes. Links remain decorative physical roles, and an empty node label
does not acquire a fabricated business label/control. A labeled no-ink node can
remain a logical keyboard row without acquiring a physical paint footprint.
Current Path anchors resolve to the captured original node center.

## Exact original bits and compatible scalar text

Owned source values and `node.raw_value` retain exact numeric bits, including
unselected siblings. A `-0` to `+0` source update therefore refreshes retained
Widget data even when layout geometry and Scene compare equal. This does not
change ordinary `Scene`/`Mark` numeric equality or make their scalars into raw
source snapshots.

For example, a retained canonical row for source `b` can keep scalar text `-0`
after its exact typed source becomes `+0`. The browser fixture allows only the
strict set `"0"`/`"-0"` for this numeric-zero compatibility scalar, while checking
original source bits separately and exactly. Nonzero values must update exactly:
the fixture also changes leaf `c` from 30 to 35 and root/`a` aggregates from 40
to 45. No stale nonzero scalar or arbitrary alternate text is accepted. Do not
infer original value bits from canonical text or `Mark.value`.

## Complete controlled application

This program is identical to [the runnable example](../examples/tree_widgets.rs).
It derives Scene/binding and the mixed composition from one owned controlled
source, creates actual Buttons, updates a nonzero leaf value and checks retained
local click state. Memory dispatch verifies the Widget protocol, not operating system input.

```rust
//! A complete Tree application with retained node and link components.
use glory::render::{Event, EventKind, NodeId, Props};
use glory::{App, Widget};
use kartis::*;
#[derive(Clone, Debug)]
struct Source {
    frame: SceneInteraction,
    composition: PlotWidgetsComposition<TreeWidgetData>,
}
fn source(data: &[HierarchyDatum]) -> Result<Source> {
    let layout = tree_layout(&ChartConfig::default(), data, &TreeOptions::default())?;
    let widgets = tree_widgets(&layout, 0, |ctx| {
        let offset = match ctx {
            TreeWidgetContext::Node(ctx) if ctx.datum.id == "visits" => [-84., -12.],
            TreeWidgetContext::Link(ctx) if ctx.target_datum.id == "sales" => [-40., -12.],
            _ => return Ok(None),
        };
        Ok(Some(PlotWidgetPlacement::AtDatum {
            offset,
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
        .map(|s| s.mark_id.clone())
        .collect();
    Ok(Source {
        frame: tree_widget_frame(&layout)?,
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
            .any(|c| c == "tree-demo")
        {
            result.push(id);
        }
        pending.extend(node.children.iter().copied());
    }
    result
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut data = vec![
        HierarchyDatum::new("root", None, 0.),
        HierarchyDatum::new("visits", Some("root".into()), 80.),
        HierarchyDatum::new("sales", Some("root".into()), 40.),
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
                .class("tree-demo")
                .label(scope.bond(move || {
                    frame.with(|f| match &f.data {
                        TreeWidgetData::Node(d) => format!(
                            "Node {}: raw={}, subtree={}, clicks={}",
                            d.source[d.source_index].label,
                            d.node.raw_value,
                            d.node.value,
                            count.get()
                        ),
                        TreeWidgetData::Link(d) => format!(
                            "Link {} -> {}, clicks={}",
                            d.source[d.link.source_index].label,
                            d.source[d.link.target_index].label,
                            count.get()
                        ),
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
    data[1].value = 60.;
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

Run it with `cargo run --example tree_widgets`.

Read the accepted `context.frame` inside child Bonds and event handlers. Capturing
a proposed source directly would bypass rejected-candidate semantics. Same ID
and revision retains the child; an explicit new revision requests a new scope.
Keep source/frame/packet updates together. A selection or hover does not rebuild
Tree data or reproduce Nivo active-size modifiers automatically; applications
own the controlled update.

## Composition, hosts and exports

The terminal `Chart::plot_widgets_composed` call in the example uses strict
composition above Scene paint. Configure callbacks, controls, fonts and any
raster publisher before that call. Plain `plot_widgets_with_data` is an overlay
with its existing local child-revision failure policy. Strict composition stages
source, data and child construction together, and commits only after paint
publication succeeds. Invalid model/packet/placement, factory/build errors and
CPU PNG publisher rejection retain the previous accepted frame and children.
External user callback effects remain outside the transaction.

`replace_ink` explicitly selects measured source Marks to replace; an empty set
keeps their ink. It does not remove independent labels or canonical node data.
Browser `mount_with_plot_widgets_in_scene` and its matching setter use actual
Scene order, initially links then nodes; later paint can cover a Widget.
`mount_with_plot_widgets_composed` uses a top layer. Neither mode implies an
arbitrary Nivo layer-list API. See [typed packets](typed-plot-widgets.md),
[strict composition](widget-composition.md) and
[Scene-order Widgets](ordered-widgets.md) for clipping, resource limits,
retention and the host input profile.

CPU image Charts use `chart_layout(frame).raster(fonts, options)` with the same
packet; fonts and the PNG publisher are application inputs. Scene exports retain
Scene paint, not live Widgets, local state or a browser DOM. This feature does
not claim React/Spring scheduling, Canvas function equivalence, arbitrary nested
raw-data recovery, operating-system input parity or a GPU backend.

## Evidence and paint boundaries

Focused native checks cover complete source/topology conflicts before callbacks,
shared pools, zero/transparent/no-ink roles, disabled links, offset Path anchors,
measured labels, source-bit-only updates, retained node/link state, explicit
revision, strict rejection and CPU PNG publication at both DPR profiles. Legacy
Scene/SVG output is compared byte for byte. Browser cases exercise trusted click
and keyboard input, actual DOM identity/count/focus retention, source selection,
all orientations, mode/curve changes, hidden state, resize, camera, zero-only and
nonzero source updates. This guide describes scope, not a final matrix count.

The browser fixture declares source labels at 20px and Widget content at 0px to
paint solid measured controls while retaining DOM text/metadata and local state.
It does not certify Widget glyph rasterization. Foreground and declared source
interior RGBA probes allow at most one unit per channel; actual DOM bounds use
`1/32` Scene pixel. The unique source label/link-color census uses the same RGBA
tolerance, retains exact-color counts and records pixel witnesses. It proves
globally visible source paint, not every glyph contour or each link's coverage.
Whole-viewport PNG hashes remain diagnostic. Rejection witnesses separately
compare accepted model/data/focus, full DOM, computed SVG paint, Canvas backing
PNG bytes and capture geometry; no strict bit-identical viewport pass is claimed.
