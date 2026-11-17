use std::collections::BTreeMap;

use kartis::*;

fn label(text: &str) -> Label {
    Label {
        x: 30.,
        y: 40.,
        text: text.into(),
        color: Color(0, 0, 0, 255),
        size: 10.,
        rotation: 0.,
        anchor: TextAnchor::Start,
    }
}

#[test]
fn normalization_is_explicit_single_line_and_unicode_safe() {
    let text = "  a\t\r\n\u{b}\u{c}\u{85}\u{2028}\u{2029}b  \u{a0}\u{202f}\u{200d}";
    assert_eq!(LabelWhitespace::Legacy.normalize(text), text);
    assert_eq!(
        LabelWhitespace::Preserve.normalize(text),
        "  a        b  \u{a0}\u{202f}\u{200d}"
    );
    let source = label(text);
    let drawable = LabelWhitespace::Preserve.drawable_label(&source);
    assert_eq!(drawable.text, "  a        b  \u{a0}\u{202f}\u{200d}");
    assert_eq!(source.text, text);
    assert_eq!(drawable.x, source.x);
}

#[test]
fn sparse_modes_validate_remap_and_keep_legacy_export() {
    let mut scene = ChartConfig::default().scene().unwrap();
    scene.labels = vec![label(" a  b "), label("second")];
    let legacy = render::to_svg(&scene).unwrap();
    scene
        .typography
        .label_whitespace
        .insert(0, LabelWhitespace::Legacy);
    assert_eq!(render::to_svg(&scene).unwrap(), legacy);
    scene
        .typography
        .label_whitespace
        .insert(1, LabelWhitespace::Preserve);
    let mut typography = scene.typography.clone();
    typography.remap_labels(&BTreeMap::from([(1, 0)]));
    assert_eq!(
        typography.label_whitespace,
        BTreeMap::from([(0, LabelWhitespace::Preserve)])
    );
    scene
        .typography
        .label_whitespace
        .insert(2, LabelWhitespace::Preserve);
    assert!(scene.validate().is_err());
}

#[test]
fn wrapping_sets_preserve_and_remaps_existing_modes_transactionally() {
    let mut scene = ChartConfig::default().scene().unwrap();
    scene.labels = vec![label("a b"), label("tail")];
    scene
        .typography
        .label_whitespace
        .insert(1, LabelWhitespace::Legacy);
    let mut measured = |r: &SemanticTextRequest| {
        assert_eq!(r.whitespace, LabelWhitespace::Preserve);
        r.measured(SemanticTextMetrics {
            advance: r.text.len() as f64 * 10.,
            line_height: r.line_height,
            baseline_from_top: 8.,
            ink_bounds: None,
        })
    };
    let result = wrap_scene_label(
        &mut scene,
        0,
        &TextWrapOptions {
            max_width: 15.,
            ..Default::default()
        },
        &mut measured,
    )
    .unwrap();
    assert_eq!(result.labels.len(), 2);
    assert_eq!(
        scene.typography.label_whitespace,
        BTreeMap::from([
            (0, LabelWhitespace::Preserve),
            (1, LabelWhitespace::Preserve),
            (2, LabelWhitespace::Legacy)
        ])
    );
    let before = scene.clone();
    assert!(
        wrap_scene_label(
            &mut scene,
            0,
            &TextWrapOptions {
                max_width: f64::NAN,
                ..Default::default()
            },
            &mut measured
        )
        .is_err()
    );
    assert_eq!(scene, before);
}

#[test]
fn managed_candidates_bind_whitespace_and_reject_stale_measurements() {
    let data = [
        HierarchyDatum::new("root", None, 0.),
        HierarchyDatum::new("a", Some("root".into()), 4.),
        HierarchyDatum::new("b", Some("root".into()), 3.),
    ];
    let raw = HierarchyFocusController::new(&data)
        .unwrap()
        .layout(
            &ChartConfig::default(),
            &HierarchyView::Treemap(TreemapLayoutOptions::default()),
        )
        .unwrap();
    assert!(!raw.label_candidates().is_empty());
    let old: Vec<_> = raw
        .label_candidates()
        .iter()
        .map(|c| {
            c.measured(LayoutRect {
                x: 0.,
                y: 0.,
                width: 2.,
                height: 2.,
            })
        })
        .collect();
    let preserve = raw
        .with_label_whitespace(LabelWhitespace::Preserve)
        .unwrap();
    assert!(
        preserve
            .label_candidates()
            .iter()
            .all(|c| c.whitespace == LabelWhitespace::Preserve)
    );
    assert!(
        preserve
            .place_labels(&old, &HierarchyLabelPlacementOptions::for_layout(&preserve))
            .is_err()
    );
    let mut hidden_options = HierarchyLabelPlacementOptions::for_layout(&preserve);
    hidden_options.hidden = preserve
        .label_candidates()
        .iter()
        .map(|c| c.node_id.clone())
        .collect();
    let current: Vec<_> = preserve
        .label_candidates()
        .iter()
        .map(|c| {
            c.measured(LayoutRect {
                x: -1.,
                y: -1.,
                width: 2.,
                height: 2.,
            })
        })
        .collect();
    let hidden = preserve.place_labels(&current, &hidden_options).unwrap();
    assert!(hidden.scene.typography.label_whitespace.is_empty());
    let restored = hidden
        .place_labels(
            &current,
            &HierarchyLabelPlacementOptions::for_layout(&hidden),
        )
        .unwrap();
    assert!(!restored.scene.labels.is_empty());
    assert!(
        restored
            .scene
            .typography
            .label_whitespace
            .values()
            .all(|m| *m == LabelWhitespace::Preserve)
    );
}

