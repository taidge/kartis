# Live Network link and node components

Available in Kartis 0.2.0. `network_link_widgets` attaches typed Glory components to actual
links in `NetworkLinkLayout`. `network_widgets` combines link and node roles in
one packet, one 128-slot budget and authored Scene order: links, then nodes.
`network_link_widget_frame` clones the exact authored Scene and existing node
binding with PaintedGeometry. Links remain decorative source marks, without
invented node controls, canonical link sites or force-drag targets.

## Current data and identity

`NetworkLinkWidgetData.source` is one shared complete current force-node snapshot;
`edges` is the complete current force-edge snapshot. `source_index` indexes edges;
its `source`/`target` indices address nodes. The data also contains checked
`NetworkLinkMetadata`, world offset, ticks and alpha. Original values retain float
bits, including signed zero and unselected siblings. This is current force state,
not arbitrary application RawDatum fields or every simulation option. The host
frame's Mark retains actual geometry, stroke/color/thickness, opacity and value.

The borrowed placement context supplies actual source/target states, source index,
Mark and original endpoint positions. Full layout/node/edge validation precedes
the first callback. `AtDatum` resolves to an explicit rectangle at the original
endpoint midpoint, even for a self-link, transparent ink or an offset Path whose
bounds center differs. Return None to leave a role unmounted. Custom Rect geometry
must be centered at its origin; use Path for offset geometry.

`NetworkWidgetData` and `NetworkWidgetContext` have Node/Link variants. Selected
roles share one owned node snapshot; selected links additionally share one edge
snapshot. Node-only APIs keep their original `NetworkNodeWidgetData` payload.

Link mark identity is `force-link:index`, preserving separate parallel edges and
self-links. Reordering edges retains index-local state, even if a different edge
occupies that index. Request a new revision to rebuild children on business-edge
replacement/reorder; this discards local state, rather than inferring a business
identity. Nivo's `source.id.target.id` key can collide for parallel edges and is
not copied as Kartis's identity rule.

## A complete application

This application owns its tick loop, displays both roles and verifies that their
actual Buttons and click counts survive a real simulation step:

```rust
//! Application-owned force ticks with retained node and link components.
use glory::render::{Event, EventKind, NodeId, Props};
use glory::{App, Widget};
use kartis::*;
#[derive(Clone, Debug)]
struct Source {
    frame: SceneInteraction,
    composition: PlotWidgetsComposition<NetworkWidgetData>,
}
fn source(sim: &ForceSimulation) -> Result<Source> {
    let layout = network_glyph_layout_with_link_factory(
        &ChartConfig::default(),
        sim,
        &NetworkGlyphOptions::default(),
        &mut |ctx| Ok(ctx.inherit(ctx.geometry.clone())),
    )?;
    let widgets = network_widgets(&layout, sim, 0, |ctx| {
        let selected = match ctx {
            NetworkWidgetContext::Link(_) => true,
            NetworkWidgetContext::Node(node) => node.node.id == "a",
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
        frame: network_link_widget_frame(&layout)?,
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
            .any(|c| c == "network-mixed-child")
        {
            result.push(id);
        }
        pending.extend(node.children.iter().copied());
    }
    result
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
                .class("network-mixed-child")
                .label(scope.bond(move || {
                    frame.with(|f| {
                        let (role, ticks, position) = match &f.data {
                            NetworkWidgetData::Link(link) => {
                                ("link", link.ticks, link.site.position)
                            }
                            NetworkWidgetData::Node(node) => {
                                ("node", node.ticks, node.site.position)
                            }
                        };
                        format!(
                            "{role}: tick={ticks}, x={:.3}, clicks={}",
                            position[0],
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
    simulation.step(1)?;
    let next = source(&simulation)?;
    input.revise(|old| *old = next);
    app.flush()?;
    assert_eq!(children(&app), before);
    for id in before {
        let tree = app.tree();
        let Props::Button(button) = tree.node(id).unwrap().descriptor.view().props else {
            panic!("not a Button")
        };
        assert!(button.label.contains("tick=1"));
        assert!(button.label.contains("clicks=1"));
    }
    app.clear()?;
    Ok(())
}
```

Run `cargo run --example network_link_widgets`.

## BrowserNetwork integration

Use `BrowserNetwork::mount_with_widgets` for original link geometry, or
`mount_with_link_factory_and_widgets` to combine exportable custom link ink with
live components. Both take `BrowserNetworkWidgetOptions` (`revision: 0`,
`replace_ink: true`, `source_ordered: true` by default), a retained borrowed
`NetworkWidgetContext` placement and a `PlotWidgetCompositionContext<NetworkWidgetData>`
factory. Return None for unselected roles, including all nodes for a link-only
composition. `set_widget_revision` requests a shared rebuild of both roles.

Force ticks, real source-node dragging, logical CSS resize, graph replacement,
heat and revision commit with their matching Scene and both component roles.
Placement, factory/build and publication failure preserve the accepted frame.
Adopted retained Bond reads observe the matching owner snapshot. Stop in the
first placement prevents later role callbacks; external callback effects cannot
be rolled back. Pure positive axis-aligned CSS scale affects client mapping,
preserving logical world units. External CSS itself cannot be rolled back.

Source ordering is observable: a later unreplaced link or node can cover an earlier
link component and block native activation there. `source_ordered: false` uses
the strict top component layer instead. `replace_ink: false` keeps authored source
ink. Widgets own native activation/focus; they do not synthesize source selection
or dragging. Hiding a node does not automatically hide independent decorative
link components. Host visibility still follows mark opacity, clipping and series
visibility; zero-sized or transparent geometry can carry a measured component.

The browser fixture explicitly authors solid offset Path ink, a centered red
rectangle for an unselected parallel link and a 14px square B node. Its independent
paint oracle applies complete source order and actual selected replacements,
checking source coverage, exposed component pixels and trusted activation at
covered/exposed points. This is a bounded solid-color profile, not arbitrary
compositor/glyph equivalence. Whole-viewport PNG hashes remain diagnostic.

The frozen Nivo0.99 reference checks actual component creation props and transition
recipes, default SVG line properties and Canvas commands. SVG receives link,
animated endpoints/color/opacity and blendMode, without node pointer handlers;
no leave transition is configured. Canvas's separate function does not set cap,
alpha or blend. Kartis does not claim React/Spring scheduling, exact D3 force
trajectories, runtime blend equivalence, arbitrary RawDatum/Widget export or
native OS input. Automatic retiring components and GPU/worker remain open.

See [node components](network-widgets.md), [exportable link geometry](network-link-custom.md)
and [中文指南](network-link-widgets.zh-CN.md).
