use std::collections::{BTreeMap, BTreeSet};

use kartis::*;

fn font(family: FontFamily, weight: Option<f64>) -> FontOptions {
    FontOptions {
        family: Some(vec![family]),
        weight,
    }
}

fn label(text: &str) -> Label {
    Label {
        rotation: 0.,
        x: 30.,
        y: 40.,
        text: text.into(),
        color: Color(0, 0, 0, 255),
        size: 12.,
        anchor: TextAnchor::Start,
    }
}

#[test]
fn theme_survives_scene_construction_and_per_slot_inheritance() {
    let mut config = ChartConfig::default();
    config.theme.font = font(FontFamily::Named("Chart Display".into()), Some(650.5));
    let mut scene = config.scene().unwrap();
    scene.labels = vec![label("axis"), label("value")];
    scene.typography.labels.insert(
        1,
        FontOptions {
            weight: Some(700.),
            ..Default::default()
        },
    );
    scene.validate().unwrap();
    assert_eq!(scene.resolved_label_font(0).weight, 650.5);
    assert_eq!(scene.resolved_label_font(1).weight, 700.);
    assert_eq!(
        scene.resolved_label_font(1).family,
        vec![FontFamily::Named("Chart Display".into())]
    );
    assert!(scene.typography.has_label_font(0));
    assert!(!SceneTypography::default().has_label_font(0));
}

#[test]
fn legend_tooltip_role_precedence_and_legacy_weight_are_independent() {
    let mut typography = SceneTypography::default();
    let empty = FontOptions::default();
    assert_eq!(typography.legend_font(&empty, 700.).weight, 700.);
    typography.default = font(FontFamily::Serif, None);
    assert_eq!(typography.legend_font(&empty, 700.).weight, 700.);
    typography.default.weight = Some(500.);
    typography.legend = font(FontFamily::Mono, Some(600.));
    typography.tooltip.weight = Some(800.);
    let local = FontOptions {
        weight: Some(450.25),
        ..Default::default()
    };
    assert_eq!(typography.legend_font(&local, 700.).weight, 450.25);
    assert_eq!(typography.legend_font(&empty, 700.).weight, 600.);
    assert_eq!(
        typography.legend_font(&empty, 700.).family,
        vec![FontFamily::Mono]
    );
    assert_eq!(typography.tooltip_font(&empty, 700.).weight, 800.);
    assert_eq!(
        typography.tooltip_font(&empty, 700.).family,
        vec![FontFamily::Serif]
    );
    let measured =
        PresentationText::default().with_resolved_font(&typography.legend_font(&local, 700.));
    let style = text_style(&measured);
    assert_eq!(style.font_family, Some(vec![FontFamily::Mono]));
    assert_eq!(style.font_weight, Some(450.25));
    assert_eq!(
        measured.resolved_font(&ResolvedFont::default()).weight,
        450.25
    );
}

#[test]
fn hiding_remaps_font_slots_and_restore_preserves_authored_scene() {
    let mut scene = ChartConfig::default().scene().unwrap();
    scene.labels = vec![label("A"), label("B"), label("unowned")];
    scene.label_series.insert(0, "A".into());
    scene.label_series.insert(1, "B".into());
    scene.typography.labels = BTreeMap::from([
        (0, font(FontFamily::Serif, Some(700.))),
        (1, font(FontFamily::Mono, Some(500.))),
        (2, font(FontFamily::Fantasy, None)),
    ]);
    let hidden = scene.without_series(&BTreeSet::from(["A".into()]));
    hidden.validate().unwrap();
    assert_eq!(hidden.labels[0].text, "B");
    assert_eq!(hidden.resolved_label_font(0).family, vec![FontFamily::Mono]);
    assert_eq!(
        hidden.resolved_label_font(1).family,
        vec![FontFamily::Fantasy]
    );
    assert!(!hidden.typography.labels.contains_key(&2));
    assert_eq!(scene.labels[0].text, "A");
    assert_eq!(scene.resolved_label_font(0).weight, 700.);
    let restored = scene.without_series(&BTreeSet::new());
    assert_eq!(restored, scene);
}

