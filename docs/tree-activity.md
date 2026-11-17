# Tree activity and inherited colors

This development API is not included in the published 0.1.0 package.
`tree_layout_with_activity` combines the [five base functions](tree-functions.md)
with four activity modifiers and fixed-depth color inheritance. The functions
are borrowed for one build and never retained by its result.
[中文](tree-activity.zh-CN.md)

`TreeActivityOptions.focus_id` is an original source ID. `None` clears activity;
an unknown ID is rejected before any function runs. Its `highlight` uses the
four independent `TreeRelationOptions` flags for ancestor/descendant nodes and
links. The focused node itself is always active. Defaults highlight ancestors
and ancestor links, matching the original Tree current-node policy.

With no current node all entries in `TreeActivityState.nodes` and `.links` are
`None`, and none of the four modifier functions run. With a current node each
entry is `Some(true)` or `Some(false)`. Disabling all link highlighting makes
every link inactive; it does not restore the neutral state.

| `TreeActivityFactories` field | Input | Output |
| --- | --- | --- |
| `active_node_size` | `TreeNodeModifierContext` | Active node diameter |
| `inactive_node_size` | `TreeNodeModifierContext` | Inactive node diameter |
| `active_link_thickness` | `TreeLinkModifierContext` | Active link width |
| `inactive_link_thickness` | `TreeLinkModifierContext` | Inactive link width |

Only the selected modifier runs for each role. A missing modifier retains its
base size or width. Constant-returning functions express numeric modifiers.
Node context contains its complete base node, original datum/source, exact base
diameter in `size`, `is_active`, complete activity state and plot. Link context
contains its base `thickness` and `color`, both final endpoint nodes and the
same state. Diameters are finite `0..=32768`; widths are zero or finite
`0.000001..=10000`. Errors return no candidate; external function side effects
are not rolled back.

## Evaluation and inheritance

`fix_node_color_at_depth: None` means unbounded depth. `Some(depth)` uses a
nonnegative integer threshold. The root always resolves its base color; nodes
at or above the threshold do so too. A deeper node inherits its already resolved
parent color and skips its own base color function. A parent's None fill is
inherited as None, rather than replaced with a child's old palette color.
This typed threshold is distinct from JavaScript fractional/negative/NaN syntax.

After complete source/options/static-path preparation, breadth-first nodes run
base size, base or inherited color, activity classification and the selected
size modifier. Final nodes are cached before any link functions. Links run base
thickness/color and their selected modifier, even when link painting is disabled.
Eligible labels run last and see final node sizes. Base link and label contexts
expose `activity: Some(&TreeActivityState)` in this constructor; ordinary
`tree_layout_with_factories` supplies None. The five-field `TreeFactories` bundle
and existing Tree node/link/layout shapes are preserved.

The static-style projection inset is unchanged. Modified sizes change real
geometry and label spacing, without reprojecting positions. Drawing labels still
do not overwrite original source or canonical business-control labels.

## Complete Memory application

This [standalone example](../examples/tree_activity.rs) drives the real Glory
plot-pointer protocol and publishes accepted activity from its hover proposal.
Memory tests that protocol, rather than physical operating-system input.

