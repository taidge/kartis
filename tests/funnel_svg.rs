use std::collections::BTreeSet;

use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 372.,
        height: 284.,
        margin: Margin {
            left: 23.,
            right: 17.,
            top: 19.,
            bottom: 13.,
        },
        ..Default::default()
    }
}
fn data() -> Vec<FunnelDatum> {
    vec![FunnelDatum::new(1., 80.), FunnelDatum::new("1", 40.)]
}
fn options() -> FunnelLayoutOptions {
    FunnelLayoutOptions {
        before: None,
        after: None,
        border_width: 2.,
        ..Default::default()
    }
}
fn layer(key: &str, kind: FunnelSvgLayerKind) -> FunnelSvgLayer {
    FunnelSvgLayer::new(key, kind).unwrap()
}
fn plan(frame: &FunnelSvgFrame, layers: Vec<FunnelSvgLayer>) -> Result<FunnelSvgPlan> {
    FunnelSvgPlan::new(
        frame,
        layers,
        FunnelSvgBudget::default(),
        FunnelSvgExtrasPolicy::RequireLayer,
    )
}
fn part_items(frame: &FunnelSvgFrame) -> Vec<DrawItem> {
    frame
        .scene()
        .draw_items()
        .into_iter()
        .filter(|item| match item {
            DrawItem::Mark(index) => matches!(
                frame.mark_role(*index).unwrap(),
                Some(FunnelSvgMarkRole::Body { .. } | FunnelSvgMarkRole::Border { .. })
            ),
            _ => false,
        })
        .collect()
}

#[test]
fn literal_omission_and_repetition_reference_one_full_business_source() {
    let frame = FunnelSvgFrame::new(&config(), &data(), &options()).unwrap();
    let baseline = frame.scene().clone();
    let labels_only = plan(
        &frame,
        vec![layer("labels-only", FunnelSvgLayerKind::Labels)],
    )
    .unwrap();
    assert!(labels_only.body_mark_indices().is_empty());
    assert!(labels_only.mark_indices().is_empty());
    assert_eq!(labels_only.frame().parts().len(), 2);
    assert_eq!(labels_only.frame().source(), data());
    assert_eq!(labels_only.frame(), &frame);
    assert_eq!(labels_only.layers()[0].items().len(), 2);

    let repeated = plan(
        &frame,
        vec![
            layer("under", FunnelSvgLayerKind::Parts),
            layer("middle", FunnelSvgLayerKind::Custom("control".into())),
            layer("over", FunnelSvgLayerKind::Parts),
        ],
    )
    .unwrap();
    let expected = part_items(&frame);
    assert_eq!(repeated.layers()[0].items(), expected);
    assert!(repeated.layers()[1].items().is_empty());
    assert_eq!(repeated.layers()[2].items(), expected);
    assert_eq!(
        repeated
            .layers()
            .iter()
            .map(|o| o.index)
            .collect::<Vec<_>>(),
        [0, 1, 2]
    );
    assert_eq!(
        repeated
            .layers()
            .iter()
            .map(|o| o.layer.key.as_str())
            .collect::<Vec<_>>(),
        ["under", "middle", "over"]
    );
    assert_eq!(repeated.mark_indices().len(), expected.len() * 2);
    assert_eq!(
        repeated.body_mark_indices().len(),
        2,
        "source identity is not duplicated by emitted views"
    );
    assert_eq!(frame.layout().interaction_binding().sites.len(), 2);
    assert_eq!(
        frame.scene(),
        &baseline,
        "a literal plan never mutates canonical source ink or sites"
    );
    assert_eq!(frame.part_by_id(&CategoryKey::Number(1.)).unwrap(), Some(0));
    assert_eq!(
        frame.part_by_id(&CategoryKey::String("1".into())).unwrap(),
        Some(1)
    );
}

