use kartis::*;
use std::{cell::Cell, collections::BTreeMap};

fn data() -> Vec<BoxObservation> {
    ["A", "B"]
        .into_iter()
        .flat_map(|g| {
            ["x", "y"].into_iter().flat_map(move |s| {
                [0., 2., 10.]
                    .into_iter()
                    .enumerate()
                    .map(move |(i, v)| BoxObservation::new(format!("{g}/{s}/{i}"), g, s, Some(v)))
            })
        })
        .collect()
}
fn options() -> BoxPlotLayoutOptions {
    BoxPlotLayoutOptions {
        legend_mode: BoxPlotLegendMode::ByLabel,
        inner_padding: 0.,
        ..Default::default()
    }
}
fn factory(ctx: &BoxPlotLegendContext<'_>) -> Result<String> {
    Ok(format!(
        "{}:{}",
        ctx.computed_box.group, ctx.computed_box.subgroup
    ))
}

#[test]
fn hover_reuses_cached_rows_targets_and_source_without_factory_reinvocation() {
    let c = ChartConfig::default();
    let d = data();
    let o = options();
    let calls = Cell::new(0);
    let mut callback = |ctx: &BoxPlotLegendContext<'_>| {
        calls.set(calls.get() + 1);
        factory(ctx)
    };
    let mut controlled = BoxPlotInteraction::with_legend_label_factory(
        &c,
        &d,
        &o,
        &Default::default(),
        &mut callback,
    )
    .unwrap();
    assert_eq!(calls.get(), 4);
    let accepted = controlled.layout.clone();
    let rows = accepted.scene.legend.clone();
    let targets = accepted.scene.legend_targets.clone();
    controlled
        .set_active(Some(BoxPlotKey::new("A", "x")))
        .unwrap();
    assert_eq!(controlled.layout.scene.legend, rows);
    assert_eq!(controlled.layout.scene.legend_targets, targets);
    controlled.whole_parts_scene_interaction().unwrap();
    controlled.set_active(None).unwrap();
    assert_eq!(controlled.layout, accepted);
    assert_eq!(calls.get(), 4);
    let before = controlled.layout.clone();
    assert!(controlled.replace_source(&c, &d, &o).is_err());
    assert_eq!(controlled.layout, before);
    assert_eq!(calls.get(), 4);
}

#[test]
fn replacement_recaches_text_keeps_live_active_key_and_clears_removed_source_atomically() {
    let c = ChartConfig::default();
    let d = data();
    let o = options();
    let mut controlled = BoxPlotInteraction::with_legend_label_factory(
        &c,
        &d,
        &o,
        &Default::default(),
        &mut factory,
    )
    .unwrap();
    controlled
        .set_active(Some(BoxPlotKey::new("A", "x")))
        .unwrap();
    let before_geometry = controlled.layout.scene.marks.clone();
    let before_targets = controlled.layout.scene.legend_targets.clone();
    let calls = Cell::new(0);
    controlled
        .replace_source_with_legend_label_factory(&c, &d, &o, &mut |ctx| {
            calls.set(calls.get() + 1);
            Ok(format!(
                "new {}:{}",
                ctx.computed_box.group, ctx.computed_box.subgroup
            ))
        })
        .unwrap();
    assert_eq!(calls.get(), 4);
    assert_eq!(controlled.active(), Some(&BoxPlotKey::new("A", "x")));
    assert_eq!(controlled.layout.scene.marks, before_geometry);
    assert_eq!(controlled.layout.scene.legend_targets, before_targets);
    assert!(
        controlled
            .layout
            .scene
            .legend
            .iter()
            .all(|r| r.label.starts_with("new "))
    );
    let remaining: Vec<_> = d.into_iter().filter(|d| d.group == "B").collect();
    controlled
        .replace_source_with_legend_label_factory(&c, &remaining, &o, &mut factory)
        .unwrap();
    assert_eq!(controlled.active(), None);
    assert_eq!(controlled.layout.boxes.len(), 2);
}

