use glory::render::GraphicStroke;
use kartis::{surface::SurfaceModel, *};
use std::collections::BTreeMap;

fn source() -> (Scene, InteractionBinding) {
    let mut scene = ChartConfig::default().scene().unwrap();
    let mut body = Mark::new(
        "body",
        Geometry::Rect {
            x: 40.,
            y: 40.,
            width: 40.,
            height: 60.,
        },
        Color(20, 40, 60, 255),
    );
    body.series = "x".into();
    body.label = "A · x".into();
    body.value = 5.;
    let mut part = body.clone();
    part.id = "part".into();
    part.label.clear();
    part.geometry = Geometry::Rect {
        x: 55.,
        y: 20.,
        width: 10.,
        height: 10.,
    };
    scene.marks = vec![body.clone(), part];
    let binding = InteractionBinding {
        sites: vec![InteractionSite {
            mark_id: body.id.clone(),
            series: body.series.clone(),
            label: body.label.clone(),
            value: Some(5.),
            formatted_value: None,
            color: body.fill.unwrap(),
            position: [60., 70.],
        }],
        ink_owners: BTreeMap::from([("part".into(), "body".into())]),
        picking_profile: PickingProfile::PaintedGeometry,
        ..Default::default()
    };
    (scene, binding)
}
#[test]
fn one_hop_owner_accepts_real_blank_label_ink_without_adding_sites() {
    let (scene, binding) = source();
    binding.validate(&scene).unwrap();
    assert_eq!(binding.sites.len(), 1);
    assert_eq!(binding.anchors.len(), 0);
    let cloned = binding.clone();
    assert_eq!(cloned, binding);
    assert!(format!("{binding:#?}").contains("ink_owners"));
}
#[test]
fn rejects_unknown_self_site_key_alias_target_and_cycles() {
    let (mut scene, binding) = source();
    let mut other = scene.marks[1].clone();
    other.id = "other-part".into();
    scene.marks.push(other);
    for map in [
        BTreeMap::from([("missing".into(), "body".into())]),
        BTreeMap::from([("part".into(), "missing".into())]),
        BTreeMap::from([("part".into(), "part".into())]),
        BTreeMap::from([("body".into(), "body".into())]),
        BTreeMap::from([
            ("part".into(), "other-part".into()),
            ("other-part".into(), "part".into()),
        ]),
    ] {
        let candidate = InteractionBinding {
            ink_owners: map,
            ..binding.clone()
        };
        assert!(candidate.validate(&scene).is_err());
    }
}
#[test]
fn rejects_cross_series_and_identity_byte_overflow() {
    let (mut scene, binding) = source();
    scene.marks[1].series = "foreign".into();
    assert!(binding.validate(&scene).is_err());
    let (scene, mut binding) = source();
    binding.ink_owners = BTreeMap::from([("a".repeat(MAX_LABEL_BYTES + 1), "body".into())]);
    assert!(binding.validate(&scene).is_err());
}
#[test]
fn live_frame_removal_or_series_change_rejects_atomically_even_without_anchors() {
    let (scene, binding) = source();
    let mut model = SurfaceModel::new_with_interaction(
        scene.clone(),
        Default::default(),
        Some(binding.clone()),
    )
    .unwrap();
    let before = model.scene().clone();
    let mut missing = scene.clone();
    missing.marks.pop();
    assert!(
        model
            .set_interaction_frame(&scene, Some(binding.clone()), missing)
            .is_err()
    );
    assert_eq!(model.scene(), &before);
    assert_eq!(model.interaction_data(), Some(&binding));
    let mut foreign = scene.clone();
    foreign.marks[1].series = "foreign".into();
    assert!(
        model
            .set_interaction_frame(&scene, Some(binding.clone()), foreign)
            .is_err()
    );
    assert_eq!(model.scene(), &before);
    assert_eq!(model.interaction_data(), Some(&binding));
}
#[test]
fn invisible_and_clipped_parts_keep_structural_metadata() {
    let (mut scene, binding) = source();
    scene.marks[1].opacity = 0.;
    scene.marks[1].clip = Some(ClipRect {
        x: 0.,
        y: 0.,
        width: 0.,
        height: 0.,
    });
    binding.validate(&scene).unwrap();
    let model =
        SurfaceModel::new_with_interaction(scene, Default::default(), Some(binding.clone()))
            .unwrap();
    assert_eq!(
        model.interaction_data().unwrap().ink_owners,
        binding.ink_owners
    );
    // This test makes no claim that Surface picking resolves aliases yet.
}
#[test]
fn empty_alias_debug_is_byte_identical_to_original_derived_field_list() {
    #[derive(Debug)]
    #[allow(dead_code)]
    struct Legacy<'a> {
        sites: &'a Vec<InteractionSite>,
        anchors: &'a BTreeMap<String, InteractionAnchor>,
        polar: &'a Option<kartis::PolarSliceIndex>,
        polar_spoke: &'a Option<GraphicStroke>,
    }
    let (_, mut binding) = source();
    binding.ink_owners.clear();
    binding.picking_profile = PickingProfile::LegacyGeometry;
    let old = Legacy {
        sites: &binding.sites,
        anchors: &binding.anchors,
        polar: &binding.polar,
        polar_spoke: &binding.polar_spoke,
    };
    assert_eq!(
        format!("{binding:?}"),
        format!("{old:?}").replacen("Legacy", "InteractionBinding", 1)
    );
    assert_eq!(
        format!("{binding:#?}"),
        format!("{old:#?}").replacen("Legacy", "InteractionBinding", 1)
    );
}
fn controller() -> (
    ChartConfig,
    Vec<BoxObservation>,
    BoxPlotLayoutOptions,
    BoxPlotInteraction,
) {
    let config = ChartConfig {
        width: 480.,
        height: 300.,
        ..Default::default()
    };
    let data = [0., 2., 4., 6., 8., 10.]
        .into_iter()
        .enumerate()
        .map(|(i, v)| BoxObservation::new(format!("s{i}"), "A", "x", Some(v)))
        .collect::<Vec<_>>();
    let options = BoxPlotLayoutOptions {
        domain: Some([-2., 12.]),
        ..Default::default()
    };
    let controller =
        BoxPlotInteraction::new(&config, &data, &options, &Default::default()).unwrap();
    (config, data, options, controller)
}
#[test]
fn controller_map_covers_exact_public_source_owners_and_restores_after_hover() {
    let (_, _, _, mut controller) = controller();
    let original = controller.whole_parts_interaction_binding().unwrap();
    assert_eq!(original.sites.len(), controller.layout.boxes.len());
    let body = &controller.layout.boxes[0].id;
    let mut owned_parts = 0;
    for mark in &controller.layout.scene.marks {
        if let Some(key) = controller.key_for_mark(&mark.id).unwrap() {
            let owner = &controller.box_for_key(key).unwrap().unwrap().id;
            if &mark.id == owner {
                assert!(!original.ink_owners.contains_key(&mark.id));
            } else {
                assert_eq!(original.ink_owners.get(&mark.id), Some(owner));
                owned_parts += 1;
            }
        } else {
            assert!(!original.ink_owners.contains_key(&mark.id));
        }
    }
    assert!(owned_parts >= 5);
    assert_eq!(original.ink_owners.len(), owned_parts);
    let key = controller.key_for_mark(body).unwrap().unwrap().clone();
    controller.set_active(Some(key)).unwrap();
    assert_eq!(
        controller
            .whole_parts_interaction_binding()
            .unwrap()
            .ink_owners,
        original.ink_owners
    );
    controller.set_active(None).unwrap();
    assert_eq!(
        controller.whole_parts_interaction_binding().unwrap(),
        original
    );
}
#[test]
fn controller_replace_source_rebuilds_aliases_and_private_guard_rejects_tampering() {
    let (config, _, options, mut controller) = controller();
    let original = controller.whole_parts_interaction_binding().unwrap();
    let data = [2., 3., 4.]
        .into_iter()
        .enumerate()
        .map(|(i, v)| BoxObservation::new(format!("new{i}"), "B", "y", Some(v)))
        .collect::<Vec<_>>();
    controller.replace_source(&config, &data, &options).unwrap();
    let binding = controller.whole_parts_interaction_binding().unwrap();
    binding.validate(&controller.layout.scene).unwrap();
    assert!(
        binding
            .ink_owners
            .keys()
            .all(|id| !original.ink_owners.contains_key(id))
    );
    let index = controller
        .layout
        .scene
        .marks
        .iter()
        .position(|m| binding.ink_owners.contains_key(&m.id))
        .unwrap();
    controller.layout.scene.marks[index].series = "foreign".into();
    assert!(controller.whole_parts_interaction_binding().is_err());
}