#[test]
fn actual_authored_draw_order_is_preserved_inside_each_repeated_role() {
    let mut layout = funnel_layout(&config(), &data(), &options()).unwrap();
    // A legal authored source paint change must not be silently replaced with a
    // new source-index traversal or inferred default body/border ordering.
    let mut order = layout.scene.draw_items();
    order.reverse();
    layout.scene.draw_order = order.clone();
    let frame = FunnelSvgFrame::from_layout(layout).unwrap();
    let expected_parts: Vec<_> = order
        .iter()
        .copied()
        .filter(|item| matches!(item, DrawItem::Mark(_)))
        .collect();
    let expected_labels: Vec<_> = order
        .iter()
        .copied()
        .filter(|item| matches!(item, DrawItem::Label(_)))
        .collect();
    let selected = plan(
        &frame,
        vec![
            layer("labels-front", FunnelSvgLayerKind::Labels),
            layer("body-copy-one", FunnelSvgLayerKind::Parts),
            layer("body-copy-two", FunnelSvgLayerKind::Parts),
        ],
    )
    .unwrap();
    assert_eq!(selected.layers()[0].items(), expected_labels);
    assert_eq!(selected.layers()[1].items(), expected_parts);
    assert_eq!(selected.layers()[2].items(), expected_parts);
    assert_eq!(frame.scene().draw_items(), order);
}

#[test]
fn disabled_labels_and_empty_plans_keep_complete_part_and_separator_data() {
    let frame = FunnelSvgFrame::new(
        &config(),
        &data(),
        &FunnelLayoutOptions {
            labels: None,
            before: Some(FunnelSeparatorOptions {
                length: 11.,
                offset: 3.,
                ..Default::default()
            }),
            after: Some(FunnelSeparatorOptions {
                length: 17.,
                offset: 5.,
                ..Default::default()
            }),
            ..options()
        },
    )
    .unwrap();
    assert_eq!(
        frame.inner_size(),
        [332., 252.],
        "inner props do not lose separator-reserved width"
    );
    assert_eq!(frame.inner_position([23., 19.]).unwrap(), [0., 0.]);
    assert!(frame.inner_position([f64::INFINITY, 0.]).is_err());
    let selected = plan(
        &frame,
        vec![
            layer("not-painted-labels", FunnelSvgLayerKind::Labels),
            layer(
                "full-data-component",
                FunnelSvgLayerKind::Custom("reader".into()),
            ),
        ],
    )
    .unwrap();
    assert!(
        selected
            .layers()
            .iter()
            .all(|occurrence| occurrence.items().is_empty())
    );
    assert!(selected.body_mark_indices().is_empty());
    assert_eq!(selected.frame().parts().len(), 2);
    assert_eq!(selected.frame().styles().len(), 2);
    assert_eq!(selected.frame().separators().len(), 6);
    assert!(
        selected
            .frame()
            .separators()
            .iter()
            .any(|separator| separator.part_id.is_none()),
        "closing separator has no invented source datum"
    );
    let omitted = FunnelSvgPlan::new(
        &frame,
        vec![],
        FunnelSvgBudget {
            layers: 0,
            work: 0,
            source_commands: 0,
            components: 0,
        },
        FunnelSvgExtrasPolicy::RequireLayer,
    )
    .unwrap();
    assert!(omitted.layers().is_empty());
    assert_eq!(omitted.work(), 0);
    assert_eq!(omitted.frame(), &frame);
}

#[test]
fn scene_extras_are_explicit_decoration_separate_from_part_labels() {
    let mut layout = funnel_layout(&config(), &data(), &options()).unwrap();
    let mut order = layout.scene.draw_items();
    let extra_mark = layout.scene.marks.len();
    let extra_label = layout.scene.labels.len();
    let mut mark = Mark::new(
        "application-decoration",
        Geometry::Rect {
            x: 30.,
            y: 40.,
            width: 11.,
            height: 13.,
        },
        Color(17, 31, 47, 255),
    );
    mark.label.clear();
    layout.scene.marks.push(mark);
    layout.scene.labels.push(Label {
        x: 45.,
        y: 56.,
        text: "not a bound Funnel annotation".into(),
        color: Color(30, 60, 90, 255),
        size: 13.,
        anchor: TextAnchor::Middle,
        rotation: 25.,
    });
    order.insert(1, DrawItem::Label(extra_label));
    order.insert(3, DrawItem::Mark(extra_mark));
    layout.scene.draw_order = order;
    let frame = FunnelSvgFrame::from_layout(layout).unwrap();
    assert_eq!(
        frame.extras(),
        [DrawItem::Label(extra_label), DrawItem::Mark(extra_mark)]
    );
    assert_eq!(frame.label_part_index(extra_label).unwrap(), None);
    assert_eq!(frame.mark_role(extra_mark).unwrap(), None);
    assert!(plan(&frame, vec![layer("parts", FunnelSvgLayerKind::Parts)]).is_err());
    let omitted = FunnelSvgPlan::new(
        &frame,
        vec![layer("parts", FunnelSvgLayerKind::Parts)],
        FunnelSvgBudget::default(),
        FunnelSvgExtrasPolicy::Omit,
    )
    .unwrap();
    assert!(
        !omitted.layers()[0]
            .items()
            .contains(&DrawItem::Mark(extra_mark))
    );
    let explicit = plan(
        &frame,
        vec![
            layer("decoration", FunnelSvgLayerKind::SceneExtras),
            layer("labels", FunnelSvgLayerKind::Labels),
        ],
    )
    .unwrap();
    assert_eq!(explicit.layers()[0].items(), frame.extras());
    assert!(
        !explicit.layers()[1]
            .items()
            .contains(&DrawItem::Label(extra_label))
    );
    assert_eq!(frame.scene().marks.len(), extra_mark + 1);
    assert_eq!(
        frame.layout().interaction_binding().sites.len(),
        2,
        "decoration is not a synthetic business site"
    );
}