#[test]
fn callback_errors_and_tampered_accepted_targets_preserve_whole_frame_and_raw_binding() {
    let c = ChartConfig::default();
    let d = data();
    let o = options();
    let mut controlled = BoxPlotInteraction::with_legend_label_factory(
        &c,
        &d,
        &o,
        &Default::default(),
        &mut factory,
    )
    .unwrap();
    controlled
        .set_active(Some(BoxPlotKey::new("A", "x")))
        .unwrap();
    let before = controlled.layout.clone();
    let binding = controlled.whole_parts_interaction_binding().unwrap();
    let calls = Cell::new(0);
    assert!(
        controlled
            .replace_source_with_legend_label_factory(&c, &d, &o, &mut |_| {
                calls.set(calls.get() + 1);
                Err(ChartError("callback refused".into()))
            })
            .is_err()
    );
    assert_eq!(calls.get(), 1);
    assert_eq!(controlled.layout, before);
    assert_eq!(
        controlled.whole_parts_interaction_binding().unwrap(),
        binding
    );
    assert_eq!(controlled.active(), Some(&BoxPlotKey::new("A", "x")));
    let row = controlled.layout.scene.legend[0].id.clone();
    controlled
        .layout
        .scene
        .legend_targets
        .insert(row, LegendTarget::Series(boxplot_series_id("y")));
    let tampered = controlled.layout.clone();
    calls.set(0);
    assert!(
        controlled
            .replace_source_with_legend_label_factory(&c, &d, &o, &mut |_| {
                calls.set(calls.get() + 1);
                Ok("ignored".into())
            })
            .is_err()
    );
    assert_eq!(calls.get(), 0);
    assert!(controlled.whole_parts_interaction_binding().is_err());
    assert!(controlled.set_active(None).is_err());
    assert_eq!(controlled.layout, tampered);
    controlled.layout = before;
    controlled.set_active(None).unwrap();
}

#[test]
fn factory_construction_policy_does_not_guess_from_empty_legend_or_targets() {
    let c = ChartConfig::default();
    let mut o = options();
    o.legend = false;
    let d = data();
    let mut controlled =
        BoxPlotInteraction::with_legend_label_factory(&c, &d, &o, &Default::default(), &mut |_| {
            panic!("legend=false callback")
        })
        .unwrap();
    assert!(controlled.layout.scene.legend_targets.is_empty());
    assert!(controlled.replace_source(&c, &d, &o).is_err());
    assert!(
        controlled
            .replace_source_with_scale_options(&c, &d, &o, &Default::default())
            .is_err()
    );
    controlled
        .replace_source_with_legend_label_factory(&c, &d, &o, &mut |_| {
            panic!("legend=false replacement callback")
        })
        .unwrap();
    let mut empty = BoxPlotInteraction::with_legend_label_factory(
        &c,
        &[],
        &options(),
        &Default::default(),
        &mut |_| panic!("no actual summary"),
    )
    .unwrap();
    assert!(empty.layout.scene.legend.is_empty());
    assert!(empty.replace_source(&c, &d, &options()).is_err());
    // A regular ByLabel constructor has no retained factory requirement.
    let mut regular = BoxPlotInteraction::new(&c, &d, &o, &Default::default()).unwrap();
    regular.replace_source(&c, &d, &o).unwrap();
    assert!(!format!("{regular:?}").contains("legend_factory_built"));
}

#[test]
fn scaled_factory_replacement_preserves_scale_and_can_explicitly_change_it() {
    let c = ChartConfig::default();
    let d = data();
    let o = options();
    let scale = ContinuousScaleOptions {
        reverse: true,
        ..Default::default()
    };
    let mut controlled = BoxPlotInteraction::with_scale_options_and_legend_label_factory(
        &c,
        &d,
        &o,
        &scale,
        &Default::default(),
        &mut factory,
    )
    .unwrap();
    let resolved = format!("{:?}", controlled.resolved_value_scale());
    controlled
        .set_active(Some(BoxPlotKey::new("A", "x")))
        .unwrap();
    controlled
        .replace_source_with_legend_label_factory(&c, &d, &o, &mut factory)
        .unwrap();
    assert_eq!(format!("{:?}", controlled.resolved_value_scale()), resolved);
    let other = ContinuousScaleOptions::default();
    controlled
        .replace_source_with_scale_options_and_legend_label_factory(
            &c,
            &d,
            &o,
            &other,
            &mut factory,
        )
        .unwrap();
    assert_ne!(format!("{:?}", controlled.resolved_value_scale()), resolved);
    assert_eq!(controlled.active(), Some(&BoxPlotKey::new("A", "x")));
}

#[test]
fn by_label_without_factory_keeps_regular_map_replacement_semantics() {
    let c = ChartConfig::default();
    let d = data();
    let mut o = options();
    let mut controlled = BoxPlotInteraction::new(&c, &d, &o, &Default::default()).unwrap();
    controlled
        .set_active(Some(BoxPlotKey::new("A", "x")))
        .unwrap();
    let old = controlled.layout.scene.marks.clone();
    o.legend_labels = Some(BTreeMap::from([
        ("x".into(), "Observed".into()),
        ("y".into(), "Plan".into()),
    ]));
    controlled.replace_source(&c, &d, &o).unwrap();
    assert_eq!(controlled.layout.scene.marks, old);
    assert_eq!(controlled.active(), Some(&BoxPlotKey::new("A", "x")));
    assert_eq!(
        controlled
            .layout
            .scene
            .legend
            .iter()
            .map(|r| r.label.as_str())
            .collect::<Vec<_>>(),
        ["Observed", "Plan"]
    );
}