#[test]
fn sunburst_modes_survive_omission_and_restore() {
    let data = [
        HierarchyDatum::new("root", None, 0.),
        HierarchyDatum::new("a", Some("root".into()), 4.),
        HierarchyDatum::new("b", Some("root".into()), 3.),
    ];
    let raw = sunburst_layout(
        &ChartConfig::default(),
        &data,
        &SunburstLayoutOptions {
            include_root: true,
            labels: Some(Default::default()),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(!raw.label_candidates().is_empty());
    let old: Vec<_> = raw
        .label_candidates()
        .iter()
        .map(|c| {
            c.measured(LayoutRect {
                x: -1.,
                y: -1.,
                width: 2.,
                height: 2.,
            })
        })
        .collect();
    let preserve = raw
        .with_label_whitespace(LabelWhitespace::Preserve)
        .unwrap();
    assert!(
        preserve
            .label_candidates()
            .iter()
            .all(|c| c.whitespace == LabelWhitespace::Preserve)
    );
    assert!(
        preserve
            .place_labels(&old, &SunburstLabelPlacementOptions::for_layout(&preserve))
            .is_err()
    );
    let current: Vec<_> = preserve
        .label_candidates()
        .iter()
        .map(|c| {
            c.measured(LayoutRect {
                x: -1.,
                y: -1.,
                width: 2.,
                height: 2.,
            })
        })
        .collect();
    let mut omit = SunburstLabelPlacementOptions::for_layout(&preserve);
    omit.hidden = preserve
        .label_candidates()
        .iter()
        .map(|c| c.node_id.clone())
        .collect();
    let hidden = preserve.place_labels(&current, &omit).unwrap();
    assert!(hidden.scene.typography.label_whitespace.is_empty());
    let restored = hidden
        .place_labels(
            &current,
            &SunburstLabelPlacementOptions::for_layout(&hidden),
        )
        .unwrap();
    assert!(!restored.scene.labels.is_empty());
    assert!(
        restored
            .scene
            .typography
            .label_whitespace
            .values()
            .all(|m| *m == LabelWhitespace::Preserve)
    );
}

#[test]
fn area_bump_candidates_bind_mode_and_reject_external_mode_mutation() {
    let data = [AreaBumpSeries::new(
        "s",
        vec![AreaBumpDatum::new("a", 3.), AreaBumpDatum::new("b", 4.)],
    )];
    let raw =
        layout_area_bump(&ChartConfig::default(), &data, &AreaBumpOptions::default()).unwrap();
    assert!(!raw.endpoint_label_candidates().is_empty());
    let old: Vec<_> = raw
        .endpoint_label_candidates()
        .iter()
        .map(|c| {
            c.measured(LayoutRect {
                x: -1.,
                y: -1.,
                width: 2.,
                height: 2.,
            })
        })
        .collect();
    let preserve = raw
        .with_label_whitespace(LabelWhitespace::Preserve)
        .unwrap();
    assert!(
        preserve
            .endpoint_label_candidates()
            .iter()
            .all(|c| c.whitespace == LabelWhitespace::Preserve)
    );
    let options = AreaBumpLabelPlacementOptions::default();
    assert!(preserve.place_endpoint_labels(&old, &options).is_err());
    let current: Vec<_> = preserve
        .endpoint_label_candidates()
        .iter()
        .map(|c| {
            c.measured(LayoutRect {
                x: -1.,
                y: -1.,
                width: 2.,
                height: 2.,
            })
        })
        .collect();
    let mut tampered = preserve.clone();
    let slot = *tampered
        .scene
        .typography
        .label_whitespace
        .keys()
        .next()
        .unwrap();
    tampered
        .scene
        .typography
        .label_whitespace
        .insert(slot, LabelWhitespace::Legacy);
    assert!(tampered.place_endpoint_labels(&current, &options).is_err());
}
