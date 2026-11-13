use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use glory::render::{Event, GraphicStroke, NodeId, Props, StrokeCap};
use kartis::*;

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
            .any(|s| s == "kartis-legend-item")
            && matches!(node.descriptor.view().props, Props::Button(button) if button.label == label)
        {
            return Ok(id);
        }
        pending.extend(node.children.iter().copied());
    }
    Err(ChartError("expected legend control is absent".into()))
}

fn expected_color(group: &str, subgroup: &str) -> Color {
    if group == "A" {
        Color(160, 80, 60, 128)
    } else if subgroup == "x" {
        Color(30, 110, 150, 200)
    } else {
        Color(60, 80, 100, 160)
    }
}

fn source_color(context: &BoxPlotColorContext<'_>) -> Result<Color> {
    assert_eq!(context.computed_box.count, 6);
    assert_eq!(context.quantile_probabilities, [0.1, 0.25, 0.5, 0.75, 0.9]);
    assert_eq!(
        context.cell_index,
        context.group_index * 3 + context.subgroup_index
    );
    assert!(context.observation_indices.iter().all(|&index| {
        let raw = &context.data[index];
        raw.group == context.computed_box.group && raw.subgroup == context.computed_box.subgroup
    }));
    // A nullable source remains available without becoming a zero sample.
    if context.computed_box.group == "B" && context.computed_box.subgroup == "y" {
        assert_eq!(context.observation_indices.len(), 7);
        assert!(
            context
                .observation_indices
                .iter()
                .any(|&i| context.data[i].value.is_none())
        );
    }
    Ok(if context.computed_box.mean >= 20. {
        Color(160, 80, 60, 128)
    } else if context.raw_color_key == "x" {
        Color(30, 110, 150, 200)
    } else {
        Color(60, 80, 100, 160)
    })
}