#[test]
fn literal_work_and_command_budgets_count_each_copy_without_double_counting_identity() {
    let frame = FunnelSvgFrame::new(
        &config(),
        &data(),
        &FunnelLayoutOptions {
            labels: None,
            ..options()
        },
    )
    .unwrap();
    assert_eq!(
        frame.scene().marks.len(),
        4,
        "two bodies and two open borders"
    );
    let commands: usize = frame
        .scene()
        .marks
        .iter()
        .map(|mark| match &mark.geometry {
            Geometry::Path(commands) => commands.len(),
            _ => panic!("authored profile generates paths"),
        })
        .sum();
    let layers = vec![
        layer("first", FunnelSvgLayerKind::Parts),
        layer("one", FunnelSvgLayerKind::Custom("shared-registry".into())),
        layer("second", FunnelSvgLayerKind::Parts),
        layer("two", FunnelSvgLayerKind::Custom("shared-registry".into())),
    ];
    let exact = FunnelSvgBudget {
        layers: 4,
        work: 10,
        source_commands: commands * 2,
        components: 2,
    };
    let accepted = FunnelSvgPlan::new(
        &frame,
        layers.clone(),
        exact,
        FunnelSvgExtrasPolicy::RequireLayer,
    )
    .unwrap();
    assert_eq!(accepted.work(), 10);
    assert_eq!(accepted.body_mark_indices().len(), 2);
    for budget in [
        FunnelSvgBudget { layers: 3, ..exact },
        FunnelSvgBudget { work: 9, ..exact },
        FunnelSvgBudget {
            source_commands: commands * 2 - 1,
            ..exact
        },
        FunnelSvgBudget {
            components: 1,
            ..exact
        },
    ] {
        assert!(
            FunnelSvgPlan::new(
                &frame,
                layers.clone(),
                budget,
                FunnelSvgExtrasPolicy::RequireLayer
            )
            .is_err()
        );
    }
    assert_eq!(
        accepted.layers()[1].layer.kind,
        accepted.layers()[3].layer.kind,
        "registry reuse differs from occurrence-key reuse"
    );
}

