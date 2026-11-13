use glory::render::{Event, NodeId, Props};
use kartis::*;
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

fn legend_node(app: &glory::App, label: &str) -> Result<NodeId> {
    let tree = app.tree();
    let mut pending = tree.roots().to_vec();
    while let Some(id) = pending.pop() {
        let node = tree.node(id).map_err(|e| ChartError(e.to_string()))?;
        if node
            .descriptor
            .view()
            .classes
            .iter()
            .any(|class| class == "kartis-legend-item")
            && matches!(node.descriptor.view().props, Props::Button(button) if button.label == label)
        {
            return Ok(id);
        }
        pending.extend(node.children.iter().copied());
    }
    Err(ChartError("expected legend control is absent".into()))
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let config = ChartConfig {
        width: 480.,
        height: 300.,
        ..Default::default()
    };
    let mut data = Vec::new();
    for group in ["B", "A"] {
        for subgroup in ["y", "x"] {
            for (i, value) in [1., 2., 3., 4.].into_iter().enumerate() {
                data.push(BoxObservation::new(
                    format!("{group}/{subgroup}/{i}"),
                    group,
                    subgroup,
                    Some(value),
                ));
            }
        }
    }
    data.push(BoxObservation::new("B/y/null", "B", "y", None));
    let options = BoxPlotLayoutOptions {
        groups: Some(vec!["B".into(), "A".into(), "spare".into()]),
        subgroups: Some(vec!["unused".into(), "y".into(), "x".into()]),
        legend_mode: BoxPlotLegendMode::ByLabel,
        legend_labels: Some(BTreeMap::from([
            ("y".into(), "Plan".into()),
            ("x".into(), "Observed".into()),
        ])),
        value_axis: AxisOptions {
            visible: false,
            grid: false,
            ..Default::default()
        },
        group_axis: CategoryAxisOptions {
            visible: false,
            grid: false,
            ..Default::default()
        },
        ..Default::default()
    };
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let mut label = move |context: &BoxPlotLegendContext<'_>| -> Result<String> {
        count.set(count.get() + 1);
        assert_eq!(context.computed_box.count, 4);
        assert_eq!(context.quantile_probabilities, [0.1, 0.25, 0.5, 0.75, 0.9]);
        assert_eq!(context.computed_box.quantiles[2], 2.5);
        assert_eq!(
            context.cell_index,
            context.group_index * 3 + context.subgroup_index
        );
        assert!(context.observation_indices.iter().all(|&i| {
            context.data[i].group == context.computed_box.group
                && context.data[i].subgroup == context.computed_box.subgroup
        }));
        Ok(format!(
            "{} / {}",
            context.computed_box.group, context.default_text
        ))
    };
    let mut controller = BoxPlotInteraction::with_legend_label_factory(
        &config,
        &data,
        &options,
        &BoxPlotInteractionOptions::default(),
        &mut label,
    )?;
    assert_eq!(calls.get(), 4);
    assert_eq!(
        controller
            .layout
            .scene
            .legend
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        ["B / Plan", "A / Plan", "B / Observed", "A / Observed"]
    );
    let row = &controller.layout.scene.legend[0];
    let target = controller.layout.scene.legend_target(&row.id).unwrap();
    assert_eq!(target, LegendTarget::Series(boxplot_series_id("y")));
    assert_ne!(row.id, target.id()); // Display-row identity is not a hidden-series key.
    controller.set_active(Some(BoxPlotKey::new("B", "y")))?;
    assert_eq!(calls.get(), 4); // Controlled hover uses cached legend rows.

    let app = glory::App::memory();
    let source = app
        .scope()
        .cage(controller.whole_parts_scene_interaction()?);
    let proposals = Rc::new(RefCell::new(Vec::<BTreeSet<String>>::new()));
    let output = proposals.clone();
    app.mount(
        chart_layout(source)
            .highlight(false)
            .on_hidden_change(move |hidden| output.borrow_mut().push(hidden)),
    )?;
    app.flush()?;
    app.dispatch(legend_node(&app, "Hide B / Plan")?, Event::Activate)?;
    assert_eq!(
        proposals.borrow()[0],
        BTreeSet::from([boxplot_series_id("y")])
    );
    // Both differently labelled y rows share the same canonical visibility bit.
    legend_node(&app, "Show A / Plan")?;
    app.dispatch(legend_node(&app, "Show B / Plan")?, Event::Activate)?;
    assert!(proposals.borrow()[1].is_empty());
    assert_eq!(calls.get(), 4);

    let before = controller.layout.clone();
    assert!(controller.replace_source(&config, &data, &options).is_err());
    assert_eq!(controller.layout, before);
    data.retain(|datum| datum.group != "A" || datum.subgroup != "y");
    controller.replace_source_with_legend_label_factory(&config, &data, &options, &mut label)?;
    assert_eq!(calls.get(), 7);
    let next = controller.whole_parts_scene_interaction()?;
    source.revise(|old| *old = next.clone());
    app.flush()?;
    assert_eq!(controller.layout.scene.legend.len(), 3);

    #[cfg(feature = "raster")]
    {
        let raster_app = glory::App::memory();
        let raster = RasterSurface::mount_layout(
            raster_app.scope(),
            next,
            render::raster::FontRegistry::new(),
            RasterSurfaceOptions {
                highlight: false,
                ..Default::default()
            },
        )?;
        raster_app.mount(raster.widget())?;
        raster_app.flush()?;
        assert_eq!(raster.handle().frame()?.width, 480);
    }
    Ok(())
}
