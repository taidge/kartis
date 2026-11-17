//! A complete Glory application with Tree's five borrowed base functions.
use glory::App;
use kartis::*;

fn source(visits: f64) -> Result<SceneInteraction> {
    let data = [
        HierarchyDatum::new("visits", Some("root".into()), visits),
        HierarchyDatum::new("root", None, 0.),
        HierarchyDatum::new("sales", Some("root".into()), 10.),
    ];
    let options = TreeOptions::default();
    // Size is a diameter; node.radius is half the accepted result.
    let mut size = |ctx: &TreeNodeFunctionContext<'_>| Ok(16. + ctx.depth as f64 * 8.);
    let mut color = |ctx: &TreeNodeFunctionContext<'_>| {
        Ok(if ctx.children.is_empty() {
            Color(30, 140, 90, 255)
        } else {
            Color(40, 100, 180, 255)
        })
    };
    let mut thickness =
        |ctx: &TreeLinkFunctionContext<'_>| Ok(if ctx.target_node.value >= 20. { 4. } else { 2. });
    let mut link_color = |ctx: &TreeLinkFunctionContext<'_>| {
        Ok(ctx.target_node.color.unwrap_or(Color(100, 100, 100, 255)))
    };
    let mut label =
        |ctx: &TreeLabelFunctionContext<'_>| Ok(format!("{}: {:.0}", ctx.datum.id, ctx.node.value));
    let layout = tree_layout_with_factories(
        &ChartConfig::default(),
        &data,
        &options,
        &mut TreeFactories {
            node_size: Some(&mut size),
            node_color: Some(&mut color),
            link_thickness: Some(&mut thickness),
            link_color: Some(&mut link_color),
            label: Some(&mut label),
        },
    )?;
    assert_eq!(layout.nodes[0].radius, 12.);
    assert_eq!(layout.nodes[1].value, visits + 10.);
    assert!(
        layout
            .scene
            .labels
            .iter()
            .any(|l| l.text == format!("visits: {visits:.0}"))
    );
    // Drawing labels do not rewrite original source or business-control labels.
    let frame = tree_widget_frame(&layout)?;
    assert_eq!(frame.interaction.as_ref().unwrap().sites[0].label, "visits");
    Ok(frame)
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let app = App::memory();
    let input = app.scope().cage(source(20.)?);
    let frame = app.scope().bond(move || input.get());
    app.mount(chart_layout(frame).on_reject(|e| eprintln!("Tree example rejected: {e:?}")))?;
    app.flush()?;
    input.revise(|old| *old = source(30.).unwrap());
    app.flush()?;
    assert!(input.with(|s| s.scene.labels.iter().any(|l| l.text == "visits: 30")));
    app.clear()?;
    Ok(())
}