#[test]
fn font_only_animation_uses_target_declarations_without_scalar_blending() {
    let mut source = ChartConfig::default().scene().unwrap();
    source.labels.push(label("same text"));
    let mut target = source.clone();
    target.typography.default = font(FontFamily::Mono, Some(700.));
    let halfway = source.interpolate(&target, 0.5).unwrap();
    assert_eq!(
        halfway.resolved_label_font(0),
        target.resolved_label_font(0)
    );
    assert_eq!(halfway.labels, source.labels);
    assert_eq!(source.resolved_label_font(0), ResolvedFont::default());
}

#[test]
fn bounded_declarations_reject_invalid_or_dangling_profiles() {
    for family in [
        vec![],
        vec![FontFamily::Sans; 17],
        vec![FontFamily::Named("   ".into())],
        vec![FontFamily::Named("bad\0face".into())],
        vec![FontFamily::Named("bad\nface".into())],
        vec![FontFamily::Named("x".repeat(257))],
        vec![FontFamily::Named("x".repeat(256)); 9],
    ] {
        let mut config = ChartConfig::default();
        config.theme.font.family = Some(family);
        assert!(config.scene().is_err());
    }
    for weight in [0., 1000.1, f64::NAN, f64::INFINITY] {
        assert!(
            FontOptions {
                weight: Some(weight),
                ..Default::default()
            }
            .validate()
            .is_err()
        );
    }
    let mut scene = ChartConfig::default().scene().unwrap();
    scene.labels.push(label("valid"));
    scene.typography.labels.insert(1, FontOptions::default());
    assert!(scene.validate().is_err());
    scene.typography.labels.clear();
    scene.typography.legend.weight = Some(f64::NAN);
    assert!(scene.validate().is_err());
}

#[test]
fn css_names_are_quoted_and_do_not_become_generic_or_declarations() {
    let font = ResolvedFont {
        family: vec![
            FontFamily::Named("serif".into()),
            FontFamily::Named("A\"; font-size:99px; \\ B".into()),
            FontFamily::Serif,
            FontFamily::Sans,
            FontFamily::Mono,
            FontFamily::System,
            FontFamily::Cursive,
            FontFamily::Fantasy,
        ],
        weight: 625.5,
    };
    font.validate().unwrap();
    assert_eq!(
        font.css_family(),
        "\"serif\", \"A\\\"; font-size:99px; \\\\ B\", serif, sans-serif, monospace, system-ui, cursive, fantasy"
    );
    assert!(font.canvas_font(12.).starts_with("625.5 12px \"serif\""));
    assert_eq!(ResolvedFont::default().canvas_font(12.), "12px sans-serif");
}

fn hierarchy_data() -> Vec<HierarchyDatum> {
    vec![
        HierarchyDatum::new("root", None, 0.),
        HierarchyDatum::new("a", Some("root".into()), 3.),
        HierarchyDatum::new("b", Some("root".into()), 2.),
    ]
}

fn ink() -> LayoutRect {
    LayoutRect {
        x: -1.,
        y: -1.,
        width: 2.,
        height: 2.,
    }
}

fn add_unmanaged(scene: &mut Scene) {
    let slot = scene.labels.len();
    scene.labels.push(label("unmanaged"));
    scene
        .typography
        .labels
        .insert(slot, font(FontFamily::Fantasy, Some(300.)));
}

fn assert_unmanaged(scene: &Scene) {
    let slot = scene
        .labels
        .iter()
        .position(|label| label.text == "unmanaged")
        .unwrap();
    assert_eq!(
        scene.resolved_label_font(slot).family,
        vec![FontFamily::Fantasy]
    );
    assert_eq!(scene.resolved_label_font(slot).weight, 300.);
}

