use std::collections::BTreeMap;

use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 360.,
        height: 220.,
        margin: Margin {
            left: 12.,
            right: 12.,
            top: 12.,
            bottom: 12.,
        },
        ..Default::default()
    }
}

fn data() -> Vec<HierarchyDatum> {
    vec![
        HierarchyDatum::new("root", None, 0.),
        HierarchyDatum::new("group", Some("root".into()), 0.),
        HierarchyDatum::new("a", Some("group".into()), 1.),
        HierarchyDatum::new("b", Some("group".into()), 1.),
    ]
}

fn layout(view: &HierarchyView) -> HierarchyLayout {
    HierarchyFocusController::new(&data())
        .unwrap()
        .layout(&config(), view)
        .unwrap()
}

fn measure(layout: &HierarchyLayout) -> Vec<HierarchyMeasuredLabel> {
    layout
        .label_candidates()
        .iter()
        .map(|candidate| {
            // Measured bearings relative to the baseline, rotated around the
            // authored anchor. The AABB is deliberately asymmetric vertically.
            let angle = candidate.label.rotation.to_radians();
            let points = [(-15., -9.), (17., -9.), (-15., 3.), (17., 3.)].map(|(x, y)| {
                (
                    x * angle.cos() - y * angle.sin(),
                    x * angle.sin() + y * angle.cos(),
                )
            });
            let left = points.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
            let top = points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
            candidate.measured(LayoutRect {
                x: left,
                y: top,
                width: points.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max) - left,
                height: points.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max) - top,
            })
        })
        .collect()
}

fn placed<'a>(layout: &'a HierarchyLayout, node: &str) -> &'a HierarchyPlacedLabel {
    layout
        .label_placements()
        .iter()
        .find(|placed| placed.node_id == node)
        .unwrap()
}

fn assert_clip(clip: &ClipPath, rect: LayoutRect) {
    match clip.geometry {
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => assert!(
            LayoutRect {
                x,
                y,
                width,
                height
            }
            .contains(rect)
        ),
        Geometry::Circle { x, y, radius } => {
            for (cx, cy) in [
                (rect.x, rect.y),
                (rect.x + rect.width, rect.y),
                (rect.x, rect.y + rect.height),
                (rect.x + rect.width, rect.y + rect.height),
            ] {
                assert!((cx - x).hypot(cy - y) <= radius);
            }
        }
        _ => panic!("expected an authored hierarchy clip"),
    }
}

#[test]
fn measured_rotated_ink_preserves_node_identity_clips_and_interaction_in_every_view() {
    let mut style = HierarchyStyleOptions::default();
    style.parent_labels.enabled = true;
    style.parent_labels.rotation = 30.;
    style.leaf_labels.rotation = -25.;
    style.hidden.insert("b".into());
    let views = [
        HierarchyView::Treemap(TreemapLayoutOptions {
            style: style.clone(),
            ..Default::default()
        }),
        HierarchyView::CirclePacking(CirclePackingLayoutOptions {
            style: style.clone(),
            ..Default::default()
        }),
        HierarchyView::Icicle(IcicleLayoutOptions {
            style,
            ..Default::default()
        }),
    ];
    for view in views {
        let raw = layout(&view);
        assert!(raw.label_placements().is_empty());
        assert!(raw.label_candidates().iter().all(|c| c.node_id != "b"));
        let measurements = measure(&raw);
        let options = HierarchyLabelPlacementOptions::for_layout(&raw);
        let result = raw.place_labels(&measurements, &options).unwrap();
        assert_eq!(raw.scene.marks, result.scene.marks);
        assert_eq!(
            raw.interaction_binding().unwrap(),
            result.interaction_binding().unwrap()
        );
        assert_eq!(raw.focus_id, result.focus_id);
        let mut occupied = Vec::new();
        for (candidate, measurement) in raw.label_candidates().iter().zip(&measurements) {
            let placement = placed(&result, &candidate.node_id);
            assert_eq!(placement.id, candidate.id);
            if let Some(bounds) = placement.bounds {
                assert!(options.collision.viewport.contains(bounds));
                assert!(occupied.iter().all(|old| bounds.overlap(*old) == 0.));
                occupied.push(bounds);
                let index = placement.label_index.unwrap();
                let label = &result.scene.labels[index];
                assert_eq!(label.text, candidate.label.text);
                assert_eq!(label.anchor, candidate.label.anchor);
                assert_eq!(label.rotation, candidate.label.rotation);
                assert!((label.x + measurement.bounds.x - bounds.x).abs() < 1e-10);
                assert!((label.y + measurement.bounds.y - bounds.y).abs() < 1e-10);
                assert_eq!(result.scene.label_series[&index], candidate.node_id);
                assert_eq!(
                    result.node(&candidate.node_id).unwrap().label_slots,
                    [index]
                );
                assert_eq!(
                    result.scene.label_clips.get(&index),
                    candidate.clip.as_ref()
                );
                assert_clip(candidate.clip.as_ref().unwrap(), bounds);
            } else {
                assert!(
                    result
                        .node(&candidate.node_id)
                        .unwrap()
                        .label_slots
                        .is_empty()
                );
            }
        }
        assert!(!occupied.is_empty());
        assert_eq!(
            result,
            result.place_labels(&measurements, &options).unwrap()
        );
        assert!(render::to_svg(&result.scene).is_ok());
        assert!(render::to_canvas_script(&result.scene, "hierarchy").is_ok());
    }
}

