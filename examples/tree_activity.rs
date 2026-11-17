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