#[test]
fn hierarchy_font_changes_invalidate_measurements_and_survive_hide_restore() {
    for view in [
        HierarchyView::Icicle(Default::default()),
        HierarchyView::Treemap(Default::default()),
        HierarchyView::CirclePacking(Default::default()),
    ] {
        let mut source = HierarchyFocusController::new(&hierarchy_data())
            .unwrap()
            .layout(&ChartConfig::default(), &view)
            .unwrap();
        assert!(!source.label_candidates().is_empty());
        add_unmanaged(&mut source.scene);
        let old: Vec<_> = source
            .label_candidates()
            .iter()
            .map(|c| c.measured(ink()))
            .collect();
        let mut inherited_hidden_options = HierarchyLabelPlacementOptions::for_layout(&source);
        inherited_hidden_options.hidden = source
            .label_candidates()
            .iter()
            .map(|c| c.node_id.clone())
            .collect();
        let mut inherited_hidden = source
            .place_labels(&old, &inherited_hidden_options)
            .unwrap();
        inherited_hidden.scene.typography.default.weight = Some(700.);
        assert!(
            inherited_hidden
                .place_labels(&old, &inherited_hidden_options)
                .is_err()
        );
        let changed = source
            .with_label_font(font(FontFamily::Mono, Some(700.)))
            .unwrap();
        assert_unmanaged(&changed.scene);
        let mut options = HierarchyLabelPlacementOptions::for_layout(&changed);
        assert!(changed.place_labels(&old, &options).is_err());
        let current: Vec<_> = changed
            .label_candidates()
            .iter()
            .map(|c| {
                assert_eq!(c.font.family, vec![FontFamily::Mono]);
                assert_eq!(c.font.weight, 700.);
                c.measured(ink())
            })
            .collect();
        options.hidden = changed
            .label_candidates()
            .iter()
            .map(|c| c.node_id.clone())
            .collect();
        let hidden = changed.place_labels(&current, &options).unwrap();
        assert!(
            hidden
                .label_placements()
                .iter()
                .all(|p| p.label_index.is_none())
        );
        assert_unmanaged(&hidden.scene);
        options.hidden.clear();
        let restored = hidden.place_labels(&current, &options).unwrap();
        assert!(
            restored
                .label_placements()
                .iter()
                .any(|p| p.label_index.is_some())
        );
        for placed in restored.label_placements() {
            if let Some(slot) = placed.label_index {
                assert_eq!(restored.scene.resolved_label_font(slot).weight, 700.);
            }
        }
        assert_unmanaged(&restored.scene);
        assert!(restored.with_label_font(FontOptions::default()).is_err());
        let mut tampered = hidden.clone();
        tampered.scene.typography.default.family = Some(vec![FontFamily::Serif]);
        // The managed override is complete, so unrelated parent family edits
        // cannot invalidate an otherwise identical font measurement.
        tampered.place_labels(&current, &options).unwrap();
        let mut tampered = changed.clone();
        let slot = tampered
            .nodes
            .iter()
            .flat_map(|node| &node.label_slots)
            .next()
            .copied()
            .unwrap();
        tampered
            .scene
            .typography
            .labels
            .get_mut(&slot)
            .unwrap()
            .weight = Some(800.);
        assert!(tampered.place_labels(&current, &options).is_err());
    }
}

#[test]
fn sunburst_font_signatures_and_hidden_slot_fonts_survive_real_arc_masks() {
    let mut source = sunburst_layout(
        &ChartConfig::default(),
        &hierarchy_data(),
        &SunburstLayoutOptions {
            labels: Some(SunburstLabelOptions::default()),
            ..Default::default()
        },
    )
    .unwrap();
    add_unmanaged(&mut source.scene);
    let old: Vec<_> = source
        .label_candidates()
        .iter()
        .map(|c| c.measured(ink()))
        .collect();
    let changed = source
        .with_label_font(font(FontFamily::Serif, Some(700.)))
        .unwrap();
    changed.interaction_binding().unwrap();
    let mut options = SunburstLabelPlacementOptions::for_layout(&changed);
    options.clip = SunburstLabelClip::ClipToArc;
    assert!(changed.place_labels(&old, &options).is_err());
    let current: Vec<_> = changed
        .label_candidates()
        .iter()
        .map(|c| c.measured(ink()))
        .collect();
    assert!(!current.is_empty());
    options.hidden = changed
        .label_candidates()
        .iter()
        .map(|c| c.node_id.clone())
        .collect();
    let hidden = changed.place_labels(&current, &options).unwrap();
    assert_unmanaged(&hidden.scene);
    options.hidden.clear();
    let restored = hidden.place_labels(&current, &options).unwrap();
    let slots: Vec<_> = restored
        .label_placements()
        .iter()
        .filter_map(|p| p.label_index)
        .collect();
    assert!(!slots.is_empty());
    for slot in slots {
        assert_eq!(restored.scene.resolved_label_font(slot).weight, 700.);
        assert_eq!(
            restored.scene.resolved_label_font(slot).family,
            vec![FontFamily::Serif]
        );
        assert!(restored.scene.label_clips.contains_key(&slot));
    }
    assert_unmanaged(&restored.scene);
    assert!(restored.with_label_font(FontOptions::default()).is_err());
}