#[test]
fn priority_changes_restore_omitted_labels_and_try_ordered_displacements() {
    let mut packing = CirclePackingLayoutOptions::default();
    packing.style.parent_labels.enabled = true;
    let raw = layout(&HierarchyView::CirclePacking(packing));
    let measurements = measure(&raw);
    let mut options = HierarchyLabelPlacementOptions::for_layout(&raw);
    options.priorities.insert("root".into(), 100);
    let first = raw.place_labels(&measurements, &options).unwrap();
    assert!(placed(&first, "root").bounds.is_some());
    assert!(placed(&first, "group").bounds.is_none());
    options.priorities.insert("group".into(), 200);
    let second = first.place_labels(&measurements, &options).unwrap();
    assert!(placed(&second, "group").bounds.is_some());
    assert!(placed(&second, "root").bounds.is_none());
    options.offsets = vec![(0., 24.), (0., -24.)];
    let moved = second.place_labels(&measurements, &options).unwrap();
    let root = placed(&moved, "root");
    let label = &moved.scene.labels[root.label_index.unwrap()];
    let source = raw
        .label_candidates()
        .iter()
        .find(|c| c.node_id == "root")
        .unwrap();
    assert_eq!(label.x, source.label.x);
    assert_eq!(label.y, source.label.y + 24.);
    assert_eq!(
        root.bounds
            .unwrap()
            .overlap(placed(&moved, "group").bounds.unwrap()),
        0.
    );

    let reversed: Vec<_> = data().into_iter().rev().collect();
    let mut controller = HierarchyFocusController::new(&reversed).unwrap();
    let reordered = controller
        .layout(
            &config(),
            &HierarchyView::CirclePacking({
                let mut options = CirclePackingLayoutOptions::default();
                options.style.parent_labels.enabled = true;
                options
            }),
        )
        .unwrap();
    let by_id = |layout: &HierarchyLayout| {
        layout
            .label_candidates()
            .iter()
            .map(|c| (c.node_id.clone(), c.id.clone()))
            .collect::<BTreeMap<_, _>>()
    };
    assert_eq!(by_id(&raw), by_id(&reordered));
}

#[test]
fn circle_clip_rejects_bbox_corners_outside_the_disk() {
    let mut packing = CirclePackingLayoutOptions::default();
    packing.style.parent_labels.enabled = true;
    let raw = layout(&HierarchyView::CirclePacking(packing));
    let mut measurements = measure(&raw);
    let candidate = raw
        .label_candidates()
        .iter()
        .find(|c| c.node_id == "root")
        .unwrap();
    let Geometry::Circle { x, y, radius } = candidate.clip.as_ref().unwrap().geometry else {
        panic!()
    };
    let size = radius * 1.5;
    let bounds = LayoutRect {
        x: x - size / 2.,
        y: y - size / 2.,
        width: size,
        height: size,
    };
    let measured = measurements
        .iter()
        .position(|m| m.id == candidate.id)
        .unwrap();
    measurements[measured].bounds = LayoutRect {
        x: bounds.x - candidate.label.x,
        y: bounds.y - candidate.label.y,
        ..bounds
    };
    let mut options = HierarchyLabelPlacementOptions::for_layout(&raw);
    options.hidden = ["a".into(), "b".into(), "group".into()]
        .into_iter()
        .collect();
    let rejected = raw.place_labels(&measurements, &options).unwrap();
    assert!(placed(&rejected, "root").bounds.is_none());
    measurements[measured].bounds = LayoutRect {
        x: x - radius / 2. - candidate.label.x,
        y: y - radius / 2. - candidate.label.y,
        width: radius,
        height: radius,
    };
    let restored = rejected.place_labels(&measurements, &options).unwrap();
    assert!(placed(&restored, "root").bounds.is_some());
}

