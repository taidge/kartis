# Live force-network node Widgets

Available in Kartis 0.2.0. `network_node_widgets` attaches measured, typed Glory components to visual nodes from `NetworkGlyphLayout`. `network_widget_frame` clones the authored Scene with explicit PaintedGeometry and its existing canonical node binding. `BrowserNetwork::mount_with_node_widgets` additionally updates real components across force ticks, source-node dragging, responsive resize and graph replacement. `mount_with_link_factory_and_node_widgets` combines them with retained exportable link geometry.

## Node data and placement

`NetworkNodeWidgetData` contains a shared `Arc<Vec<ForceNodeState>>`, source index, current `NetworkGlyphSite`, world-to-Scene offset, total ticks and heat. It is a complete current node-state snapshot; it does not include arbitrary caller RawDatum properties or every simulation/edge option. Read `source[source_index]` for label, physical radius, world position, velocity and fixed position. Original state float bits participate in equality, including hidden/unselected siblings. Geometry/paint comes from the accepted host Mark. The node Mark's compatibility value stays zero; the physical radius is metadata, independently of visual glyph size/bounds.

The borrowed context supplies the private checked layout, current simulation/node, source index, site, actual Mark and offset. Whole-frame/source validation precedes the first callback. Return `None` to skip a role; a failure returns no packet. External callback effects remain application-owned. Selected nodes share one owned input copy, and at most 128 slots are admitted.

All actual nodes can have roles, including empty-label, transparent and zero visual-size nodes. This does not invent labels, canonical source controls or picking sites. Existing node binding eligibility stays unchanged. The adapter resolves `AtDatum` to an explicit rectangle from the saved exact glyph origin, including unlabeled Paths whose bounds center differs from that origin. Rebuild a packet for each accepted simulation snapshot; stable revision retains local child state. Scene export contains authored marks/labels, not live Widgets.

## A complete application

This standalone Memory example owns its tick loop and retains the same actual Button and click count after a real simulation step:

```rust
//! Explicit application-owned force ticks and retained Glory node components.
use glory::render::{Event, EventKind, NodeId, Props};
use glory::{App, Widget};
use kartis::*;
#[derive(Clone, Debug)]
struct Source {
    frame: SceneInteraction,
    composition: PlotWidgetsComposition<NetworkNodeWidgetData>,
}
fn source(sim: &ForceSimulation) -> Result<Source> {
    let layout = sim.scene_with_glyphs(&ChartConfig::default(), &NetworkGlyphOptions::default())?;
    let widgets = network_node_widgets(&layout, sim, 0, |ctx| {
        Ok(
            (ctx.node.id == "a").then_some(PlotWidgetPlacement::AtDatum {
                offset: [-40., -12.],
                measured: TextSize {
                    width: 80.,
                    height: 24.,
                },
            }),
        )
    })?;
    let replace_ink = widgets
        .layout
        .slots
        .iter()
        .map(|s| s.mark_id.clone())
        .collect();
    Ok(Source {
        frame: network_widget_frame(&layout)?,
        composition: PlotWidgetsComposition {
            widgets,
            replace_ink,
        },
    })
}
fn child(app: &App) -> NodeId {
    let tree = app.tree();
    let mut pending = tree.roots().to_vec();
    while let Some(id) = pending.pop() {
        let n = tree.node(id).unwrap();
        if n.descriptor
            .view()
            .classes
            .iter()
            .any(|c| c == "network-example-child")
        {
            return id;
        }
        pending.extend(n.children.iter().copied());
    }
    panic!("node component missing")
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut simulation = ForceSimulation::new(
        &[NetworkNode::new("a"), NetworkNode::new("b")],
        &[NetworkLink::new("a", "b", 2.)],
        ForceOptions {
            labels: false,
            ..Default::default()
        },
    )?;
    let app = App::memory();
    let input = app.scope().cage(source(&simulation)?);
    let frame = app.scope().bond(move || input.with(|s| s.frame.clone()));
    let packet = app
        .scope()
        .bond(move || input.with(|s| s.composition.clone()));
    app.mount(chart_layout(frame).plot_widgets_composed(packet, |ctx| {
        let frame = ctx.frame;
        Ok(glory::ui::widget(move |scope| {
            let count = scope.cage(0u32);
            glory::ui::button()
                .class("network-example-child")
                .label(scope.bond(move || {
                    frame.with(|f| {
                        let node = &f.data.source[f.data.source_index];
                        format!(
                            "{}: tick={}, x={:.3}, clicks={}",
                            node.label,
                            f.data.ticks,
                            node.position[0],
                            count.get()
                        )
                    })
                }))
                .on(EventKind::Activate, move |_| count.revise(|n| *n += 1))
                .show_in(scope)
        }))
    }))?;
    let node = child(&app);
    app.dispatch(node, Event::Activate)?;
    app.flush()?;
    simulation.step(1)?;
    let next = source(&simulation)?;
    input.revise(|old| *old = next);
    app.flush()?;
    assert_eq!(child(&app), node);
    {
        let tree = app.tree();
        let Props::Button(button) = tree.node(node).unwrap().descriptor.view().props else {
            panic!("not a Button")
        };
        assert!(button.label.contains("tick=1"));
        assert!(button.label.contains("clicks=1"));
    }
    app.clear()?;
    Ok(())
}
```

