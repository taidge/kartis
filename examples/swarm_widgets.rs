//! Retained Glory node components with actual packed Swarm source data.
use glory::render::{Event, EventKind, NodeId, Props};
use glory::{App, Widget};
use kartis::*;
#[derive(Clone, Debug)]
struct Source {
    frame: SceneInteraction,
    composition: PlotWidgetsComposition<SwarmNodeWidgetData>,
}
fn source(positive_zero: bool, reverse_input: bool) -> Result<Source> {
    let mut b = SwarmPoint::new("b", "north", if positive_zero { 0. } else { -0. });
    b.size = Some(0.);
    let mut data = vec![SwarmPoint::new("a", "north", 2.), b];
    data[0].label = Some("Alpha".into());
    if reverse_input {
        data.reverse();
    }
    let layout = swarm_layout(
        &ChartConfig {
            width: 600.,
            height: 260.,
            ..Default::default()
        },
        &data,
        &SwarmLayoutOptions {
            orientation: SwarmOrientation::Horizontal,
            value_domain: Some([-1., 10.]),
            value_axis: None,
            group_axis: None,
            ..Default::default()
        },
    )?;
    let widgets = swarm_node_widgets(&layout, &data, 0, |_| {
        Ok(Some(PlotWidgetPlacement::AtDatum {
            offset: [-16., -11.],
            measured: TextSize {
                width: 32.,
                height: 22.,
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
        frame: swarm_widget_frame(&layout)?,
        composition: PlotWidgetsComposition {
            widgets,
            replace_ink,
        },
    })
}
fn main_node(app: &App) -> NodeId {
    let tree = app.tree();
    let mut pending = tree.roots().to_vec();
    while let Some(id) = pending.pop() {
        let n = tree.node(id).unwrap();
        if n.descriptor
            .view()
            .classes
            .iter()
            .any(|c| c == "swarm-example-main")
        {
            return id;
        }
        pending.extend(n.children.iter().copied());
    }
    panic!("main node component missing")
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let app = App::memory();
    let input = app.scope().cage(source(false, false)?);
    let frame = app.scope().bond(move || input.with(|s| s.frame.clone()));
    let packet = app
        .scope()
        .bond(move || input.with(|s| s.composition.clone()));
    app.mount(
        chart_layout(frame).plot_widgets_composed(packet, |context| {
            let frame = context.frame;
            let main = frame.with(|f| f.data.site.node_id == "a");
            Ok(glory::ui::widget(move |scope| {
                let count = scope.cage(0u32);
                glory::ui::button()
                    .class(if main {
                        "swarm-example-main"
                    } else {
                        "swarm-example-node"
                    })
                    .label(scope.bond(move || {
                        frame.with(|f| {
                            let d = &f.data.source[f.data.source_index];
                            let zero = f.data.source.iter().find(|v| v.id == "b").unwrap();
                            format!(
                                "{} · {}: {} (zero={:016x}, clicks={})",
                                f.data.group.label,
                                d.label.as_deref().unwrap_or(&d.id),
                                f.data.site.formatted_value,
                                zero.value.to_bits(),
                                count.get()
                            )
                        })
                    }))
                    .on(EventKind::Activate, move |_| count.revise(|v| *v += 1))
                    .show_in(scope)
            }))
        }),
    )?;
    let child = main_node(&app);
    app.dispatch(child, Event::Activate)?;
    app.flush()?;
    input.revise(|s| *s = source(true, false).unwrap());
    app.flush()?;
    assert_eq!(main_node(&app), child);
    {
        let tree = app.tree();
        let Props::Button(button) = tree.node(child).unwrap().descriptor.view().props else {
            panic!("not a Button")
        };
        assert!(button.label.contains("zero=0000000000000000, clicks=1"));
    }
    input.revise(|s| *s = source(false, true).unwrap());
    app.flush()?;
    assert_eq!(main_node(&app), child);
    app.clear()?;
    Ok(())
}