#[test]
fn area_bump_endpoint_fonts_survive_omission_restoration_and_focus() {
    let config = ChartConfig {
        width: 460.,
        height: 200.,
        margin: Margin {
            left: 120.,
            right: 120.,
            top: 20.,
            bottom: 20.,
        },
        ..Default::default()
    };
    let rows = vec![AreaBumpSeries::new(
        "a",
        vec![
            AreaBumpDatum::new("start", 3.),
            AreaBumpDatum::new("end", 2.),
        ],
    )];
    let mut source = layout_area_bump(
        &config,
        &rows,
        &AreaBumpOptions {
            axis_bottom: None,
            axis_top: None,
            ..Default::default()
        },
    )
    .unwrap();
    add_unmanaged(&mut source.scene);
    let bounds = |c: &AreaBumpLabelCandidate| LayoutRect {
        x: if c.side == AreaBumpLabelSide::Start {
            -3.
        } else {
            1.
        },
        ..ink()
    };
    let old: Vec<_> = source
        .endpoint_label_candidates()
        .iter()
        .map(|c| c.measured(bounds(c)))
        .collect();
    let changed = source
        .with_label_font(font(FontFamily::Mono, Some(600.)))
        .unwrap();
    let options = AreaBumpLabelPlacementOptions::for_scene(&changed.scene);
    assert!(changed.place_endpoint_labels(&old, &options).is_err());
    let current: Vec<_> = changed
        .endpoint_label_candidates()
        .iter()
        .map(|c| c.measured(bounds(c)))
        .collect();
    let mut omitted = options.clone();
    omitted.collision.viewport.width = 0.;
    let hidden = changed.place_endpoint_labels(&current, &omitted).unwrap();
    assert!(
        hidden
            .endpoint_label_placements()
            .iter()
            .all(|p| p.label_index.is_none())
    );
    assert_unmanaged(&hidden.scene);
    let mut restored = hidden.place_endpoint_labels(&current, &options).unwrap();
    assert_eq!(
        restored
            .endpoint_label_placements()
            .iter()
            .filter_map(|p| p.label_index)
            .count(),
        2
    );
    restored.set_active(BTreeSet::from(["a".into()])).unwrap();
    for slot in restored
        .endpoint_label_placements()
        .iter()
        .filter_map(|p| p.label_index)
    {
        assert_eq!(restored.scene.resolved_label_font(slot).weight, 600.);
    }
    assert_unmanaged(&restored.scene);
    assert!(restored.with_label_font(FontOptions::default()).is_err());
}

