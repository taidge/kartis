use std::cell::RefCell;

use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 320.,
        height: 180.,
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 20.,
            bottom: 20.,
        },
        ..Default::default()
    }
}
fn data() -> Vec<HeatCell> {
    vec![
        HeatCell {
            row: "r".into(),
            column: "a".into(),
            value: None,
        },
        HeatCell::new("r", "b", 0.),
        HeatCell::new("r", "c", 10.),
    ]
}
fn options() -> HeatmapLayoutOptions {
    HeatmapLayoutOptions {
        axes: HeatmapAxes::none(),
        ..Default::default()
    }
}
fn with_text(o: &HeatmapLayoutOptions) -> HeatmapLayout {
    let mut label = |ctx: &HeatmapLabelContext<'_>| Ok(format!("custom:{}", ctx.default_text));
    heatmap_layout_with_factories(
        &config(),
        &data(),
        o,
        HeatmapFactories {
            cell: None,
            label: Some(&mut label),
        },
    )
    .unwrap()
}

#[test]
fn typed_callbacks_follow_configured_order_and_original_indices_not_dense_pairs() {
    let cells = vec![
        HeatCell::new("b", "z", 3.),
        HeatCell {
            row: "a".into(),
            column: "x".into(),
            value: None,
        },
        HeatCell::new("a", "z", 0.),
        HeatCell::new("hidden", "x", 9.),
        HeatCell::new("b", "hidden", 5.),
    ];
    let o = HeatmapLayoutOptions {
        row_order: Some(vec!["b".into(), "a".into(), "hidden".into()]),
        column_order: Some(vec!["z".into(), "x".into(), "hidden".into()]),
        hidden_rows: ["hidden".into()].into(),
        hidden_columns: ["hidden".into()].into(),
        label_offset: [2., 3.],
        ..options()
    };
    let mut calls = vec![];
    let mut label = |ctx: &HeatmapLabelContext<'_>| {
        assert!(std::ptr::eq(ctx.datum, &cells[ctx.source_index]));
        assert_eq!(
            ctx.key,
            &HeatmapCellKey::new(&ctx.datum.row, &ctx.datum.column)
        );
        assert_eq!(ctx.mark_id, ctx.key.mark_id());
        assert_eq!(
            ctx.position,
            [
                ctx.band.x + ctx.band.width / 2.,
                ctx.band.y + ctx.band.height / 2.
            ]
        );
        assert_eq!(
            ctx.label_position,
            [ctx.position[0] + 2., ctx.position[1] + 7.]
        );
        calls.push((
            ctx.source_index,
            ctx.datum.value,
            ctx.formatted_value.map(str::to_owned),
            ctx.default_text.to_owned(),
        ));
        Ok(format!("source-{}", ctx.source_index))
    };
    let layout = heatmap_layout_with_factories(
        &config(),
        &cells,
        &o,
        HeatmapFactories {
            cell: None,
            label: Some(&mut label),
        },
    )
    .unwrap();
    assert_eq!(
        calls,
        vec![
            (0, Some(3.), Some("3".into()), "3".into()),
            (2, Some(0.), Some("0".into()), "0".into()),
            (1, None, None, "missing".into())
        ]
    );
    assert_eq!(
        layout
            .scene
            .labels
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>(),
        ["source-0", "source-2", "source-1"]
    );
    assert_eq!(layout.interaction_binding().unwrap().sites.len(), 3);
    assert_eq!(layout.cells.len(), 5);
}

#[test]
fn empty_default_can_be_rewritten_and_empty_return_keeps_owned_slot() {
    let o = HeatmapLayoutOptions {
        empty_label: String::new(),
        ..options()
    };
    let mut calls = 0;
    let mut label = |ctx: &HeatmapLabelContext<'_>| {
        calls += 1;
        if ctx.datum.value.is_none() {
            assert_eq!(ctx.default_text, "");
            Ok("NULL".into())
        } else {
            Ok(String::new())
        }
    };
    let mut layout = heatmap_layout_with_factories(
        &config(),
        &data(),
        &o,
        HeatmapFactories {
            cell: None,
            label: Some(&mut label),
        },
    )
    .unwrap();
    assert_eq!(calls, 3);
    assert_eq!(layout.scene.labels.len(), 3);
    assert_eq!(
        layout
            .scene
            .labels
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>(),
        ["NULL", "", ""]
    );
    assert_eq!(layout.scene.label_series.len(), 3);
    layout
        .set_active(Some(HeatmapCellKey::new("r", "b")))
        .unwrap();
    layout.set_active(None).unwrap();
    layout.interaction_binding().unwrap();
}

