use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::{surface::*, *};
use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::{PI, TAU};

fn scene(geometry: Geometry, value: f64) -> Scene {
    let mut scene = ChartConfig {
        width: 240.,
        height: 200.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    let mut mark = Mark::new("glyph", geometry, Color(25, 80, 180, 255));
    mark.series = "series".into();
    mark.label = "actual glyph".into();
    mark.value = value;
    scene.marks.push(mark);
    scene
}
fn polygon(center: [f64; 2], radius: f64, rotation: f64, star: bool) -> Geometry {
    let count = if star { 10 } else { 3 };
    let mut path = Vec::new();
    for i in 0..count {
        let angle = rotation - PI / 2. + TAU * i as f64 / count as f64;
        let r = radius * if star && i % 2 == 1 { 0.4 } else { 1. };
        let point = P::new(center[0] + angle.cos() * r, center[1] + angle.sin() * r);
        path.push(if i == 0 {
            C::Move(point)
        } else {
            C::Line(point)
        });
    }
    path.push(C::Close);
    Geometry::Path(path)
}
fn binding(scene: &Scene, center: [f64; 2]) -> InteractionBinding {
    let Geometry::Path(path) = &scene.marks[0].geometry else {
        unreachable!()
    };
    InteractionBinding {
        sites: vec![InteractionSite {
            mark_id: "glyph".into(),
            series: "series".into(),
            label: "series".into(),
            value: Some(scene.marks[0].value),
            formatted_value: Some(format!("{:.1}", scene.marks[0].value)),
            color: Color(25, 80, 180, 255),
            position: center,
        }],
        anchors: BTreeMap::from([(
            "glyph".into(),
            InteractionAnchor::path_points(
                &scene.marks[0].geometry,
                center,
                &(0..path.len() - 1).collect::<Vec<_>>(),
            )
            .unwrap(),
        )]),
        ..Default::default()
    }
}
fn pick(model: &SurfaceModel, point: (f64, f64)) -> Result<Vec<String>> {
    let mut next = model.clone();
    next.apply(SurfaceInput::Move { id: 1, point })?;
    Ok(next.hovered().to_vec())
}
fn close(actual: [f64; 2], expected: [f64; 2]) {
    for axis in 0..2 {
        assert!(
            (actual[axis] - expected[axis]).abs() < 1e-10,
            "{actual:?} != {expected:?}"
        );
    }
}
#[test]
fn path_binding_requires_explicit_centroid_support_and_rejects_bbox_guess() {
    let scene = scene(polygon([100., 100.], 40., 0., false), 1.);
    let mut data = binding(&scene, [100., 100.]);
    data.validate(&scene).unwrap();
    data.anchors.clear();
    assert!(data.validate(&scene).is_err());
    // Triangle's geometric origin is y=100; its bounding-box center is y=90.
    assert!(
        InteractionAnchor::path_points(&scene.marks[0].geometry, [100., 90.], &[0, 1, 2]).is_err()
    );
    assert!(
        InteractionAnchor::path_points(&scene.marks[0].geometry, [100., 100.], &[0, 0, 1]).is_err()
    );
    assert!(
        InteractionAnchor::path_points(&scene.marks[0].geometry, [f64::NAN, 100.], &[0, 1, 2])
            .is_err()
    );
    assert!(InteractionAnchor::path_points(&scene.marks[0].geometry, [100., 100.], &[3]).is_err());
    let malformed = Geometry::Path(vec![
        C::Move(P::new(100., 100.)),
        C::Line(P::new(f64::NAN, 100.)),
    ]);
    assert!(InteractionAnchor::path_points(&malformed, [100., 100.], &[0]).is_err());
    let mut extra = binding(&scene, [100., 100.]);
    extra
        .anchors
        .insert("missing".into(), extra.anchors["glyph"].clone());
    assert!(extra.validate(&scene).is_err());
}
#[test]
fn exact_star_hits_real_ink_and_misses_concave_notch_while_nearest_uses_anchor() {
    let scene = scene(polygon([100., 100.], 40., 0., true), 4.);
    let data = binding(&scene, [100., 100.]);
    let exact =
        SurfaceModel::new_with_interaction(scene.clone(), Default::default(), Some(data.clone()))
            .unwrap();
    assert_eq!(pick(&exact, (100., 65.)).unwrap(), ["glyph"]);
    let notch = (
        100. + 24. * (-PI / 2. + TAU / 10.).cos(),
        100. + 24. * (-PI / 2. + TAU / 10.).sin(),
    );
    assert!(pick(&exact, notch).unwrap().is_empty());
    assert_eq!(exact.scene().marks.len(), 1);
    assert!(matches!(exact.scene().marks[0].geometry, Geometry::Path(_)));
    let nearest = SurfaceModel::new_with_interaction(
        scene,
        SurfaceOptions {
            picking: Picking::Nearest { distance: 3. },
            ..Default::default()
        },
        Some(data),
    )
    .unwrap();
    assert_eq!(pick(&nearest, (100., 100.)).unwrap(), ["glyph"]);
    assert!(pick(&nearest, (100., 90.)).unwrap().is_empty());
}
#[test]
fn current_anchor_tracks_actual_rotated_scaled_animation_and_rejects_topology_atomically() {
    let old = scene(polygon([60., 100.], 20., 0., false), 2.);
    let target = scene(polygon([160., 70.], 45., PI / 2., false), 9.);
    let mut model = SurfaceModel::new_with_interaction(
        old.clone(),
        SurfaceOptions {
            picking: Picking::Nearest { distance: 10. },
            ..Default::default()
        },
        Some(binding(&old, [60., 100.])),
    )
    .unwrap();
    model
        .set_interaction_target(&target, Some(binding(&target, [160., 70.])))
        .unwrap();
    close(
        model.active_interaction(&["glyph".into()]).unwrap().anchor,
        [60., 100.],
    );
    let mut motion = motion::SpringMotion::new(old, Default::default()).unwrap();
    motion.retarget(target).unwrap();
    motion.tick(0.08).unwrap();
    let frame = motion.scene().clone();
    let expected = frame.marks[0].geometry.commands()[..3]
        .iter()
        .map(|c| match c {
            C::Move(p) | C::Line(p) => [p.x, p.y],
            _ => unreachable!(),
        })
        .fold([0., 0.], |a, p| [a[0] + p[0] / 3., a[1] + p[1] / 3.]);
    model.set_scene_preserving_interaction(frame).unwrap();
    let active = model.active_interaction(&["glyph".into()]).unwrap();
    close(active.anchor, expected);
    close(active.rows[0].position, expected);
    assert_eq!(active.rows[0].value, Some(9.));
    assert_ne!(active.anchor, [160., 70.]);
    let overlays = model
        .interaction_overlays(
            &["glyph".into()],
            Some(&CrosshairOptions {
                kind: CrosshairType::Cross,
                dash: None,
                ..Default::default()
            }),
        )
        .unwrap();
    let x = overlays[0].geometry.commands();
    assert!(matches!(x[0],C::Move(p) if (p.x-expected[0]).abs()<1e-10));
    let before = model.scene().clone();
    let mut incompatible = before.clone();
    incompatible.marks[0].geometry = polygon([120., 100.], 40., 0., true);
    assert!(
        model
            .set_scene_preserving_interaction(incompatible)
            .is_err()
    );
    assert_eq!(model.scene(), &before);
    close(
        model.active_interaction(&["glyph".into()]).unwrap().anchor,
        expected,
    );
}
#[test]
fn path_slice_keyboard_hidden_and_clipped_nearest_share_the_explicit_anchor() {
    let source = scene(polygon([100., 100.], 30., PI / 7., false), 5.);
    let mut model = SurfaceModel::new_with_interaction(
        source.clone(),
        SurfaceOptions {
            picking: Picking::SliceX { distance: 240. },
            ..Default::default()
        },
        Some(binding(&source, [100., 100.])),
    )
    .unwrap();
    assert_eq!(pick(&model, (100., 10.)).unwrap(), ["glyph"]);
    model.apply(SurfaceInput::First).unwrap();
    assert_eq!(model.selected(), Some("glyph"));
    model.set_hidden(BTreeSet::from(["series".into()])).unwrap();
    assert!(pick(&model, (100., 100.)).unwrap().is_empty());
    assert_eq!(model.selected(), None);
}
#[test]
fn invisible_zero_radius_site_does_not_invent_exact_glyph_ink() {
    let mut source = scene(
        Geometry::Circle {
            x: 100.,
            y: 100.,
            radius: 0.,
        },
        1.,
    );
    source.marks[0].fill = Some(Color(0, 0, 0, 0));
    let data = InteractionBinding {
        sites: vec![InteractionSite {
            mark_id: "glyph".into(),
            series: "series".into(),
            label: "site".into(),
            value: Some(1.),
            formatted_value: None,
            color: Color(0, 0, 0, 0),
            position: [100., 100.],
        }],
        ..Default::default()
    };
    let exact =
        SurfaceModel::new_with_interaction(source.clone(), Default::default(), Some(data.clone()))
            .unwrap();
    assert!(pick(&exact, (100., 100.)).unwrap().is_empty());
    let nearest = SurfaceModel::new_with_interaction(
        source,
        SurfaceOptions {
            picking: Picking::Nearest { distance: 5. },
            ..Default::default()
        },
        Some(data),
    )
    .unwrap();
    assert_eq!(pick(&nearest, (100., 100.)).unwrap(), ["glyph"]);
}
#[test]
fn rounded_endpoint_anchor_outside_fractional_clip_keeps_its_visible_glyph_site() {
    let plot = ClipRect {
        x: 60.25,
        y: 20.25,
        width: 340.,
        height: 200.,
    };
    let mut source = scene(
        Geometry::Circle {
            x: 60.,
            y: 20.,
            radius: 6.,
        },
        0.,
    );
    source.width = 480.;
    source.height = 300.;
    source.marks[0].clip = Some(plot);
    let binding = InteractionBinding {
        sites: vec![InteractionSite {
            mark_id: "glyph".into(),
            series: "series".into(),
            label: "rounded endpoint".into(),
            value: Some(0.),
            formatted_value: None,
            color: Color(25, 80, 180, 255),
            position: [60., 20.],
        }],
        ..Default::default()
    };
    let exact = SurfaceModel::new_with_interaction(
        source.clone(),
        SurfaceOptions {
            plot: Some(plot),
            ..Default::default()
        },
        Some(binding.clone()),
    )
    .unwrap();
    assert_eq!(pick(&exact, (61., 21.)).unwrap(), ["glyph"]);
    #[cfg(feature = "raster")]
    {
        let image =
            render::raster::render_raster(&source, &Default::default(), &Default::default())
                .unwrap();
        let rgba = &image.rgba[(21 * image.width as usize + 61) * 4..][..4];
        assert!(
            rgba[2] > rgba[0] + 80,
            "rounded glyph really has clipped visible ink"
        );
    }
    for picking in [
        Picking::Nearest { distance: 10. },
        Picking::SliceX { distance: 20. },
        Picking::SliceY { distance: 20. },
    ] {
        let model = SurfaceModel::new_with_interaction(
            source.clone(),
            SurfaceOptions {
                plot: Some(plot),
                picking,
                ..Default::default()
            },
            Some(binding.clone()),
        )
        .unwrap();
        assert_eq!(
            pick(&model, (61., 21.)).unwrap(),
            ["glyph"],
            "{picking:?} discarded visible rounded endpoint"
        );
        assert!(
            pick(&model, (60., 20.)).unwrap().is_empty(),
            "query clip must still be enforced"
        );
    }
}
#[test]
fn radar_triangle_binds_actual_path_without_a_duplicate_proxy() {
    let config = ChartConfig {
        width: 400.,
        height: 400.,
        ..Default::default()
    };
    let axes = vec!["speed".into(), "quality".into(), "uptime".into()];
    let options = RadarLayoutOptions {
        domain: Some([0., 10.]),
        points: RadarPointOptions {
            shape: RadarPointShape::Triangle,
            size: 30.,
            ..Default::default()
        },
        ..Default::default()
    };
    let layout = radar_layout(
        &config,
        &axes,
        &[RadarSeries::new("A", vec![8., 7., 6.])],
        &options,
        &RadarInteractionOptions {
            slices: false,
            ..Default::default()
        },
    )
    .unwrap();
    let data = layout.interaction_binding();
    assert!(data.polar.is_none());
    assert_eq!(data.anchors.len(), 3);
    for site in &data.sites {
        assert!(matches!(
            layout
                .scene
                .marks
                .iter()
                .find(|m| m.id == site.mark_id)
                .unwrap()
                .geometry,
            Geometry::Path(_)
        ));
    }
    assert!(
        !layout
            .scene
            .marks
            .iter()
            .any(|m| m.id.starts_with("radar-glyph:"))
    );
    let model =
        SurfaceModel::new_with_interaction(layout.scene, Default::default(), Some(data.clone()))
            .unwrap();
    assert_eq!(
        pick(
            &model,
            (data.sites[0].position[0], data.sites[0].position[1])
        )
        .unwrap(),
        [data.sites[0].mark_id.clone()]
    );
}
#[test]
fn fade_live_frame_installs_new_shape_atomically_and_keeps_logical_selection_at_zero_presence() {
    let mut old = scene(
        Geometry::Circle {
            x: 100.,
            y: 100.,
            radius: 12.,
        },
        2.,
    );
    let old_binding = InteractionBinding {
        sites: vec![InteractionSite {
            mark_id: "glyph".into(),
            series: "series".into(),
            label: "circle".into(),
            value: Some(2.),
            formatted_value: None,
            color: Color(25, 80, 180, 255),
            position: [100., 100.],
        }],
        ..Default::default()
    };
    let mut model =
        SurfaceModel::new_with_interaction(old.clone(), Default::default(), Some(old_binding))
            .unwrap();
    model.apply(SurfaceInput::First).unwrap();
    let target = scene(polygon([100., 100.], 30., 0., true), 9.);
    let target_binding = binding(&target, [100., 100.]);
    let mut motion = motion::PropertyMotion::new(old.clone(), Default::default()).unwrap();
    motion.retarget(target.clone()).unwrap();
    model
        .set_interaction_frame(
            &target,
            Some(target_binding.clone()),
            motion.scene().clone(),
        )
        .unwrap();
    assert_eq!(
        model.selected(),
        Some("glyph"),
        "stable business selection must survive enter opacity zero"
    );
    assert!(
        pick(&model, (100., 100.)).unwrap().is_empty(),
        "zero-opacity target must not become Exact physical ink"
    );
    let controls = model.controls_scene();
    let data = controls
        .marks
        .iter()
        .filter(|m| !m.label.is_empty() && m.opacity > 0.)
        .collect::<Vec<_>>();
    assert_eq!(
        data.len(),
        1,
        "old exit ghost must not become a duplicate control"
    );
    assert_eq!(data[0].id, "glyph");
    assert_eq!(data[0].value, 9.);
    motion.tick(0.04).unwrap();
    model
        .set_scene_preserving_interaction(motion.scene().clone())
        .unwrap();
    assert_eq!(
        model.active_interaction(&["glyph".into()]).unwrap().rows[0].value,
        Some(9.)
    );
    let before = model.scene().clone();
    old.marks[0].geometry = Geometry::Circle {
        x: 110.,
        y: 100.,
        radius: 12.,
    };
    assert!(
        model
            .set_interaction_frame(&target, Some(target_binding), old)
            .is_err()
    );
    assert_eq!(model.scene(), &before);
}
#[test]
fn all_fourteen_d3_glyphs_bind_real_geometry_and_affine_frame_anchors() {
    let shapes = [
        GlyphShape::Filled(FilledGlyph::Circle),
        GlyphShape::Filled(FilledGlyph::Cross),
        GlyphShape::Filled(FilledGlyph::Diamond),
        GlyphShape::Filled(FilledGlyph::Square),
        GlyphShape::Filled(FilledGlyph::Star),
        GlyphShape::Filled(FilledGlyph::Triangle),
        GlyphShape::Filled(FilledGlyph::Wye),
        GlyphShape::Stroked(StrokedGlyph::Circle),
        GlyphShape::Stroked(StrokedGlyph::Plus),
        GlyphShape::Stroked(StrokedGlyph::Times),
        GlyphShape::Stroked(StrokedGlyph::Triangle),
        GlyphShape::Stroked(StrokedGlyph::Asterisk),
        GlyphShape::Stroked(StrokedGlyph::Square),
        GlyphShape::Stroked(StrokedGlyph::Diamond),
    ];
    for shape in shapes {
        let initial = glyph_geometry(
            [100., 90.],
            &GlyphOptions {
                shape,
                size: 256.,
                rotation: 0.37,
            },
        )
        .unwrap();
        let next = glyph_geometry(
            [160., 110.],
            &GlyphOptions {
                shape,
                size: 400.,
                rotation: 1.1,
            },
        )
        .unwrap();
        let make = |glyph: &GlyphGeometry| {
            let mut source = scene(glyph.geometry.clone(), 3.);
            if glyph.paint_mode == GlyphPaintMode::Stroke {
                source.marks[0].fill = None;
                source.marks[0].stroke = Some(glory::render::GraphicStroke {
                    color: Color(20, 60, 180, 255),
                    width: 1.5,
                    cap: glory::render::StrokeCap::Round,
                });
            }
            let data = InteractionBinding {
                sites: vec![InteractionSite {
                    mark_id: "glyph".into(),
                    series: "series".into(),
                    label: "actual".into(),
                    value: Some(3.),
                    formatted_value: Some("3".into()),
                    color: Color(20, 60, 180, 255),
                    position: glyph.anchor,
                }],
                anchors: if glyph.anchor_indices.is_empty() {
                    Default::default()
                } else {
                    BTreeMap::from([(
                        "glyph".into(),
                        InteractionAnchor::path_points(
                            &glyph.geometry,
                            glyph.anchor,
                            &glyph.anchor_indices,
                        )
                        .unwrap(),
                    )])
                },
                ..Default::default()
            };
            (source, data)
        };
        let (old, old_binding) = make(&initial);
        let (target, target_binding) = make(&next);
        let mut nearest = SurfaceModel::new_with_interaction(
            old.clone(),
            SurfaceOptions {
                picking: Picking::Nearest { distance: 2. },
                ..Default::default()
            },
            Some(old_binding.clone()),
        )
        .unwrap();
        assert_eq!(pick(&nearest, (100., 90.)).unwrap(), ["glyph"], "{shape:?}");
        let exact =
            SurfaceModel::new_with_interaction(old.clone(), Default::default(), Some(old_binding))
                .unwrap();
        let ink = match &initial.geometry {
            Geometry::Circle { x, y, radius } => (*x + radius, *y),
            Geometry::Path(p) => match p[0] {
                C::Move(p) => (p.x, p.y),
                _ => unreachable!(),
            },
            _ => unreachable!(),
        };
        if initial.paint_mode == GlyphPaintMode::Stroke {
            assert_eq!(pick(&exact, ink).unwrap(), ["glyph"], "{shape:?}");
        }
        assert!(pick(&exact, (20., 20.)).unwrap().is_empty());
        nearest
            .set_interaction_target(&target, Some(target_binding))
            .unwrap();
        nearest
            .set_scene_preserving_interaction(old.interpolate(&target, 0.37).unwrap())
            .unwrap();
        close(
            nearest
                .active_interaction(&["glyph".into()])
                .unwrap()
                .anchor,
            [122.2, 97.4],
        );
    }
}
#[test]
fn radar_explicit_stroked_glyph_requires_border_and_uses_true_point_exact() {
    let config = ChartConfig {
        width: 400.,
        height: 400.,
        ..Default::default()
    };
    let axes = vec!["a".into(), "b".into(), "c".into()];
    let data = [RadarSeries::new("A", vec![8., 7., 6.])];
    let mut options = RadarLayoutOptions {
        domain: Some([0., 10.]),
        points: RadarPointOptions {
            glyph: Some(GlyphOptions {
                shape: GlyphShape::Stroked(StrokedGlyph::Triangle),
                size: 256.,
                rotation: 0.4,
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    let interaction = RadarInteractionOptions {
        slices: false,
        ..Default::default()
    };
    assert!(radar_layout(&config, &axes, &data, &options, &interaction).is_err());
    options.points.border.enabled = true;
    options.points.border.width = 1.5;
    let layout = radar_layout(&config, &axes, &data, &options, &interaction).unwrap();
    let binding = layout.interaction_binding();
    let anchor = binding.sites[0].position;
    let model = SurfaceModel::new_with_interaction(layout.scene, Default::default(), Some(binding))
        .unwrap();
    // A hollow triangle's center is not physical ink or a proxy Circle.
    assert!(pick(&model, (anchor[0], anchor[1])).unwrap().is_empty());
}
#[cfg(feature = "raster")]
#[test]
fn zero_size_fill_has_no_exact_ink_but_round_capped_open_stroke_can_have_ink() {
    for shape in [
        GlyphShape::Filled(FilledGlyph::Circle),
        GlyphShape::Filled(FilledGlyph::Star),
    ] {
        let glyph = glyph_geometry(
            [100., 100.],
            &GlyphOptions {
                shape,
                size: 0.,
                rotation: 0.,
            },
        )
        .unwrap();
        let source = scene(glyph.geometry.clone(), 1.);
        let mut data = InteractionBinding {
            sites: vec![InteractionSite {
                mark_id: "glyph".into(),
                series: "series".into(),
                label: "zero".into(),
                value: Some(1.),
                formatted_value: None,
                color: Color(25, 80, 180, 255),
                position: glyph.anchor,
            }],
            ..Default::default()
        };
        if !glyph.anchor_indices.is_empty() {
            data.anchors.insert(
                "glyph".into(),
                InteractionAnchor::path_points(
                    &glyph.geometry,
                    glyph.anchor,
                    &glyph.anchor_indices,
                )
                .unwrap(),
            );
        }
        let model =
            SurfaceModel::new_with_interaction(source.clone(), Default::default(), Some(data))
                .unwrap();
        let image =
            render::raster::render_raster(&source, &Default::default(), &Default::default())
                .unwrap();
        assert_eq!(
            &image.rgba[(100 * image.width as usize + 100) * 4..][..4],
            &[255, 255, 255, 255],
            "{shape:?}"
        );
        assert!(
            pick(&model, (100., 100.)).unwrap().is_empty(),
            "zero-size filled {shape:?} fabricated ink"
        );
    }
    let glyph = glyph_geometry(
        [100., 100.],
        &GlyphOptions {
            shape: GlyphShape::Stroked(StrokedGlyph::Plus),
            size: 0.,
            rotation: 0.,
        },
    )
    .unwrap();
    let mut source = scene(glyph.geometry.clone(), 1.);
    source.marks[0].fill = None;
    source.marks[0].stroke = Some(glory::render::GraphicStroke {
        color: Color(25, 80, 180, 255),
        width: 6.,
        cap: glory::render::StrokeCap::Round,
    });
    let mut data = binding(&source, [100., 100.]);
    data.anchors.insert(
        "glyph".into(),
        InteractionAnchor::path_points(&glyph.geometry, glyph.anchor, &glyph.anchor_indices)
            .unwrap(),
    );
    let model =
        SurfaceModel::new_with_interaction(source.clone(), Default::default(), Some(data)).unwrap();
    let image =
        render::raster::render_raster(&source, &Default::default(), &Default::default()).unwrap();
    assert!(image.rgba[(100 * image.width as usize + 100) * 4..][..4] != [255, 255, 255, 255]);
    assert_eq!(pick(&model, (100., 100.)).unwrap(), ["glyph"]);
    source.marks[0].stroke.as_mut().unwrap().cap = glory::render::StrokeCap::Butt;
    let data = model.interaction_data().unwrap().clone();
    let butt =
        SurfaceModel::new_with_interaction(source.clone(), Default::default(), Some(data)).unwrap();
    let image =
        render::raster::render_raster(&source, &Default::default(), &Default::default()).unwrap();
    assert_eq!(
        &image.rgba[(100 * image.width as usize + 100) * 4..][..4],
        &[255, 255, 255, 255]
    );
    assert!(
        pick(&butt, (100., 100.)).unwrap().is_empty(),
        "zero-length Butt glyph has no physical stroke ink"
    );
}
#[cfg(feature = "raster")]
#[test]
fn cpu_star_pixels_confirm_true_notch_miss_and_real_ink_pick() {
    let source = scene(polygon([100., 100.], 40., 0., true), 4.);
    let model = SurfaceModel::new_with_interaction(
        source.clone(),
        Default::default(),
        Some(binding(&source, [100., 100.])),
    )
    .unwrap();
    let image =
        render::raster::render_raster(&source, &Default::default(), &Default::default()).unwrap();
    let pixel = |x: usize, y: usize| &image.rgba[(y * image.width as usize + x) * 4..][..4];
    assert!(pixel(100, 65)[2] > pixel(100, 65)[0] + 80);
    assert!(pick(&model, (100.5, 65.5)).unwrap().len() == 1);
    assert_eq!(pixel(114, 80), [255, 255, 255, 255]);
    assert!(pick(&model, (114.5, 80.5)).unwrap().is_empty());
}