#[test]
fn pie_link_fonts_invalidate_stale_measurements_and_retain_base_arc_fonts() {
    let mut options = PieLayoutOptions::default();
    options.pie.style.link_labels.enabled = true;
    let source = pie_layout(
        &ChartConfig::default(),
        &[RadialDatum::new("a", 1.), RadialDatum::new("b", 1.)],
        &options,
    )
    .unwrap();
    let bounds = |c: &PieLinkLabelCandidate| LayoutRect {
        x: if c.label.anchor == TextAnchor::End {
            -2.
        } else {
            0.
        },
        ..ink()
    };
    let old: Vec<_> = source
        .link_candidates
        .iter()
        .map(|c| c.measured(bounds(c)))
        .collect();
    let changed = source
        .with_label_font(font(FontFamily::Serif, Some(700.)))
        .unwrap();
    changed.interaction_binding().unwrap();
    let placement = PieLinkPlacementOptions {
        collision: LabelCollisionOptions {
            viewport: LayoutRect {
                x: 0.,
                y: 0.,
                width: 800.,
                height: 440.,
            },
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(changed.place_link_labels(&old, &placement).is_err());
    let current: Vec<_> = changed
        .link_candidates
        .iter()
        .map(|c| {
            assert_eq!(c.font.weight, 700.);
            c.measured(bounds(c))
        })
        .collect();
    let mut omitted = placement.clone();
    omitted.collision.viewport.width = 0.;
    let hidden = changed.place_link_labels(&current, &omitted).unwrap();
    let restored = hidden.place_link_labels(&current, &placement).unwrap();
    restored.interaction_binding().unwrap();
    assert!(
        restored
            .link_placements
            .iter()
            .all(|p| p.label_index.is_some())
    );
    for slot in 0..restored.scene.labels.len() {
        assert_eq!(restored.scene.resolved_label_font(slot).weight, 700.);
    }
    assert!(restored.with_label_font(FontOptions::default()).is_err());
}

#[test]
fn maximum_legal_family_names_remain_admissible_measurement_signatures() {
    let family = FontOptions {
        family: Some(vec![FontFamily::Named("\"".repeat(256)); 8]),
        weight: Some(700.),
    };
    family.validate().unwrap();
    let node = "n".repeat(1000);
    let root = "r".repeat(1000);
    let rows = vec![
        HierarchyDatum::new(&root, None, 0.),
        HierarchyDatum::new(&node, Some(root.clone()), 1.),
    ];
    let config = ChartConfig {
        theme: Theme {
            font: family.clone(),
            ..Default::default()
        },
        ..Default::default()
    };
    let hierarchy = HierarchyFocusController::new(&rows)
        .unwrap()
        .layout(&config, &HierarchyView::Icicle(Default::default()))
        .unwrap();
    let measured: Vec<_> = hierarchy
        .label_candidates()
        .iter()
        .map(|c| c.measured(ink()))
        .collect();
    hierarchy
        .place_labels(
            &measured,
            &HierarchyLabelPlacementOptions::for_layout(&hierarchy),
        )
        .unwrap();
    let sunburst = sunburst_layout(
        &config,
        &rows,
        &SunburstLayoutOptions {
            labels: Some(SunburstLabelOptions::default()),
            ..Default::default()
        },
    )
    .unwrap();
    let measured: Vec<_> = sunburst
        .label_candidates()
        .iter()
        .map(|c| c.measured(ink()))
        .collect();
    sunburst
        .place_labels(
            &measured,
            &SunburstLabelPlacementOptions::for_layout(&sunburst),
        )
        .unwrap();
    let mut options = PieLayoutOptions::default();
    options.pie.style.link_labels.enabled = true;
    let pie = pie_layout(&config, &[RadialDatum::new(node, 1.)], &options).unwrap();
    let measured: Vec<_> = pie
        .link_candidates
        .iter()
        .map(|c| c.measured(ink()))
        .collect();
    pie.place_link_labels(
        &measured,
        &PieLinkPlacementOptions {
            collision: LabelCollisionOptions {
                viewport: LayoutRect {
                    x: 0.,
                    y: 0.,
                    width: 800.,
                    height: 440.,
                },
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
}

#[test]
fn legal_unicode_family_names_do_not_expand_the_measurement_budget() {
    let config = ChartConfig {
        theme: Theme {
            font: FontOptions {
                family: Some(vec![FontFamily::Named("\u{ad}".repeat(128)); 8]),
                weight: Some(700.),
            },
            ..Default::default()
        },
        ..Default::default()
    };
    let root = "r".repeat(1000);
    let rows = [
        HierarchyDatum::new(&root, None, 0.),
        HierarchyDatum::new("n".repeat(1000), Some(root), 1.),
    ];
    let layout = HierarchyFocusController::new(&rows)
        .unwrap()
        .layout(&config, &HierarchyView::Icicle(Default::default()))
        .unwrap();
    let measured: Vec<_> = layout
        .label_candidates()
        .iter()
        .map(|c| c.measured(ink()))
        .collect();
    assert!(!measured.is_empty());
    assert!(measured.iter().all(|m| m.signature.len() <= 8192));
    layout
        .place_labels(
            &measured,
            &HierarchyLabelPlacementOptions::for_layout(&layout),
        )
        .unwrap();
}