#[test]
fn disabled_labels_skip_only_text_factory_and_all_hidden_skip_both() {
    for hidden in [false, true] {
        let mut o = HeatmapLayoutOptions {
            labels: false,
            ..options()
        };
        if hidden {
            o.hidden_rows.insert("r".into());
        }
        let mut text_calls = 0;
        let mut cell_calls = 0;
        let mut label = |_: &HeatmapLabelContext<'_>| {
            text_calls += 1;
            Ok("unused".into())
        };
        let mut cell = |ctx: &HeatmapCellContext<'_>| {
            cell_calls += 1;
            Ok(ctx.inherit(ctx.geometry.clone()))
        };
        let layout = heatmap_layout_with_factories(
            &config(),
            &data(),
            &o,
            HeatmapFactories {
                cell: Some(&mut cell),
                label: Some(&mut label),
            },
        )
        .unwrap();
        assert_eq!(text_calls, 0);
        assert_eq!(cell_calls, if hidden { 0 } else { 3 });
        assert!(layout.scene.labels.is_empty());
    }
}

#[test]
fn factories_compose_explicit_scale_label_roles_and_hover_without_source_pollution() {
    let o = HeatmapLayoutOptions {
        opacity: 0.8,
        active_opacity: 0.9,
        inactive_opacity: 0.2,
        hover_target: HeatmapHoverTarget::Cell,
        colors: ColorScaleOptions {
            domain: Some([0., 10.]),
            colors: vec![Color(20, 40, 60, 160), Color(200, 180, 160, 220)],
            unknown: Color(90, 100, 110, 64),
            ..Default::default()
        },
        label_color_profile: Some(InheritedColor::FromContext(vec![ColorModifier::Darker(1.)])),
        ..options()
    };
    let default =
        heatmap_layout_with_color_profile(&config(), &data(), &o, ColorInterpolation::Lab).unwrap();
    let order = RefCell::new(vec![]);
    let mut label = |ctx: &HeatmapLabelContext<'_>| {
        order.borrow_mut().push((ctx.source_index, 'l'));
        Ok(format!("{}:{}", ctx.datum.column, ctx.default_text))
    };
    let mut cell = |ctx: &HeatmapCellContext<'_>| {
        order.borrow_mut().push((ctx.source_index, 'c'));
        let mut node = ctx.inherit(ctx.geometry.clone());
        node.fill = Some(Color(240, 30, 180, 255));
        node.opacity = 0.5;
        Ok(node)
    };
    let mut layout = heatmap_layout_with_color_profile_and_factories(
        &config(),
        &data(),
        &o,
        ColorInterpolation::Lab,
        HeatmapFactories {
            cell: Some(&mut cell),
            label: Some(&mut label),
        },
    )
    .unwrap();
    assert_eq!(
        *order.borrow(),
        [(0, 'l'), (0, 'c'), (1, 'l'), (1, 'c'), (2, 'l'), (2, 'c')]
    );
    assert_eq!(
        layout
            .scene
            .labels
            .iter()
            .map(|l| l.color)
            .collect::<Vec<_>>(),
        [
            Color(63, 70, 77, 26),
            Color(14, 28, 42, 64),
            Color(140, 126, 112, 88)
        ]
    );
    for (actual, original) in layout.cells.iter().zip(&default.cells) {
        assert_eq!(actual.key, original.key);
        assert_eq!(actual.mark_id, original.mark_id);
        assert_eq!(actual.value, original.value);
        assert_eq!(actual.formatted_value, original.formatted_value);
        assert_eq!(actual.label, original.label);
        assert_eq!(actual.tooltip_datum().label, original.tooltip_datum().label);
    }
    let base = layout.scene.clone();
    for _ in 0..3 {
        layout
            .set_active(Some(HeatmapCellKey::new("r", "a")))
            .unwrap();
        assert_eq!(
            layout
                .scene
                .labels
                .iter()
                .map(|l| l.color)
                .collect::<Vec<_>>(),
            [
                Color(63, 70, 77, 29),
                Color(14, 28, 42, 16),
                Color(140, 126, 112, 22)
            ]
        );
        assert_eq!(
            layout
                .scene
                .labels
                .iter()
                .map(|l| l.text.as_str())
                .collect::<Vec<_>>(),
            ["a:missing", "b:0", "c:10"]
        );
        layout.interaction_binding().unwrap();
        layout.set_active(None).unwrap();
        assert_eq!(layout.scene, base);
    }
}

#[test]
fn label_only_freezes_text_row_and_body_paint_for_binding_and_atomic_hover() {
    for mutation in 0..4 {
        let mut layout = with_text(&options());
        match mutation {
            0 => layout.scene.labels[0].text = "forged".into(),
            1 => {
                layout.scene.label_series.insert(0, "other".into());
            }
            2 => layout.scene.marks[0].fill = Some(Color(1, 2, 3, 255)),
            _ => layout.cells[0].value = Some(0.),
        }
        let bad_scene = layout.scene.clone();
        let before = format!("{:?}", layout);
        assert!(layout.interaction_binding().is_err());
        assert!(
            layout
                .set_active(Some(HeatmapCellKey::new("r", "a")))
                .is_err()
        );
        assert_eq!(layout.scene, bad_scene);
        assert_eq!(format!("{:?}", layout), before);
    }
}