#[test]
fn invalid_original_position_can_move_into_a_rotated_parent_header_clip() {
    let mut treemap = TreemapLayoutOptions::default();
    treemap.style.parent_labels.rotation = 30.;
    let raw = layout(&HierarchyView::Treemap(treemap));
    let mut measurements = measure(&raw);
    let candidate = raw
        .label_candidates()
        .iter()
        .find(|c| c.node_id == "root")
        .unwrap();
    measurements
        .iter_mut()
        .find(|m| m.id == candidate.id)
        .unwrap()
        .bounds = LayoutRect {
        x: -10.,
        y: -10.,
        width: 32.,
        height: 14.,
    };
    let mut options = HierarchyLabelPlacementOptions::for_layout(&raw);
    options.hidden = ["a".into(), "b".into(), "group".into()]
        .into_iter()
        .collect();
    assert!(
        placed(&raw.place_labels(&measurements, &options).unwrap(), "root")
            .bounds
            .is_none()
    );
    options.offsets = vec![(10., 0.)];
    let result = raw.place_labels(&measurements, &options).unwrap();
    let placement = placed(&result, "root");
    let label = &result.scene.labels[placement.label_index.unwrap()];
    assert_eq!(label.x, candidate.label.x + 10.);
    assert_eq!(label.y, candidate.label.y);
    assert_eq!(label.rotation, 30.);
    assert_clip(candidate.clip.as_ref().unwrap(), placement.bounds.unwrap());
}

#[test]
fn authored_plot_clip_and_obstacles_filter_offsets_before_placement() {
    let mut treemap = TreemapLayoutOptions::default();
    treemap.style.parent_labels.enabled = false;
    treemap.style.leaf_labels.clip_to_node = false;
    let raw = layout(&HierarchyView::Treemap(treemap));
    let mut measurements = measure(&raw);
    let candidate = raw
        .label_candidates()
        .iter()
        .find(|c| c.node_id == "a")
        .unwrap();
    let plot = raw.plot();
    let original = LayoutRect {
        x: plot.x - 5.,
        y: plot.y + 10.,
        width: 20.,
        height: 12.,
    };
    measurements
        .iter_mut()
        .find(|m| m.id == candidate.id)
        .unwrap()
        .bounds = LayoutRect {
        x: original.x - candidate.label.x,
        y: original.y - candidate.label.y,
        ..original
    };
    let mut options = HierarchyLabelPlacementOptions::for_layout(&raw);
    options.collision.viewport = LayoutRect {
        x: 0.,
        y: 0.,
        width: config().width,
        height: config().height,
    };
    options.hidden.insert("b".into());
    assert!(
        placed(&raw.place_labels(&measurements, &options).unwrap(), "a")
            .bounds
            .is_none()
    );
    options.offsets = vec![(10., 0.), (50., 0.)];
    options.collision.forbidden.push(LayoutRect {
        x: original.x + 10.,
        ..original
    });
    let result = raw.place_labels(&measurements, &options).unwrap();
    let bounds = placed(&result, "a").bounds.unwrap();
    assert_eq!(bounds.x, original.x + 50.);
    assert_eq!(bounds.overlap(options.collision.forbidden[0]), 0.);
    assert_clip(candidate.clip.as_ref().unwrap(), bounds);
}