#[test]
fn aliases_share_existing_count_and_cumulative_identity_byte_budgets() {
    let (mut scene, mut binding) = source();
    binding.ink_owners = (0..MAX_MARKS)
        .map(|i| (format!("part-{i}"), "body".into()))
        .collect();
    assert!(
        binding
            .validate(&scene)
            .unwrap_err()
            .0
            .contains("count budget")
    );
    let owner = "b".repeat(MAX_LABEL_BYTES);
    scene.marks[0].id = owner.clone();
    binding.sites[0].mark_id = owner.clone();
    let prototype = scene.marks[1].clone();
    scene.marks.truncate(1);
    binding.ink_owners.clear();
    for i in 0..2_000 {
        let prefix = format!("part-{i}:");
        let id = format!("{prefix}{}", "p".repeat(MAX_LABEL_BYTES - prefix.len()));
        let mut mark = prototype.clone();
        mark.id = id.clone();
        scene.marks.push(mark);
        binding.ink_owners.insert(id, owner.clone());
    }
    scene.validate().unwrap();
    assert!(
        binding
            .validate(&scene)
            .unwrap_err()
            .0
            .contains("byte budget")
    );
}

#[test]
fn profile_is_explicit_and_legacy_controller_entry_stays_body_only() {
    let (_, _, _, controller) = controller();
    let legacy = controller.interaction_binding().unwrap();
    assert_eq!(legacy.picking_profile, PickingProfile::LegacyGeometry);
    assert!(legacy.ink_owners.is_empty());
    assert!(!format!("{legacy:?}").contains("picking_profile"));
    assert!(!format!("{legacy:?}").contains("ink_owners"));
    let legacy_frame = controller.scene_interaction().unwrap();
    let painted_frame = controller.whole_parts_scene_interaction().unwrap();
    assert_eq!(legacy_frame.scene, painted_frame.scene);
    assert_eq!(legacy_frame.plot, painted_frame.plot);
    let painted = painted_frame.interaction.unwrap();
    assert_eq!(legacy.sites, painted.sites);
    assert_eq!(legacy.anchors, painted.anchors);
    assert_eq!(painted.picking_profile, PickingProfile::PaintedGeometry);
    assert!(!painted.ink_owners.is_empty());
    let mut accidental = painted;
    accidental.picking_profile = PickingProfile::LegacyGeometry;
    assert!(accidental.validate(&legacy_frame.scene).is_err());
}
#[test]
fn strict_profile_survives_empty_alias_map_for_body_only_or_empty_source() {
    let (config, data, mut options, _) = controller();
    options.border = None;
    options.median.width = 0.;
    options.whisker.width = 0.;
    options.outlier_radius = 0.;
    for source in [data.as_slice(), &[]] {
        let c = BoxPlotInteraction::new(&config, source, &options, &Default::default()).unwrap();
        let binding = c.whole_parts_interaction_binding().unwrap();
        assert!(binding.ink_owners.is_empty());
        assert_eq!(binding.picking_profile, PickingProfile::PaintedGeometry);
        assert!(format!("{binding:?}").contains("picking_profile: PaintedGeometry"));
        binding.validate(&c.layout.scene).unwrap();
    }
}