fn caption(context: &BoxPlotLegendContext<'_>) -> Result<String> {
    assert_eq!(
        context.color,
        expected_color(&context.computed_box.group, &context.computed_box.subgroup)
    );
    // B/y and A/y return the same text; B/y is the first representative.
    Ok(context.default_text.to_string())
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
            for (i, value) in [0., 2., 4., 6., 8., 10.].into_iter().enumerate() {
                let offset = if group == "A" { 20. } else { 0. };
                data.push(BoxObservation::new(
                    format!("{group}/{subgroup}/{i}"),
                    group,
                    subgroup,
                    Some(value + offset),
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
        fill_opacity: 0.5,
        border: Some(GraphicStroke {
            color: Color(9, 9, 9, 255),
            width: 2.,
            cap: StrokeCap::Butt,
        }),
        border_color_profile: Some(InheritedColor::FromContext(vec![])),
        median_color_profile: Some(InheritedColor::FromContext(vec![ColorModifier::Darker(1.)])),
        whisker_color_profile: Some(InheritedColor::FromContext(vec![ColorModifier::Opacity(
            0.5,
        )])),
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

    // No factories preserve the ordinary Keys layout and export paths.
    let mut keys = options.clone();
    keys.legend_mode = BoxPlotLegendMode::Keys;
    let legacy = boxplot_layout(&config, &data, &keys)?;
    let unchanged =
        boxplot_layout_with_factories(&config, &data, &keys, BoxPlotFactories::default())?;
    assert_eq!(legacy, unchanged);
    assert_eq!(
        render::to_svg(&legacy.scene)?,
        render::to_svg(&unchanged.scene)?
    );
    assert_eq!(
        render::to_canvas_script(&legacy.scene, "chart")?,
        render::to_canvas_script(&unchanged.scene, "chart")?
    );

    let color_calls = Rc::new(Cell::new(0));
    let label_calls = Rc::new(Cell::new(0));
    let mut controller = {
        let colors = color_calls.clone();
        let labels = label_calls.clone();
        let mut color = move |context: &BoxPlotColorContext<'_>| {
            colors.set(colors.get() + 1);
            source_color(context)
        };
        let mut label = move |context: &BoxPlotLegendContext<'_>| {
            labels.set(labels.get() + 1);
            caption(context)
        };
        BoxPlotInteraction::with_factories(
            &config,
            &data,
            &options,
            &BoxPlotInteractionOptions::default(),
            BoxPlotFactories {
                color: Some(&mut color),
                legend_label: Some(&mut label),
            },
        )?
    }; // Both closures have ended. The controller retains resolved results.
    assert_eq!((color_calls.get(), label_calls.get()), (4, 4));
    let scene = &controller.layout.scene;
    assert_eq!(
        scene
            .legend
            .iter()
            .map(|row| row.label.as_str())
            .collect::<Vec<_>>(),
        ["Plan", "Observed"]
    );
    assert_eq!(scene.legend[0].color, Color(60, 80, 100, 160));
    let row_id = scene.legend[0].id.clone();
    assert_eq!(
        scene.legend_target(&row_id),
        Some(LegendTarget::Series(boxplot_series_id("y")))
    );
    assert_ne!(row_id, boxplot_series_id("y"));
    for summary in &controller.layout.boxes {
        let color = expected_color(&summary.group, &summary.subgroup);
        let body = scene.marks.iter().find(|m| m.id == summary.id).unwrap();
        assert_eq!(body.fill, Some(color));
        assert_eq!(body.opacity, 0.5); // Source alpha is not premultiplied.
        let border = scene
            .marks
            .iter()
            .find(|m| m.id == format!("{}:border", summary.id))
            .unwrap();
        assert_eq!(border.stroke.unwrap().color, color);
        assert!(!summary.outliers.is_empty());
        for outlier in &summary.outliers {
            let id = format!(
                "{}:outlier:{}:{}",
                summary.id,
                outlier.source_id.len(),
                outlier.source_id
            );
            assert_eq!(
                scene.marks.iter().find(|m| m.id == id).unwrap().fill,
                Some(color)
            );
        }
    }
    let first = &controller.layout.boxes[0];
    let median = scene
        .marks
        .iter()
        .find(|m| m.id == format!("{}:median", first.id))
        .unwrap();
    assert_eq!(median.stroke.unwrap().color, Color(42, 56, 70, 160));
    let whisker = scene
        .marks
        .iter()
        .find(|m| m.id == format!("{}:whiskers", first.id))
        .unwrap();
    assert_eq!(whisker.stroke.unwrap().color, Color(60, 80, 100, 128));

    let cached = controller.layout.clone();
    controller.set_active(Some(BoxPlotKey::new("A", "x")))?;
    controller.set_active(None)?;
    assert_eq!(controller.layout, cached);
    assert_eq!((color_calls.get(), label_calls.get()), (4, 4));
    controller.set_active(Some(BoxPlotKey::new("A", "x")))?;

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
    app.dispatch(legend_node(&app, "Hide Plan")?, Event::Activate)?;
    assert_eq!(
        proposals.borrow()[0],
        BTreeSet::from([boxplot_series_id("y")])
    );
    app.dispatch(legend_node(&app, "Show Plan")?, Event::Activate)?;
    assert!(proposals.borrow()[1].is_empty());
    assert_eq!((color_calls.get(), label_calls.get()), (4, 4));

    let before = controller.layout.clone();
    assert!(controller.replace_source(&config, &data, &options).is_err());
    assert_eq!(controller.layout, before);
    data.retain(|datum| datum.group != "B" || datum.subgroup != "y");
    {
        let colors = color_calls.clone();
        let labels = label_calls.clone();
        let mut color = move |context: &BoxPlotColorContext<'_>| {
            colors.set(colors.get() + 1);
            source_color(context)
        };
        let mut label = move |context: &BoxPlotLegendContext<'_>| {
            labels.set(labels.get() + 1);
            caption(context)
        };
        controller.replace_source_with_factories(
            &config,
            &data,
            &options,
            BoxPlotFactories {
                color: Some(&mut color),
                legend_label: Some(&mut label),
            },
        )?;
    }
    assert_eq!((color_calls.get(), label_calls.get()), (7, 7));
    assert_eq!(controller.active(), Some(&BoxPlotKey::new("A", "x")));
    // A/y becomes the representative; its source color travels with its row.
    assert_eq!(
        controller.layout.scene.legend[0].color,
        Color(160, 80, 60, 128)
    );
    let next = controller.whole_parts_scene_interaction()?;
    source.revise(|old| *old = next.clone());
    app.flush()?;
    legend_node(&app, "Hide Plan")?;
    Ok(())
}
