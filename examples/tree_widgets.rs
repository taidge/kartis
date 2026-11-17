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
