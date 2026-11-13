use kartis::*;
use std::f64::consts::{FRAC_PI_4, TAU};

fn config() -> ChartConfig {
    ChartConfig {
        width: 340.,
        height: 260.,
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
        HierarchyDatum::new("root", None, 2.),
        HierarchyDatum::new("group", Some("root".into()), 1.),
        HierarchyDatum::new("a", Some("group".into()), 4.),
        HierarchyDatum::new("b", Some("group".into()), 3.),
        HierarchyDatum::new("outside", Some("root".into()), 2.),
    ]
}

fn options() -> SunburstLayoutOptions {
    SunburstLayoutOptions {
        include_root: true,
        labels: Some(SunburstLabelOptions {
            text: SunburstLabelText::Label,
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn source() -> SunburstLayout {
    sunburst_layout(&config(), &data(), &options()).unwrap()
}

fn measure(layout: &SunburstLayout) -> Vec<SunburstMeasuredLabel> {
    layout
        .label_candidates()
        .iter()
        .map(|candidate| {
            candidate.measured(LayoutRect {
                x: -8.,
                y: -8.,
                width: 16.,
                height: 10.,
            })
        })
        .collect()
}

fn placed<'a>(layout: &'a SunburstLayout, id: &str) -> &'a SunburstPlacedLabel {
    layout
        .label_placements()
        .iter()
        .find(|p| p.node_id == id)
        .unwrap()
}

fn keep_only(options: &mut SunburstLabelPlacementOptions, source: &SunburstLayout, id: &str) {
    options.hidden = source
        .label_candidates()
        .iter()
        .filter(|c| c.node_id != id)
        .map(|c| c.node_id.clone())
        .collect();
}

#[test]
fn explicit_placement_keeps_default_labels_identity_and_raw_interaction() {
    let default = sunburst_layout(&config(), &data(), &Default::default()).unwrap();
    assert!(default.scene.labels.is_empty());
    assert!(default.label_candidates().is_empty());
    let raw = source();
    assert!(raw.label_placements().is_empty());
    assert!(raw.scene.labels.iter().all(|l| l.rotation == 0.));
    assert!(raw.scene.label_clips.is_empty());
    assert_eq!(raw, raw.with_label_rotation(0.).unwrap());
    let mut reversed = data();
    reversed.reverse();
    let reversed = sunburst_layout(&config(), &reversed, &options()).unwrap();
    assert_eq!(
        raw.label_candidates()
            .iter()
            .map(|c| &c.id)
            .collect::<Vec<_>>(),
        reversed
            .label_candidates()
            .iter()
            .map(|c| &c.id)
            .collect::<Vec<_>>()
    );
    let measurements = measure(&raw);
    let result = raw
        .place_labels(
            &measurements,
            &SunburstLabelPlacementOptions::for_layout(&raw),
        )
        .unwrap();
    assert_eq!(result.scene.marks, raw.scene.marks);
    assert_eq!(
        result.interaction_binding().unwrap(),
        raw.interaction_binding().unwrap()
    );
    assert_eq!(result.total, raw.total);
    assert_eq!(
        result
            .path("a")
            .unwrap()
            .iter()
            .map(|n| n.data.id.as_str())
            .collect::<Vec<_>>(),
        ["a", "group", "root"]
    );
    for candidate in raw.label_candidates() {
        let placement = placed(&result, &candidate.node_id);
        assert_eq!(placement.id, candidate.id);
        if let Some(slot) = placement.label_index {
            assert_eq!(result.scene.label_series[&slot], candidate.node_id);
            assert_eq!(
                result.node(&candidate.node_id).unwrap().label_index,
                Some(slot)
            );
            assert_eq!(result.scene.labels[slot], candidate.label);
        }
    }
    assert_eq!(
        result.label_placement_stats(),
        SunburstLabelPlacementStats::default()
    );
    assert_eq!(
        result,
        result
            .place_labels(
                &measurements,
                &SunburstLabelPlacementOptions::for_layout(&raw)
            )
            .unwrap()
    );
    assert!(render::to_svg(&result.scene).is_ok());
    assert!(render::to_canvas_script(&result.scene, "sunburst").is_ok());
}

#[test]
fn priorities_offsets_hidden_restore_and_custom_paint_order_are_preserved() {
    let mut raw = source();
    let custom = Label {
        x: 20.,
        y: 24.,
        text: "Annotation".into(),
        size: 12.,
        color: Color(20, 30, 40, 255),
        anchor: TextAnchor::Start,
        rotation: 0.,
    };
    let custom_clip = ClipPath {
        geometry: Geometry::Rect {
            x: 12.,
            y: 12.,
            width: 150.,
            height: 30.,
        },
        rule: ClipRule::NonZero,
    };
    let index = raw.scene.labels.len();
    raw.scene.labels.push(custom.clone());
    raw.scene.label_series.insert(index, "annotation".into());
    raw.scene.label_clips.insert(index, custom_clip.clone());
    raw.scene.draw_order = raw.scene.draw_items();
    let annotation = raw.scene.draw_order.pop().unwrap();
    raw.scene.draw_order.insert(0, annotation);
    let common = LayoutRect {
        x: 140.,
        y: 110.,
        width: 30.,
        height: 12.,
    };
    let measurements = raw
        .label_candidates()
        .iter()
        .map(|candidate| {
            candidate.measured(LayoutRect {
                x: common.x - candidate.label.x,
                y: common.y - candidate.label.y,
                ..common
            })
        })
        .collect::<Vec<_>>();
    let mut options = SunburstLabelPlacementOptions::for_layout(&raw);
    options.priorities.insert("root".into(), 100);
    let first = raw.place_labels(&measurements, &options).unwrap();
    assert!(placed(&first, "root").label_index.is_some());
    assert!(placed(&first, "a").label_index.is_none());
    options.priorities.insert("a".into(), 200);
    let second = first.place_labels(&measurements, &options).unwrap();
    assert!(placed(&second, "a").label_index.is_some());
    assert!(placed(&second, "root").label_index.is_none());
    options.offsets = vec![(0., 18.), (0., -18.), (0., 36.), (0., -36.)];
    let moved = second.place_labels(&measurements, &options).unwrap();
    assert!(
        moved
            .label_placements()
            .iter()
            .all(|p| p.label_index.is_some())
    );
    assert_eq!(placed(&moved, "root").bounds.unwrap().y, common.y + 18.);
    for (i, a) in moved.label_placements().iter().enumerate() {
        for b in moved.label_placements().iter().skip(i + 1) {
            assert_eq!(a.bounds.unwrap().overlap(b.bounds.unwrap()), 0.);
        }
    }
    options.hidden = raw
        .label_candidates()
        .iter()
        .map(|c| c.node_id.clone())
        .collect();
    let hidden = moved.place_labels(&measurements, &options).unwrap();
    assert_eq!(
        hidden.scene.labels.as_slice(),
        std::slice::from_ref(&custom)
    );
    assert_eq!(hidden.scene.label_series[&0], "annotation");
    assert_eq!(hidden.scene.label_clips[&0], custom_clip);
    assert_eq!(hidden.scene.draw_order[0], DrawItem::Label(0));
    assert!(hidden.nodes.iter().all(|node| node.label_index.is_none()));
    options.hidden.clear();
    let restored = hidden.place_labels(&measurements, &options).unwrap();
    assert_eq!(restored.scene.labels[0], custom);
    assert_eq!(restored.scene.label_clips[&0], custom_clip);
    assert_eq!(restored.scene.draw_order[0], DrawItem::Label(0));
    assert_eq!(restored.scene.marks, raw.scene.marks);
    restored.scene.validate().unwrap();
}

#[test]
fn actual_arc_mask_respects_rounding_padding_and_partial_bounds() {
    let mut options = options();
    options.corner_radius = 8.;
    options.pad_angle = 0.2;
    let raw = sunburst_layout(&config(), &data(), &options).unwrap();
    let candidate = raw
        .label_candidates()
        .iter()
        .find(|c| c.node_id == "a")
        .unwrap();
    assert_eq!(
        candidate.mask.geometry,
        raw.scene.marks[raw.node("a").unwrap().mark_index.unwrap()].geometry
    );
    let radius = (candidate.arc.inner_radius + candidate.arc.outer_radius) / 2.;
    let gap = raw.frame.point(radius, candidate.arc.angles[0] + 0.01);
    assert!(!candidate.mask.contains(gap[0], gap[1]));
    let mut measurements = measure(&raw);
    let index = measurements
        .iter()
        .position(|m| m.id == candidate.id)
        .unwrap();
    measurements[index].bounds = LayoutRect {
        x: gap[0] - candidate.label.x - 0.2,
        y: gap[1] - candidate.label.y - 0.2,
        width: 0.4,
        height: 0.4,
    };
    let mut placement = SunburstLabelPlacementOptions::for_layout(&raw);
    keep_only(&mut placement, &raw, "a");
    let plot = raw.place_labels(&measurements, &placement).unwrap();
    assert!(placed(&plot, "a").label_index.is_some());
    placement.clip = SunburstLabelClip::ClipToArc;
    assert!(
        placed(&raw.place_labels(&measurements, &placement).unwrap(), "a")
            .label_index
            .is_none()
    );

    let at = raw.frame.point(
        candidate.arc.outer_radius - 1.,
        (candidate.arc.angles[0] + candidate.arc.angles[1]) / 2.,
    );
    measurements[index].bounds = LayoutRect {
        x: at[0] - candidate.label.x - 5.,
        y: at[1] - candidate.label.y - 5.,
        width: 10.,
        height: 10.,
    };
    let clipped = raw.place_labels(&measurements, &placement).unwrap();
    let slot = placed(&clipped, "a").label_index.unwrap();
    assert_eq!(clipped.scene.label_clips[&slot], candidate.mask);
    assert!(clipped.label_placement_stats().mask_queries > 0);
    placement.clip = SunburstLabelClip::RequireFullInkBounds;
    assert!(
        placed(&raw.place_labels(&measurements, &placement).unwrap(), "a")
            .label_index
            .is_none()
    );
}

#[test]
fn strict_mask_detects_inner_holes_even_when_all_four_corners_are_filled() {
    let raw = sunburst_layout(
        &config(),
        &[HierarchyDatum::new("ring", None, 1.)],
        &SunburstLayoutOptions {
            inner_ratio: 0.5,
            ..options()
        },
    )
    .unwrap();
    let candidate = &raw.label_candidates()[0];
    let [cx, cy] = raw.frame.center;
    let half = raw.frame.outer_radius * 0.7;
    for (x, y) in [
        (cx - half, cy - half),
        (cx + half, cy - half),
        (cx - half, cy + half),
        (cx + half, cy + half),
    ] {
        assert!(
            candidate.mask.contains(x, y),
            "point={x},{y}; arc={:?}; path={:?}",
            candidate.arc,
            candidate.mask.geometry
        );
    }
    let measurements = [candidate.measured(LayoutRect {
        x: cx - half - candidate.label.x,
        y: cy - half - candidate.label.y,
        width: 2. * half,
        height: 2. * half,
    })];
    let mut options = SunburstLabelPlacementOptions::for_layout(&raw);
    options.clip = SunburstLabelClip::RequireFullInkBounds;
    assert!(
        placed(&raw.place_labels(&measurements, &options).unwrap(), "ring")
            .bounds
            .is_none()
    );
    options.clip = SunburstLabelClip::ClipToArc;
    assert!(
        placed(&raw.place_labels(&measurements, &options).unwrap(), "ring")
            .bounds
            .is_some()
    );
}

#[test]
fn strict_rotated_aabb_can_be_rejected_while_its_oriented_ink_box_fits() {
    let cfg = ChartConfig {
        width: 224.,
        height: 224.,
        ..config()
    };
    let raw = sunburst_layout(
        &cfg,
        &[HierarchyDatum::new("ring", None, 1.)],
        &SunburstLayoutOptions {
            inner_ratio: 0.8,
            start_angle: -3. * FRAC_PI_4,
            end_angle: -3. * FRAC_PI_4 + TAU,
            labels: Some(SunburstLabelOptions {
                radius_offset: 0.7,
                ..Default::default()
            }),
            ..options()
        },
    )
    .unwrap()
    .with_label_rotation(45.)
    .unwrap();
    let candidate = &raw.label_candidates()[0];
    let center = raw.frame.point(94., FRAC_PI_4);
    let unit = 0.5_f64.sqrt();
    for along in [-20., 0., 20.] {
        for across in [-3., 0., 3.] {
            assert!(
                candidate.mask.contains(
                    center[0] + unit * (along + across),
                    center[1] + unit * (along - across)
                ),
                "point={along},{across}; arc={:?}; center={center:?}",
                candidate.arc
            );
        }
    }
    let half = 23. * unit;
    let measurements = [candidate.measured(LayoutRect {
        x: center[0] - half - candidate.label.x,
        y: center[1] - half - candidate.label.y,
        width: half * 2.,
        height: half * 2.,
    })];
    let mut options = SunburstLabelPlacementOptions::for_layout(&raw);
    options.clip = SunburstLabelClip::ClipToArc;
    assert!(
        placed(&raw.place_labels(&measurements, &options).unwrap(), "ring")
            .bounds
            .is_some()
    );
    options.clip = SunburstLabelClip::RequireFullInkBounds;
    assert!(
        placed(&raw.place_labels(&measurements, &options).unwrap(), "ring")
            .bounds
            .is_none()
    );
}

#[test]
fn mask_work_is_cumulative_across_preparation_queries_and_nodes() {
    let raw = source();
    let measurements = measure(&raw);
    let mut options = SunburstLabelPlacementOptions::for_layout(&raw);
    options.clip = SunburstLabelClip::ClipToArc;
    options.offsets = vec![(0., 0.)];
    let work: usize = raw
        .label_candidates()
        .iter()
        .map(|c| c.mask.picking_stats().unwrap().work)
        .sum();
    options.max_mask_work = work * 3;
    let placed = raw.place_labels(&measurements, &options).unwrap();
    assert_eq!(placed.label_placement_stats().mask_work, work * 3);
    assert_eq!(
        placed.label_placement_stats().mask_queries,
        raw.label_candidates().len() * 2
    );
    options.max_mask_work -= 1;
    assert!(
        raw.place_labels(&measurements, &options)
            .unwrap_err()
            .to_string()
            .contains("mask work budget")
    );
    options.clip = SunburstLabelClip::Plot;
    options.max_mask_work = 1;
    assert_eq!(
        raw.place_labels(&measurements, &options)
            .unwrap()
            .label_placement_stats()
            .mask_work,
        0
    );
}

#[test]
fn stale_style_arc_topology_metadata_and_resource_errors_preserve_the_source() {
    let raw = source();
    let before = raw.clone();
    let measurements = measure(&raw);
    let options = SunburstLabelPlacementOptions::for_layout(&raw);
    let mut duplicate = measurements.clone();
    duplicate[0] = duplicate[1].clone();
    assert!(raw.place_labels(&duplicate, &options).is_err());
    assert!(raw.place_labels(&measurements[1..], &options).is_err());
    let mut invalid = measurements.clone();
    invalid[0].bounds.width = f64::NAN;
    assert!(raw.place_labels(&invalid, &options).is_err());
    let rotated = raw.with_label_rotation(20.).unwrap();
    for (original, rotated) in raw
        .label_candidates()
        .iter()
        .zip(rotated.label_candidates())
    {
        let baseline = original.label.size * 0.35;
        let angle = rotated.label.rotation.to_radians();
        assert!((rotated.label.x + angle.sin() * baseline - original.label.x).abs() < 1e-12);
        assert!(
            (rotated.label.y - angle.cos() * baseline - (original.label.y - baseline)).abs()
                < 1e-12
        );
    }
    assert!(rotated.place_labels(&measurements, &options).is_err());
    assert!(raw.with_label_rotation(f64::INFINITY).is_err());
    let mut padded = self::options();
    padded.pad_angle = 0.1;
    let padded = sunburst_layout(&config(), &data(), &padded).unwrap();
    assert_eq!(padded.scene.labels, raw.scene.labels);
    assert!(padded.place_labels(&measurements, &options).is_err());
    let mut recolored = self::options();
    recolored.labels.as_mut().unwrap().color = InheritedColor::Fixed(Color(100, 20, 60, 255));
    assert!(
        sunburst_layout(&config(), &data(), &recolored)
            .unwrap()
            .place_labels(&measurements, &options)
            .is_err()
    );
    let mut edited = raw.clone();
    edited.scene.labels[0].x += 1.;
    assert!(edited.place_labels(&measurements, &options).is_err());
    let mut edited = raw.clone();
    edited.nodes[0].label_index = Some(usize::MAX);
    assert!(edited.place_labels(&measurements, &options).is_err());
    let mut edited = raw.clone();
    edited.nodes[0].arc.outer_radius -= 1.;
    assert!(edited.place_labels(&measurements, &options).is_err());
    let mut edited = raw.clone();
    edited.nodes[0].data.parent = Some(String::new());
    assert!(edited.place_labels(&measurements, &options).is_err());
    let mut edited = raw.clone();
    edited.scene.marks[0].clip = Some(raw.plot());
    assert!(edited.place_labels(&measurements, &options).is_err());
    let mut limit = options.clone();
    limit.offsets = vec![(0., 0.); 17];
    assert!(raw.place_labels(&measurements, &limit).is_err());
    limit = options.clone();
    limit.collision.max_checks = 1;
    assert!(raw.place_labels(&measurements, &limit).is_err());
    let result = raw.place_labels(&measurements, &options).unwrap();
    assert!(result.with_label_rotation(10.).is_err());
    assert_eq!(raw, before);
}

#[cfg(feature = "raster")]
#[test]
fn actual_abeezee_rotated_ink_drives_all_strategies_and_cpu_outlines() {
    use kartis::render::raster::{FontRegistry, MissingGlyphPolicy, render_raster};
    let mut fonts = FontRegistry::new();
    fonts
        .register(include_bytes!("fonts/ABeeZee-Regular.ttf").as_slice(), 0)
        .unwrap();
    let mut rows = data();
    for (node, text) in rows
        .iter_mut()
        .zip(["AV fi", "jiffy", "ffi j", "fi AV", "glyph"])
    {
        node.label = text.into();
    }
    let raw = sunburst_layout(
        &config(),
        &rows,
        &SunburstLayoutOptions {
            corner_radius: 3.,
            pad_angle: 0.025,
            ..options()
        },
    )
    .unwrap()
    .with_label_rotation(12.)
    .unwrap();
    let measurements = raw
        .label_candidates()
        .iter()
        .map(|c| {
            let ink = fonts
                .measure_label(&c.label, MissingGlyphPolicy::Error)
                .unwrap()
                .unwrap();
            c.measured(LayoutRect {
                x: ink.x - c.label.x,
                y: ink.y - c.label.y,
                width: ink.width,
                height: ink.height,
            })
        })
        .collect::<Vec<_>>();
    for clip in [
        SunburstLabelClip::Plot,
        SunburstLabelClip::ClipToArc,
        SunburstLabelClip::RequireFullInkBounds,
    ] {
        let mut options = SunburstLabelPlacementOptions::for_layout(&raw);
        options.clip = clip;
        options.offsets = vec![(0., 12.), (0., -12.)];
        let result = raw.place_labels(&measurements, &options).unwrap();
        let mut occupied = Vec::new();
        for p in result.label_placements() {
            let Some(slot) = p.label_index else { continue };
            let label = &result.scene.labels[slot];
            let ink = fonts
                .measure_label(label, MissingGlyphPolicy::Error)
                .unwrap()
                .unwrap();
            let bounds = LayoutRect {
                x: ink.x,
                y: ink.y,
                width: ink.width,
                height: ink.height,
            };
            let expected = p.bounds.unwrap();
            assert!((bounds.x - expected.x).abs() < 1e-7 && (bounds.y - expected.y).abs() < 1e-7);
            assert!(occupied.iter().all(|other| bounds.overlap(*other) == 0.));
            occupied.push(bounds);
            assert_eq!(label.rotation, 12.);
            assert_eq!(result.scene.label_series[&slot], p.node_id);
            assert_eq!(result.node(&p.node_id).unwrap().label_index, Some(slot));
            if clip != SunburstLabelClip::Plot {
                assert_eq!(
                    result.scene.label_clips[&slot],
                    raw.label_candidates()
                        .iter()
                        .find(|c| c.id == p.id)
                        .unwrap()
                        .mask
                );
            }
        }
        assert!(!occupied.is_empty());
        let outline = outline_labels(&result.scene, &fonts, &Default::default()).unwrap();
        assert_eq!(outline.labels.len(), result.scene.labels.len());
        assert!(outline.glyph_count > 0 && outline.command_count > 20);
        let mut text = result.scene.clone();
        text.marks.clear();
        text.background = Color(0, 0, 0, 0);
        text.draw_order = (0..text.labels.len()).map(DrawItem::Label).collect();
        let image = render_raster(&text, &fonts, &Default::default()).unwrap();
        assert!(
            image
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|p| p[3] > 0)
                .count()
                > 30
        );
    }
}

#[cfg(feature = "raster")]
#[test]
fn actual_annular_text_mask_crops_ink_while_strict_bounds_omit_it() {
    use kartis::render::raster::{FontRegistry, MissingGlyphPolicy, render_raster};
    let mut fonts = FontRegistry::new();
    fonts
        .register(include_bytes!("fonts/ABeeZee-Regular.ttf").as_slice(), 0)
        .unwrap();
    let mut row = HierarchyDatum::new("ring", None, 1.);
    row.label = "WIDE AV ffi j WIDE".into();
    let raw = sunburst_layout(
        &config(),
        &[row],
        &SunburstLayoutOptions {
            inner_ratio: 0.55,
            labels: Some(SunburstLabelOptions {
                text: SunburstLabelText::Label,
                size: Some(24.),
                ..Default::default()
            }),
            ..options()
        },
    )
    .unwrap();
    let c = &raw.label_candidates()[0];
    let ink = fonts
        .measure_label(&c.label, MissingGlyphPolicy::Error)
        .unwrap()
        .unwrap();
    let measurements = [c.measured(LayoutRect {
        x: ink.x - c.label.x,
        y: ink.y - c.label.y,
        width: ink.width,
        height: ink.height,
    })];
    let mut options = SunburstLabelPlacementOptions::for_layout(&raw);
    let unmasked = raw.place_labels(&measurements, &options).unwrap();
    options.clip = SunburstLabelClip::ClipToArc;
    let masked = raw.place_labels(&measurements, &options).unwrap();
    assert!(placed(&masked, "ring").label_index.is_some());
    options.clip = SunburstLabelClip::RequireFullInkBounds;
    assert!(
        placed(&raw.place_labels(&measurements, &options).unwrap(), "ring")
            .label_index
            .is_none()
    );
    let pixels = |layout: &SunburstLayout| {
        let mut scene = layout.scene.clone();
        scene.background = Color(0, 0, 0, 0);
        scene.marks.clear();
        scene.draw_order = (0..scene.labels.len()).map(DrawItem::Label).collect();
        render_raster(&scene, &fonts, &Default::default())
            .unwrap()
            .rgba
    };
    let whole = pixels(&unmasked);
    let cropped = pixels(&masked);
    let ink_pixels = |rgba: &[u8]| rgba.as_chunks::<4>().0.iter().filter(|p| p[3] > 0).count();
    assert!(ink_pixels(&cropped) > 100);
    assert!(ink_pixels(&cropped) < ink_pixels(&whole));
}
