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