#[test]
fn revalidating_public_layer_values_rejects_duplicate_invalid_keys_and_overlarge_budgets() {
    let frame = FunnelSvgFrame::new(&config(), &data(), &options()).unwrap();
    let repeated_key = vec![
        layer("same", FunnelSvgLayerKind::Parts),
        layer("same", FunnelSvgLayerKind::Labels),
    ];
    assert!(plan(&frame, repeated_key).is_err());
    for key in [String::new(), " ".into(), "é".repeat(65)] {
        assert!(FunnelSvgLayer::new(&key, FunnelSvgLayerKind::Parts).is_err());
    }
    assert!(
        FunnelSvgLayer::new("valid-occurrence", FunnelSvgLayerKind::Custom(" ".into())).is_err()
    );
    let mut mutated = layer("was-valid", FunnelSvgLayerKind::Parts);
    mutated.key.clear();
    assert!(
        plan(&frame, vec![mutated]).is_err(),
        "public fields must be checked at admission"
    );
    assert!(
        FunnelSvgPlan::validate_layers(
            &[],
            FunnelSvgBudget {
                layers: MAX_FUNNEL_SVG_LAYERS + 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        FunnelSvgPlan::validate_layers(
            &[],
            FunnelSvgBudget {
                work: MAX_FUNNEL_SVG_WORK + 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        FunnelSvgPlan::validate_layers(
            &[],
            FunnelSvgBudget {
                source_commands: MAX_FUNNEL_SVG_SOURCE_COMMANDS + 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        FunnelSvgPlan::validate_layers(
            &[],
            FunnelSvgBudget {
                components: MAX_FUNNEL_SVG_COMPONENTS + 1,
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn body_index_union_is_an_actual_source_projection_not_a_solid_alpha_or_clip_oracle() {
    let mut source = data();
    source[0].paint = Some(FillPaint::Linear {
        start: [0., 0.],
        end: [100., 100.],
        stops: vec![
            GradientStop::new(0., Color(200, 10, 30, 255)),
            GradientStop::new(1., Color(10, 80, 200, 0)),
        ],
    });
    source.push(FunnelDatum::new("last-zero", -0.));
    let frame = FunnelSvgFrame::new(
        &config(),
        &source,
        &FunnelLayoutOptions {
            fill_opacity: 0.,
            labels: None,
            ..options()
        },
    )
    .unwrap();
    let selected = plan(
        &frame,
        vec![
            layer("first", FunnelSvgLayerKind::Parts),
            layer("second", FunnelSvgLayerKind::Parts),
        ],
    )
    .unwrap();
    let bodies: BTreeSet<_> = frame
        .scene()
        .marks
        .iter()
        .enumerate()
        .filter_map(|(index, _)| {
            matches!(
                frame.mark_role(index).unwrap(),
                Some(FunnelSvgMarkRole::Body { .. })
            )
            .then_some(index)
        })
        .collect();
    assert_eq!(bodies.len(), 2);
    assert_eq!(selected.body_mark_indices(), &bodies);
    assert!(frame.body_mark(0).unwrap().unwrap().fill_paint.is_some());
    assert_eq!(frame.body_mark(0).unwrap().unwrap().opacity, 0.);
    assert!(
        frame.body_mark(2).unwrap().is_none(),
        "zero terminal profile has no fabricated body Mark"
    );
    assert_eq!(
        selected.frame().source()[2].value.to_bits(),
        (-0f64).to_bits()
    );
}

#[test]
fn opacity_groups_must_remain_whole_in_each_literal_occurrence() {
    let mut layout = funnel_layout(
        &config(),
        &data(),
        &FunnelLayoutOptions {
            labels: None,
            ..options()
        },
    )
    .unwrap();
    layout.scene.opacity_groups.push(MarkOpacityGroup {
        id: "one-part-isolation".into(),
        members: layout.scene.marks[..2]
            .iter()
            .map(|mark| mark.id.clone())
            .collect(),
        opacity: 0.5,
    });
    let grouped = FunnelSvgFrame::from_layout(layout).unwrap();
    assert!(
        plan(
            &grouped,
            vec![
                layer("one", FunnelSvgLayerKind::Parts),
                layer("two", FunnelSvgLayerKind::Parts)
            ]
        )
        .is_ok()
    );

    let mut cross = funnel_layout(
        &config(),
        &data(),
        &FunnelLayoutOptions {
            labels: None,
            before: Some(FunnelSeparatorOptions {
                length: 8.,
                offset: 3.,
                ..Default::default()
            }),
            ..options()
        },
    )
    .unwrap();
    let boundary = cross
        .scene
        .marks
        .windows(2)
        .find(|pair| {
            pair[0].id.starts_with("funnel:separator:") && pair[1].id.starts_with("funnel:border:")
        })
        .unwrap();
    let members = boundary.iter().map(|mark| mark.id.clone()).collect();
    cross.scene.opacity_groups.push(MarkOpacityGroup {
        id: "cross-role-isolation".into(),
        members,
        opacity: 0.5,
    });
    let cross = FunnelSvgFrame::from_layout(cross).unwrap();
    assert!(
        plan(
            &cross,
            vec![
                layer("separators", FunnelSvgLayerKind::Separators),
                layer("parts", FunnelSvgLayerKind::Parts)
            ]
        )
        .is_err(),
        "splitting an isolated composite changes source-over semantics"
    );
    assert!(
        plan(
            &cross,
            vec![layer(
                "only-component",
                FunnelSvgLayerKind::Custom("reader".into())
            )]
        )
        .is_ok(),
        "omitting the complete group is distinct from splitting it"
    );
}