```rust
//! A Glory Memory application driven by real plot pointer move/leave events.
use std::cell::Cell;

use glory::App;
use glory::render::{Event, NodeId, PlotPointer, PlotPointerDevice, PlotPointerPhase, Rect};
use kartis::*;

fn source(focus_id: Option<String>) -> Result<TreeActivityLayout> {
    let data = [
        HierarchyDatum::new("visits", Some("group".into()), 30.),
        HierarchyDatum::new("group", Some("root".into()), 0.),
        HierarchyDatum::new("root", None, 0.),
        HierarchyDatum::new("returns", Some("root".into()), 10.),
    ];
    let activity = TreeActivityOptions {
        focus_id,
        fix_node_color_at_depth: Some(1),
        ..Default::default()
    };
    let colors = Cell::new(0);
    let modifiers = Cell::new(0);
    let mut size = |_: &TreeNodeFunctionContext<'_>| Ok(16.);
    let mut color = |ctx: &TreeNodeFunctionContext<'_>| {
        colors.set(colors.get() + 1);
        Ok(if ctx.datum.id == "root" {
            Color(40, 100, 180, 255)
        } else {
            Color(30, 140, 90, 255)
        })
    };
    let mut width = |_: &TreeLinkFunctionContext<'_>| Ok(2.);
    let mut link_color = |ctx: &TreeLinkFunctionContext<'_>| {
        Ok(ctx.source_node.color.unwrap_or(Color(100, 100, 100, 255)))
    };
    let mut label = |ctx: &TreeLabelFunctionContext<'_>| {
        Ok(format!(
            "{}: {:?}",
            ctx.datum.id,
            ctx.activity.unwrap().nodes[ctx.source_index]
        ))
    };
    let mut active_size = |ctx: &TreeNodeModifierContext<'_>| {
        modifiers.set(modifiers.get() + 1);
        Ok(ctx.size + 8.)
    };
    let mut inactive_size = |ctx: &TreeNodeModifierContext<'_>| {
        modifiers.set(modifiers.get() + 1);
        Ok(ctx.size / 2.)
    };
    let mut active_width = |ctx: &TreeLinkModifierContext<'_>| {
        modifiers.set(modifiers.get() + 1);
        Ok(ctx.thickness + 3.)
    };
    let mut inactive_width = |ctx: &TreeLinkModifierContext<'_>| {
        modifiers.set(modifiers.get() + 1);
        Ok(ctx.thickness / 2.)
    };
    let layout = tree_layout_with_activity(
        &ChartConfig::default(),
        &data,
        &TreeOptions::default(),
        &mut TreeFactories {
            node_size: Some(&mut size),
            node_color: Some(&mut color),
            link_thickness: Some(&mut width),
            link_color: Some(&mut link_color),
            label: Some(&mut label),
        },
        &activity,
        &mut TreeActivityFactories {
            active_node_size: Some(&mut active_size),
            inactive_node_size: Some(&mut inactive_size),
            active_link_thickness: Some(&mut active_width),
            inactive_link_thickness: Some(&mut inactive_width),
        },
    )?;
    assert_eq!(colors.get(), 3); // Depth-two visits inherits its parent's color.
    assert_eq!(
        modifiers.get(),
        if activity.focus_id.is_some() { 7 } else { 0 }
    );
    Ok(layout)
}

fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut pending = tree.roots().to_vec();
    let mut result = vec![];
    while let Some(id) = pending.pop() {
        let node = tree.node(id).unwrap();
        if node.descriptor.view().classes.iter().any(|c| c == class) {
            result.push(id);
        }
        pending.extend(node.children.iter().copied());
    }
    result
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let app = App::memory();
    let accepted = app.scope().cage(source(None)?);
    let frame = app
        .scope()
        .bond(move || accepted.with(|a| tree_activity_frame(a).unwrap()));
    app.mount(chart_layout(frame).highlight(false).on_hover(move |mark| {
        let focus =
            mark.and_then(|m| accepted.with(|a| a.node_id_from_mark(&m.id).map(str::to_owned)));
        if accepted.with(|a| a.activity.focus_id != focus) {
            accepted.revise(|old| *old = source(focus).unwrap());
        }
    }))?;
    let original = nodes(&app, "kartis-datum");
    let plot = nodes(&app, "kartis-plot")[0];
    let position = accepted.with(|a| a.layout.nodes[0].position);
    let config = ChartConfig::default();
    let pointer = |phase, point: [f64; 2]| Event::PlotPointer {
        from: plot,
        pointer: PlotPointer {
            pointer_id: 1,
            device: PlotPointerDevice::Mouse,
            phase,
            x: point[0],
            y: point[1],
            viewport: Rect {
                x: 0.,
                y: 0.,
                width: config.width,
                height: config.height,
            },
        },
    };
    app.dispatch(plot, pointer(PlotPointerPhase::Move, position))?;
    app.flush()?;
    assert_eq!(
        accepted.with(|a| a.activity.focus_id.clone()).as_deref(),
        Some("visits")
    );
    assert_eq!(nodes(&app, "kartis-datum"), original);
    app.dispatch(plot, pointer(PlotPointerPhase::Leave, [0., 0.]))?;
    app.flush()?;
    assert!(accepted.with(|a| a.activity.nodes.iter().all(Option::is_none)));
    app.clear()?;
    Ok(())
}
```