#[test]
fn hidden_labels_preserve_custom_ink_owned_clips_and_explicit_order() {
    let mut raw = layout(&HierarchyView::Icicle(Default::default()));
    let custom = Label {
        x: 21.,
        y: 18.,
        text: "Custom annotation".into(),
        size: 12.,
        color: Color(30, 40, 50, 255),
        anchor: TextAnchor::Start,
        rotation: 0.,
    };
    let clip = ClipPath {
        geometry: Geometry::Rect {
            x: 12.,
            y: 12.,
            width: 200.,
            height: 30.,
        },
        rule: ClipRule::NonZero,
    };
    let slot = raw.scene.labels.len();
    raw.scene.labels.push(custom.clone());
    raw.scene.label_series.insert(slot, "annotation".into());
    raw.scene.label_clips.insert(slot, clip.clone());
    raw.scene.draw_order = raw.scene.draw_items();
    let item = raw.scene.draw_order.pop().unwrap();
    raw.scene.draw_order.insert(0, item);
    raw.scene.validate().unwrap();
    let measurements = measure(&raw);
    let mut options = HierarchyLabelPlacementOptions::for_layout(&raw);
    options.hidden = raw
        .label_candidates()
        .iter()
        .map(|c| c.node_id.clone())
        .collect();
    let result = raw.place_labels(&measurements, &options).unwrap();
    assert_eq!(
        result.scene.labels.as_slice(),
        std::slice::from_ref(&custom)
    );
    assert_eq!(result.scene.label_series[&0], "annotation");
    assert_eq!(result.scene.label_clips[&0], clip);
    assert_eq!(result.scene.draw_order[0], DrawItem::Label(0));
    assert_eq!(result.scene.marks, raw.scene.marks);
    assert!(result.nodes.iter().all(|node| node.label_slots.is_empty()));
    options.hidden.clear();
    let restored = result.place_labels(&measurements, &options).unwrap();
    assert_eq!(restored.scene.labels[0], custom);
    assert_eq!(restored.scene.label_clips[&0], clip);
    assert_eq!(restored.scene.draw_order[0], DrawItem::Label(0));
    assert!(
        restored
            .label_placements()
            .iter()
            .any(|p| p.label_index.is_some())
    );
    restored.scene.validate().unwrap();
}

#[test]
fn stale_measurements_metadata_edits_and_budget_failures_leave_the_layout_intact() {
    let raw = layout(&HierarchyView::Icicle(Default::default()));
    let before = raw.clone();
    let measurements = measure(&raw);
    let options = HierarchyLabelPlacementOptions::for_layout(&raw);
    let mut duplicate = measurements.clone();
    duplicate[0] = duplicate[1].clone();
    assert!(raw.place_labels(&duplicate, &options).is_err());
    assert!(raw.place_labels(&measurements[1..], &options).is_err());
    let mut invalid = measurements.clone();
    invalid[0].bounds.x = f64::NAN;
    assert!(raw.place_labels(&invalid, &options).is_err());
    let mut stale = measurements.clone();
    stale[0].signature.push('x');
    assert!(raw.place_labels(&stale, &options).is_err());
    let mut invalid_options = options.clone();
    invalid_options.offsets = vec![(0., 0.); 17];
    assert!(raw.place_labels(&measurements, &invalid_options).is_err());
    invalid_options = options.clone();
    invalid_options.collision.max_checks = 1;
    assert!(raw.place_labels(&measurements, &invalid_options).is_err());
    let mut edited = raw.clone();
    edited.scene.labels[0].x += 1.;
    assert!(edited.place_labels(&measurements, &options).is_err());
    let mut corrupt = raw.clone();
    corrupt.nodes[0].label_slots = vec![usize::MAX];
    assert!(corrupt.place_labels(&measurements, &options).is_err());
    let mut controller = HierarchyFocusController::new(&data()).unwrap();
    let focused = controller
        .focus(
            &config(),
            "group",
            &HierarchyView::Icicle(Default::default()),
        )
        .unwrap();
    assert!(focused.place_labels(&measurements, &options).is_err());
    let mut treemap = TreemapLayoutOptions::default();
    treemap.style.parent_labels.enabled = false;
    let view = HierarchyView::Treemap(treemap);
    let mut focus = HierarchyFocusController::new(&data()).unwrap();
    let root = focus.layout(&config(), &view).unwrap();
    let measured_root = measure(&root);
    let group = focus.focus(&config(), "group", &view).unwrap();
    assert_eq!(
        root.label_candidates().len(),
        group.label_candidates().len()
    );
    assert_eq!(
        root.label_candidates()[0].id,
        group.label_candidates()[0].id
    );
    assert!(
        group
            .place_labels(
                &measured_root,
                &HierarchyLabelPlacementOptions::for_layout(&group)
            )
            .is_err()
    );
    assert_eq!(raw, before);
}

