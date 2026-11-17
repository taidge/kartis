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