Run `cargo run --example network_widgets`.

## BrowserNetwork integration

`BrowserNetworkNodeWidgetOptions` has `revision`, `replace_ink` and `source_ordered`. Defaults are `0`, `true`, `true`: replace selected node ink at its Scene position. `source_ordered: false` uses the strict top Widget layer. `replace_ink: false` keeps authored node ink. Both modes use strict candidate construction; geometry, input metadata and child build failure reject the whole candidate.

Use the retained borrowed placement closure and the usual `PlotWidgetCompositionContext<NetworkNodeWidgetData>` factory. Change `set_node_widget_revision` to rebuild children; ordinary ticks, dragging, resize and source replacement update existing data. Pending rebuild/heat requests are accepted with their matching force/Scene/Widget frame. `reheat(alpha)` raises heat with `max(current, alpha)`; it cannot cool a simulation, and consecutive queued calls preserve their maximum. The latest pending heat is applied to the latest queued graph candidate.

The driver computes layout dimensions from untransformed CSS content dimensions and uses actual positive axis-aligned client bounds for pointer mapping. Pure CSS scaling changes screen mapping, preserving logical world units. A real content-size change rebuilds node positions and slots together. External CSS width itself cannot be rolled back: after a failed resize, the preceding logical Scene, node data and responsive Widget frame remain mutually consistent until retry.

Widgets own their native activation/focus. They do not synthesize source selection or force dragging. Unreplaced visible nodes retain ordinary contour-based source dragging; replaced ink is excluded before picking. Hiding node series hides those nodes/Widgets but does not automatically remove decorative link ink. Links remain a separate geometry/component role.

Placement/factory construction sees the preceding accepted owner state and a tentative build context. Retained live Bond reads see the matching owner force snapshot once the shared Widget stage is adopted; prepaint failures do not expose the candidate. State reads and stop are supported from callbacks. A stop during placement prevents subsequent user placements. Nested graph replacement during layout is rejected. Stop releases RAF, listeners, captures and active child scopes. Callback side effects cannot be rolled back.

Nivo0.99 `nodeComponent` receives its computed node, animated style and four handlers; default SVG radius is visual size/2, with translate/scale transitions. Its Canvas renderer is separate. Rust preserves typed finite force state, String identity, explicit host input and independent physical/visual sizes. The original component/style/handler recipes are verified separately; this does not claim arbitrary raw fields, React/Spring scheduling or identical D3 force trajectories.

The browser fixture authors a 6px decorative link for a solid interior sample; the preceding 2px profile retains its failed color sample and is not reported as repaired. Drag-failure injection and the accepted-state baseline are read in one synchronous boundary so a legal RAF tick cannot intervene. Drag expectations use trusted delivered pointerdown/pointerup coordinates, the measured stable client bounds and preserved grab offset; requested automation coordinates are recorded separately. A one-client-pixel input-delivery qualification does not replace the separate 1/32 world/DOM geometry gate. An idle source hover is also checked for unchanged ticks, revision and no pending RAF.

The browser fixture uses font-zero solid Buttons, actual DOM geometry, independent initial force inputs, exact accepted-state/DOM/SVG-style/Canvas-backing rollback and original float-bit checks. Whole-viewport PNG hashes are diagnostic. Arbitrary glyph equivalence, historical strict compositor failures, live Widget export, native same-layer input, automatic retiring-node animations and GPU/worker remain open.

See [visual network geometry](../src/network_glyphs.rs), [link factories](network-link-custom.md), [typed packets](typed-plot-widgets.md) and [中文指南](network-widgets.zh-CN.md).