#[cfg(feature = "raster")]
#[test]
fn registered_font_ink_drives_rotated_hierarchy_collision_and_cpu_rendering() {
    use kartis::render::raster::{FontRegistry, MissingGlyphPolicy, render_raster};

    let mut fonts = FontRegistry::new();
    fonts
        .register(include_bytes!("fonts/ABeeZee-Regular.ttf").as_slice(), 0)
        .unwrap();
    let mut rows = data();
    for (node, text) in rows
        .iter_mut()
        .zip(["AV ffi j", "fj fi AV", "jiffy AV", "fij glyph"])
    {
        node.label = text.into();
    }
    let mut style = HierarchyStyleOptions::default();
    style.parent_labels.enabled = true;
    style.parent_labels.size = Some(13.);
    style.parent_labels.rotation = 15.;
    style.leaf_labels.size = Some(15.);
    style.leaf_labels.rotation = -25.;
    let views = [
        HierarchyView::Treemap(TreemapLayoutOptions {
            parent_label_size: 36.,
            style: style.clone(),
            ..Default::default()
        }),
        HierarchyView::CirclePacking(CirclePackingLayoutOptions {
            style: style.clone(),
            ..Default::default()
        }),
        HierarchyView::Icicle(IcicleLayoutOptions {
            style,
            ..Default::default()
        }),
    ];
    for view in views {
        let mut controller = HierarchyFocusController::new(&rows).unwrap();
        let raw = controller.layout(&config(), &view).unwrap();
        let measurements = raw
            .label_candidates()
            .iter()
            .map(|candidate| {
                let bounds = fonts
                    .measure_label(&candidate.label, MissingGlyphPolicy::Error)
                    .unwrap()
                    .unwrap();
                candidate.measured(LayoutRect {
                    x: bounds.x - candidate.label.x,
                    y: bounds.y - candidate.label.y,
                    width: bounds.width,
                    height: bounds.height,
                })
            })
            .collect::<Vec<_>>();
        assert!(
            measurements
                .iter()
                .all(|m| m.bounds.x != 0. && m.bounds.y != 0.)
        );
        let mut options = HierarchyLabelPlacementOptions::for_layout(&raw);
        options.priorities.insert("a".into(), 100);
        options.offsets = vec![(0., 24.), (0., -24.), (0., 48.), (0., -48.)];
        let result = raw.place_labels(&measurements, &options).unwrap();
        let mut occupied = Vec::new();
        for placement in result.label_placements() {
            let Some(slot) = placement.label_index else {
                continue;
            };
            let label = &result.scene.labels[slot];
            let measured = fonts
                .measure_label(label, MissingGlyphPolicy::Error)
                .unwrap()
                .unwrap();
            let actual = LayoutRect {
                x: measured.x,
                y: measured.y,
                width: measured.width,
                height: measured.height,
            };
            let expected = placement.bounds.unwrap();
            assert!((actual.x - expected.x).abs() < 1e-7);
            assert!((actual.y - expected.y).abs() < 1e-7);
            assert!((actual.width - expected.width).abs() < 1e-7);
            assert!((actual.height - expected.height).abs() < 1e-7);
            assert!(options.collision.viewport.contains(actual));
            assert_clip(&result.scene.label_clips[&slot], actual);
            assert!(occupied.iter().all(|other| actual.overlap(*other) == 0.));
            occupied.push(actual);
            assert_eq!(result.scene.label_series[&slot], placement.node_id);
            assert_eq!(result.node(&placement.node_id).unwrap().label_slots, [slot]);
            assert!(label.rotation != 0.);
        }
        assert!(occupied.len() >= 2);
        let outlined =
            outline_labels(&result.scene, &fonts, &OutlineLabelOptions::default()).unwrap();
        assert_eq!(outlined.labels.len(), result.scene.labels.len());
        assert!(outlined.glyph_count > 4 && outlined.command_count > 50);
        assert!(
            outlined
                .labels
                .values()
                .all(|outline| !outline.glyphs.is_empty())
        );

        let mut text_scene = result.scene.clone();
        text_scene.background = Color(0, 0, 0, 0);
        text_scene.marks.clear();
        text_scene.draw_order = (0..text_scene.labels.len()).map(DrawItem::Label).collect();
        let image = render_raster(&text_scene, &fonts, &Default::default()).unwrap();
        assert!(
            image
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|pixel| pixel[3] > 0)
                .count()
                > 100
        );
        assert_eq!(raw.scene.marks, result.scene.marks);
    }
}