#[test]
fn no_factories_preserves_legacy_decoration_editability_and_existing_api_output() {
    let o = options();
    let old = heatmap_layout(&config(), &data(), &o).unwrap();
    let mut fresh =
        heatmap_layout_with_factories(&config(), &data(), &o, HeatmapFactories::default()).unwrap();
    assert_eq!(fresh.scene, old.scene);
    assert_eq!(
        render::to_svg(&fresh.scene).unwrap(),
        render::to_svg(&old.scene).unwrap()
    );
    assert_eq!(
        render::to_canvas_script(&fresh.scene, "c").unwrap(),
        render::to_canvas_script(&old.scene, "c").unwrap()
    );
    fresh.scene.marks[0].fill = Some(Color(1, 2, 3, 255));
    fresh.scene.labels[0].size = 19.;
    fresh.set_active(None).unwrap();
    fresh.interaction_binding().unwrap();
}

#[test]
fn invalid_configuration_precedes_every_callback_even_without_labels_or_cells() {
    let cells = data();
    for empty in [false, true] {
        for labels in [false, true] {
            let mut count = 0;
            let mut label = |_: &HeatmapLabelContext<'_>| {
                count += 1;
                Ok("bad".into())
            };
            let o = HeatmapLayoutOptions {
                labels,
                opacity: f64::NAN,
                ..options()
            };
            assert!(
                heatmap_layout_with_factories(
                    &config(),
                    if empty { &[] } else { &cells },
                    &o,
                    HeatmapFactories {
                        cell: None,
                        label: Some(&mut label)
                    }
                )
                .is_err()
            );
            assert_eq!(count, 0);
        }
    }
}

#[test]
fn callback_error_and_bad_utf8_byte_length_do_not_mutate_inputs_or_accepted_frame() {
    let cells = data();
    let input_before = format!("{:?}", cells);
    let accepted = with_text(&options());
    let accepted_before = format!("{:?}", accepted);
    for bad_length in [false, true] {
        let mut label_count = 0;
        let mut cell_count = 0;
        let mut label = |_: &HeatmapLabelContext<'_>| -> Result<String> {
            label_count += 1;
            if bad_length {
                Ok("é".repeat(512) + "x")
            } else {
                Err(ChartError("intentional".into()))
            }
        };
        let mut cell = |ctx: &HeatmapCellContext<'_>| {
            cell_count += 1;
            Ok(ctx.inherit(ctx.geometry.clone()))
        };
        let err = heatmap_layout_with_factories(
            &config(),
            &cells,
            &options(),
            HeatmapFactories {
                cell: Some(&mut cell),
                label: Some(&mut label),
            },
        )
        .unwrap_err();
        assert!(err.0.contains(if bad_length {
            "text budget"
        } else {
            "intentional"
        }));
        assert_eq!(label_count, 1);
        assert_eq!(cell_count, 0);
        assert_eq!(format!("{:?}", cells), input_before);
        assert_eq!(format!("{:?}", accepted), accepted_before);
    }
}

#[test]
fn exact_1024_byte_result_is_valid_and_escapes_as_owned_text() {
    let text = "<&é".repeat(256);
    assert_eq!(text.len(), 1024);
    let mut label = |_: &HeatmapLabelContext<'_>| Ok(text.clone());
    let layout = heatmap_layout_with_factories(
        &config(),
        &data(),
        &options(),
        HeatmapFactories {
            cell: None,
            label: Some(&mut label),
        },
    )
    .unwrap();
    assert_eq!(layout.scene.labels[0].text, text);
    assert!(
        render::to_svg(&layout.scene)
            .unwrap()
            .contains("&lt;&amp;é")
    );
    layout.interaction_binding().unwrap();
}

#[test]
fn scene_budget_includes_axis_labels_and_rejects_before_first_callback() {
    let cells = (0..MAX_HEATMAP_CELLS)
        .map(|i| HeatCell::new("r", format!("c{i:05}"), 1.))
        .collect::<Vec<_>>();
    let mut o = options();
    let mut axis = HeatmapAxis::default();
    axis.options.tick_values = Some(vec!["c00000".into()]);
    o.axes.bottom = Some(axis);
    let mut count = 0;
    let mut label = |_: &HeatmapLabelContext<'_>| {
        count += 1;
        Ok("x".into())
    };
    let err = heatmap_layout_with_factories(
        &config(),
        &cells,
        &o,
        HeatmapFactories {
            cell: None,
            label: Some(&mut label),
        },
    )
    .unwrap_err();
    assert!(err.0.contains("before factories"));
    assert_eq!(count, 0);
}

#[test]
fn actual_sparse_label_budget_uses_sources_not_row_column_product() {
    let cells = (0..150)
        .map(|i| HeatCell::new(format!("r{i}"), format!("c{i}"), i as f64))
        .collect::<Vec<_>>();
    let mut count = 0;
    let mut label = |ctx: &HeatmapLabelContext<'_>| {
        count += 1;
        Ok(ctx.default_text.to_owned())
    };
    let layout = heatmap_layout_with_factories(
        &config(),
        &cells,
        &options(),
        HeatmapFactories {
            cell: None,
            label: Some(&mut label),
        },
    )
    .unwrap();
    assert_eq!(count, 150);
    assert_eq!(layout.scene.labels.len(), 150);
    assert_eq!(layout.interaction_binding().unwrap().sites.len(), 150);
}