## Typed Widgets and accepted state

`TreeActivityLayout` contains the final `layout` and shared `activity` sidecar.
`tree_activity_frame` and `tree_activity_widgets` preflight both before exposing
the source. Widgets receive the complete original Tree role in `.tree`, shared
activity and their own `is_active`. Original source bits remain exact.

Activity is bound to its original captured Tree constructor. Replacing the public
layout with a separately constructed valid layout is rejected, even if both have
the same number of nodes. Legal current Scene paint, Paths and labels edited
within the original constructor remain supported. Thus a source reorder cannot
silently reinterpret an old activity-index vector.

## Browser ownership

`kartis::browser::BrowserTree` owns a BrowserChart and its current-node loop.
Use `mount_with_factory` to create borrowed base/modifier functions inside its
`BrowserTreeBuildContext` callback, or `mount_with_factory_and_widgets` for
retained `TreeActivityWidgetData` children. `BrowserTreeWidgetOptions` controls
revision, ink replacement and source ordering. Factory/build/paint failure keeps
the accepted Scene, activity and retained child data together; a new revision
reconstructs children. The weak handle cannot keep a stopped owner alive.

Native listeners only reserve coalesced pointer work. A separate owner RAF
resolves it and builds a candidate; it never recursively flushes from a chart
event callback. Accepted snapshots move with successful paint and Widget-stage
adoption. Publishing-generated hover changes do not clear the current node,
so a modifier that removes its ink cannot cause a rebuild loop. Event-only or
idle frames do not rerun factories or repaint the same Scene.

`BrowserTreePicking::Geometry` uses actual painted canonical node hits; nodes
without a business binding remain available to the separate nearest policy. Default
`Nearest { distance: 1_000_000. }` independently indexes captured logical centers,
including zero-size, transparent and empty-label nodes, while filtering hidden
nodes and rejecting points outside the plot. Distance is in logical viewport
pixels and respects the accepted camera. The shared chart remains strictly
PaintedGeometry: nearest activity does not invent canonical controls, physical
hits or nearest-node tooltips. Equal-distance ties use Rust's lexical source-ID
policy, rather than claiming identical D3 tie behavior.

Real primary mouse/pen move and leave drive activity. Click and keyboard Focus
events are forwarded, but do not set the current node; application-directed
focus uses `set_activity_options`. Touch is not this activity-input protocol.
`dispatch(SurfaceInput::Move)` targets the shared chart only and does not pretend
to be a native nearest-node event. Positive axis-aligned CSS scaling is mapped
through actual bounds; arbitrary rotation/skew/perspective is outside this scope.

Source/config/options/revision changes are candidate requests. A failed request
can leave `desired_focus` different from the accepted layout; `refresh` retries
it explicitly, rather than starting an automatic failure loop. Clear current
and flush before removing its source node. Public subscriptions forward owner
events without replacing its internal chart subscription. Keep the BrowserTree
value for its lifetime, then stop or drop it to release listeners and RAF work.

[Label components](tree-label-components.md) and [native Canvas programs](tree-canvas.md)
have separate later APIs. Automatic Spring/retiring components, arbitrary SVG
function layers and live Widget export remain open. GPU/worker remain deferred. Shared SVG/Canvas scenes,
finite verified input and native contour checks do not establish arbitrary React,
D3, tooltip, OS or full-frame equivalence.

The original activity acceptance waited for hover to settle before clicking and
exposed a separate rapid move/down/up race. The subsequent
[pointer lifecycle work](tree-input-lifecycle.md) reserves native presses and
defers automatic activity through click completion. An earlier 64-case
four-engine comparison recorded old-package WebKit failures and passed with the
fix, without a pre-press hover wait or retry. Manual cleanup later removed those
raw historical artifacts. [Verification](verification.md#tree-native-pointer-lifecycle)
distinguishes that execution history from current preserved acceptance. These
new APIs are not part of published 0.1.0.