#[test]
fn physical_source_sidecar_is_bounded_strict_and_independent_of_live_scalar() {
    let (mut scene, mut binding) = source();
    binding.part_data.insert(
        "part".into(),
        InteractionPartDatum {
            source_id: "original/λ".into(),
            value: f64::MAX,
        },
    );
    scene.marks[1].value = -123.;
    binding.validate(&scene).unwrap();
    let mut invalid = binding.clone();
    invalid.picking_profile = PickingProfile::LegacyGeometry;
    assert!(invalid.validate(&scene).is_err());
    for datum in [
        InteractionPartDatum {
            source_id: String::new(),
            value: 1.,
        },
        InteractionPartDatum {
            source_id: "valid".into(),
            value: f64::NAN,
        },
        InteractionPartDatum {
            source_id: "valid".into(),
            value: f64::INFINITY,
        },
        InteractionPartDatum {
            source_id: "λ".repeat(MAX_LABEL_BYTES),
            value: 1.,
        },
    ] {
        let mut invalid = binding.clone();
        invalid.part_data.insert("part".into(), datum);
        assert!(invalid.validate(&scene).is_err());
    }
    let mut invalid = binding.clone();
    invalid.part_data.insert(
        "unknown".into(),
        InteractionPartDatum {
            source_id: "valid".into(),
            value: 1.,
        },
    );
    assert!(invalid.validate(&scene).is_err());
    binding.ink_owners.clear();
    binding.part_data.clear();
    binding.part_data.insert(
        "body".into(),
        InteractionPartDatum {
            source_id: "original-body".into(),
            value: -9.,
        },
    );
    binding.validate(&scene).unwrap();
    let mut model = SurfaceModel::new_with_interaction(
        scene.clone(),
        Default::default(),
        Some(binding.clone()),
    )
    .unwrap();
    let mut foreign = scene.clone();
    foreign.marks[0].series = "foreign".into();
    assert!(
        model
            .set_interaction_frame(&scene, Some(binding), foreign)
            .is_err()
    );
    assert_eq!(model.scene(), &scene);
}
#[test]
fn box_sidecar_contains_only_true_original_outliers_not_median_or_decorations() {
    let (_, _, _, controller) = controller();
    let binding = controller.whole_parts_interaction_binding().unwrap();
    assert_eq!(
        binding.part_data.len(),
        controller
            .layout
            .boxes
            .iter()
            .map(|b| b.outliers.len())
            .sum::<usize>()
    );
    for mark in &controller.layout.scene.marks {
        let original = controller.outlier_for_mark(&mark.id).unwrap();
        match original {
            Some(source) => assert_eq!(
                binding.part_data.get(&mark.id),
                Some(&InteractionPartDatum {
                    source_id: source.source_id.clone(),
                    value: source.value
                })
            ),
            None => assert!(!binding.part_data.contains_key(&mark.id)),
        }
    }
    assert!(
        controller
            .interaction_binding()
            .unwrap()
            .part_data
            .is_empty()
    );
}
