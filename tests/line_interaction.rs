use glory::render::PathCommand;
use kartis::{mesh::SliceAxis, surface::*, *};
use std::collections::BTreeSet;

fn config() -> ChartConfig {
    ChartConfig {
        width: 400.,
        height: 300.,
        margin: Margin {
            left: 40.,
            right: 40.,
            top: 30.,
            bottom: 30.,
        },
        ..Default::default()
    }
}
fn layout(phase: f64) -> LineLayout {
    layout_for(&config(), phase)
}
fn layout_for(config: &ChartConfig, phase: f64) -> LineLayout {
    let mut options = LineOptions {
        x_domain: Some([0., 10.]),
        y_domain: Some([0., 10.]),
        points: false,
        ..Default::default()
    };
    options.axes.y.format = ValueFormat::Fixed(2);
    line_layout_with_style(
        config,
        &[
            Series::new(
                "a",
                vec![
                    Point::new(0., 2.),
                    Point::new(5., 6. + phase),
                    Point::gap(7.),
                    Point::new(10., 8.),
                ],
            ),
            Series::new(
                "b",
                vec![
                    Point::new(0., 8.),
                    Point::new(5., 2.),
                    Point::new(10., 6. + phase),
                ],
            ),
        ],
        &options,
        &LineStyleOptions {
            interactive_sites: true,
            connect_missing: true,
            ..Default::default()
        },
    )
    .unwrap()
}
fn surface(layout: &LineLayout, picking: Picking) -> SurfaceModel {
    SurfaceModel::new_with_interaction(
        layout.scene.clone(),
        SurfaceOptions {
            plot: Some(ClipRect::plot(&config())),
            picking,
            ..Default::default()
        },
        Some(layout.interaction_binding()),
    )
    .unwrap()
}
fn ends(mark: &Mark) -> ([f64; 2], [f64; 2]) {
    let Geometry::Path(ref commands) = mark.geometry else {
        panic!()
    };
    let [PathCommand::Move(a), PathCommand::Line(b)] = commands.as_slice() else {
        panic!()
    };
    ([a.x, a.y], [b.x, b.y])
}

#[test]
fn eleven_actual_nivo_crosshair_types_and_forced_full_slice_lines() {
    use CrosshairType::*;
    let plot = ClipRect {
        x: 10.,
        y: 20.,
        width: 100.,
        height: 80.,
    };
    let anchor = [40., 50.];
    for (kind, expected) in [
        (X, vec![([40., 20.], [40., 100.])]),
        (Y, vec![([10., 50.], [110., 50.])]),
        (
            Cross,
            vec![([40., 20.], [40., 100.]), ([10., 50.], [110., 50.])],
        ),
        (Top, vec![([40., 20.], [40., 50.])]),
        (Bottom, vec![([40., 50.], [40., 100.])]),
        (Left, vec![([10., 50.], [40., 50.])]),
        (Right, vec![([40., 50.], [110., 50.])]),
        (
            TopLeft,
            vec![([40., 20.], [40., 50.]), ([10., 50.], [40., 50.])],
        ),
        (
            TopRight,
            vec![([40., 20.], [40., 50.]), ([40., 50.], [110., 50.])],
        ),
        (
            BottomLeft,
            vec![([40., 50.], [40., 100.]), ([10., 50.], [40., 50.])],
        ),
        (
            BottomRight,
            vec![([40., 50.], [40., 100.]), ([40., 50.], [110., 50.])],
        ),
    ] {
        let options = CrosshairOptions {
            kind,
            ..Default::default()
        };
        let marks = crosshair_marks(plot, anchor, &options, None).unwrap();
        assert_eq!(marks.iter().map(ends).collect::<Vec<_>>(), expected);
        assert!(
            marks
                .iter()
                .all(|m| m.label.is_empty() && m.series.is_empty() && m.clip == Some(plot))
        );
        let forced = crosshair_marks(plot, anchor, &options, Some(SliceAxis::X)).unwrap();
        assert_eq!(forced.len(), 1);
        assert_eq!(ends(&forced[0]), ([40., 20.], [40., 100.]));
    }
    assert_eq!(
        crosshair_marks(
            plot,
            [40., -500.],
            &CrosshairOptions::default(),
            Some(SliceAxis::X)
        )
        .unwrap()
        .len(),
        1,
        "slice's irrelevant Y must not hide its X crosshair"
    );
    assert!(
        crosshair_marks(plot, [-1., 50.], &CrosshairOptions::default(), None)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn typed_sites_keep_raw_values_formats_source_indices_and_invisible_sites() {
    let l = layout(0.);
    assert_eq!(l.sites.len(), 6);
    assert_eq!(l.slices_x.len(), 3);
    assert_eq!(l.slices_y.len(), 3);
    let last = l
        .sites
        .iter()
        .find(|s| s.series_id == "a" && s.raw_x == 10.)
        .unwrap();
    assert_eq!(last.source_index, 3);
    assert_eq!(last.mark_id, "line-point:1:a:3");
    let site = l
        .sites
        .iter()
        .find(|s| s.series_id == "a" && s.raw_x == 5.)
        .unwrap();
    assert_eq!((site.raw_y, site.formatted_y.as_str()), (6., "6.00"));
    assert_eq!(site.series_color, config().theme.color(0));
    for site in &l.sites {
        let mark = l.scene.marks.iter().find(|m| m.id == site.mark_id).unwrap();
        assert!(matches!(mark.geometry, Geometry::Circle { radius: 0., .. }));
        assert_eq!(mark.fill, Some(Color(0, 0, 0, 0)));
        assert!(mark.stroke.is_none());
    }
    l.interaction_binding().validate(&l.scene).unwrap();
    assert_eq!(
        line_with_style(
            &config(),
            &[Series::new(
                "single",
                vec![Point::new(0., 1.), Point::new(10., 2.)]
            )],
            &LineOptions::default(),
            &LineStyleOptions::default()
        )
        .unwrap()
        .legend[0]
            .id,
        "single"
    );
}

#[test]
fn actual_slice_picker_multirows_keyboard_focus_and_hidden_repick() {
    let l = layout(0.);
    let mut m = surface(&l, Picking::SliceX { distance: 400. });
    let site = l
        .sites
        .iter()
        .find(|s| s.series_id == "a" && s.raw_x == 5.)
        .unwrap();
    m.apply(SurfaceInput::Move {
        id: 1,
        point: (site.position[0], 40.),
    })
    .unwrap();
    assert_eq!(m.hovered().len(), 2);
    let active = m.active_interaction(m.hovered()).unwrap();
    assert_eq!(
        active
            .rows
            .iter()
            .map(|r| r.label.as_str())
            .collect::<Vec<_>>(),
        ["b", "a"]
    );
    assert_eq!(active.rows[1].formatted_value.as_deref(), Some("6.00"));
    assert!(active.slice_id.unwrap().starts_with("line-slice:x:"));
    m.set_hovered(vec![site.mark_id.clone()]).unwrap();
    assert_eq!(
        m.active_interaction(m.hovered()).unwrap().rows.len(),
        2,
        "focused single datum expands to its current slice"
    );
    m.apply(SurfaceInput::First).unwrap();
    let selected = m.selected().unwrap().to_owned();
    assert_eq!(m.active_interaction(&[selected]).unwrap().rows.len(), 2);
    m.apply(SurfaceInput::Move {
        id: 1,
        point: (site.position[0], 40.),
    })
    .unwrap();
    m.set_hidden(BTreeSet::from(["a".into()])).unwrap();
    assert_eq!(m.hovered().len(), 1);
    let current = m.active_interaction(m.hovered()).unwrap();
    assert_eq!(current.rows[0].series, "b");
    let marks = m
        .interaction_overlays(m.hovered(), Some(&CrosshairOptions::default()))
        .unwrap();
    assert_eq!(marks.len(), 1);
    assert_eq!(marks[0].id, CROSSHAIR_X_ID);
}

#[test]
fn slice_y_does_not_accidentally_group_x_and_point_nearest_keeps_one_row() {
    let l = layout(0.);
    let a = l
        .sites
        .iter()
        .find(|s| s.series_id == "a" && s.raw_x == 5.)
        .unwrap();
    let mut y = surface(&l, Picking::SliceY { distance: 300. });
    y.apply(SurfaceInput::Move {
        id: 1,
        point: (80., a.position[1]),
    })
    .unwrap();
    let rows = y.active_interaction(y.hovered()).unwrap();
    assert_eq!(rows.rows.len(), 2);
    assert_ne!(rows.rows[0].position[0], rows.rows[1].position[0]);
    let marks = y
        .interaction_overlays(y.hovered(), Some(&CrosshairOptions::default()))
        .unwrap();
    assert_eq!(marks[0].id, CROSSHAIR_Y_ID);
    let mut nearest = surface(&l, Picking::Nearest { distance: 20. });
    nearest
        .apply(SurfaceInput::Move {
            id: 1,
            point: (a.position[0], a.position[1]),
        })
        .unwrap();
    assert_eq!(
        nearest
            .active_interaction(nearest.hovered())
            .unwrap()
            .rows
            .len(),
        1
    );
}

#[test]
fn zoom_crosshair_endpoints_cover_visible_plot_once_and_keep_css_width() {
    let l = layout(0.);
    let mut m = surface(&l, Picking::SliceX { distance: 400. });
    let a = l
        .sites
        .iter()
        .find(|s| s.series_id == "a" && s.raw_x == 5.)
        .unwrap();
    m.apply(SurfaceInput::Wheel {
        point: (a.position[0], a.position[1]),
        delta: -2_f64.ln() / 0.002,
    })
    .unwrap();
    assert_eq!(m.view().scale, 2.);
    let overlay = m
        .interaction_overlays(
            m.hovered(),
            Some(&CrosshairOptions {
                dash: None,
                ..Default::default()
            }),
        )
        .unwrap();
    assert_eq!(overlay.len(), 1);
    assert_eq!(overlay[0].stroke.unwrap().width, 0.5);
    let (first, last) = ends(&overlay[0]);
    let first = m.paint_transform().project((first[0], first[1])).unwrap();
    let last = m.paint_transform().project((last[0], last[1])).unwrap();
    assert_eq!(first.1, m.plot().y);
    assert_eq!(last.1, m.plot().y + m.plot().height);
    assert_eq!(first.0, last.0);
}

#[test]
fn bad_binding_is_transactional_and_legacy_scene_replace_clears_typed_data() {
    let l = layout(0.);
    let mut m = surface(&l, Picking::SliceX { distance: 400. });
    let before = m.scene().clone();
    let original = m.interaction_data().unwrap().clone();
    for change in 0..4 {
        let mut bad = l.interaction_binding();
        match change {
            0 => bad.sites[0].position[0] += 1.,
            1 => bad.sites[0].value = Some(999.),
            2 => bad.sites[0].mark_id = "missing".into(),
            _ => bad.sites.push(bad.sites[0].clone()),
        };
        assert!(
            m.set_scene_with_interaction(l.scene.clone(), Some(bad))
                .is_err()
        );
        assert_eq!(*m.scene(), before);
        assert_eq!(m.interaction_data(), Some(&original));
    }
    let fresh = layout(1.);
    m.set_scene_with_interaction(fresh.scene.clone(), Some(fresh.interaction_binding()))
        .unwrap();
    assert_eq!(
        m.interaction_data()
            .unwrap()
            .sites
            .iter()
            .find(|s| s.mark_id == "line-point:1:a:1")
            .unwrap()
            .value,
        Some(7.)
    );
    m.set_scene(l.scene).unwrap();
    assert!(m.interaction_data().is_none());
}

#[test]
fn motion_frames_keep_business_formats_but_resolve_current_geometry() {
    let old = layout(0.);
    let target = layout(1.);
    let mut m = surface(&old, Picking::Nearest { distance: 400. });
    m.set_interaction_target(&target.scene, Some(target.interaction_binding()))
        .unwrap();
    let id = "line-point:1:a:1".to_owned();
    let start = m.active_interaction(std::slice::from_ref(&id)).unwrap();
    assert_eq!(start.rows[0].value, Some(7.));
    assert_eq!(start.rows[0].formatted_value.as_deref(), Some("7.00"));
    assert_eq!(
        m.controls_scene()
            .marks
            .iter()
            .find(|mark| mark.id == id)
            .unwrap()
            .value,
        7.,
        "accessible controls must use the accepted business value during motion"
    );
    assert_eq!(
        start.rows[0].position,
        old.sites.iter().find(|s| s.mark_id == id).unwrap().position
    );
    let scene = old.scene.interpolate(&target.scene, 0.5).unwrap();
    m.set_scene_preserving_interaction(scene).unwrap();
    let current = m.active_interaction(&[id]).unwrap();
    assert_ne!(current.rows[0].position, start.rows[0].position);
    assert_eq!(current.rows[0].value, Some(7.));
}

#[test]
fn safe_typed_tooltip_prefers_layout_format_and_honors_explicit_override() {
    let l = layout(0.);
    let m = surface(&l, Picking::Nearest { distance: 20. });
    let id = l.sites[0].mark_id.clone();
    let mut active = m.active_interaction(&[id]).unwrap();
    active.rows[0].label = "<script>".into();
    let text = interaction_tooltip_text(&TooltipOptions::default(), &active).unwrap();
    assert_eq!(text, "<script>: 2.00");
    let explicit = interaction_tooltip_text(
        &TooltipOptions {
            value_format: ValueFormat::Fixed(1),
            ..Default::default()
        },
        &active,
    )
    .unwrap();
    assert_eq!(explicit, "<script>: 2.0");
    active.rows[0].value = None;
    assert!(
        interaction_tooltip_text(&TooltipOptions::default(), &active)
            .unwrap()
            .is_empty()
    );
    assert!(
        interaction_tooltip_text(
            &TooltipOptions {
                missing: MissingValue::Placeholder("missing".into()),
                ..Default::default()
            },
            &active
        )
        .unwrap()
        .ends_with("missing")
    );
}

#[test]
fn typed_resize_commits_new_plot_scene_and_binding_as_one_candidate() {
    let initial = layout(0.);
    let mut model = surface(&initial, Picking::SliceX { distance: 400. });
    let old_plot = model.plot();
    let old_scene = model.scene().clone();
    let resized = layout_for(
        &ChartConfig {
            width: 500.,
            ..config()
        },
        1.,
    );
    let mut invalid: SceneInteraction = resized.clone().into();
    invalid.plot.as_mut().unwrap().width = 1000.;
    assert!(model.set_layout(invalid).is_err());
    assert_eq!(model.plot(), old_plot);
    assert_eq!(*model.scene(), old_scene);
    let target = resized
        .sites
        .iter()
        .find(|s| s.series_id == "a" && s.raw_x == 5.)
        .unwrap();
    let point = target.position;
    model.set_layout(resized.into()).unwrap();
    assert_eq!(model.plot().width, 420.);
    model
        .apply(SurfaceInput::Move {
            id: 1,
            point: (point[0], point[1]),
        })
        .unwrap();
    let active = model.active_interaction(model.hovered()).unwrap();
    assert_eq!(
        active.rows.iter().find(|r| r.series == "a").unwrap().value,
        Some(7.)
    );
    let overlay = model
        .interaction_overlays(model.hovered(), Some(&CrosshairOptions::default()))
        .unwrap();
    assert_eq!(ends(&overlay[0]).1[1], 270.);
}

#[test]
fn configured_contour_motion_rejects_incompatible_layout_before_scene_or_binding_commit() {
    use kartis::motion::{
        ContourBindings, ContourMorphOptions, MorphContour, PathMorphOptions, PropertyMotionOptions,
    };
    let l = layout(0.);
    let binding = l.interaction_binding();
    let mut scene = l.scene;
    let mut audit = Mark::new(
        "motion-audit",
        Geometry::Circle {
            x: 100.,
            y: 100.,
            radius: 10.,
        },
        Color(0, 0, 0, 0),
    );
    audit.label.clear();
    scene.marks.push(audit);
    let mut options = PropertyMotionOptions::default();
    options.geometry.contour_morphs.insert(
        "motion-audit".into(),
        ContourMorphOptions {
            sampling: PathMorphOptions {
                samples: 8,
                ..Default::default()
            },
            contours: vec![
                MorphContour {
                    key: "outer".into(),
                    closed: true,
                    collapse: [100., 100.],
                },
                MorphContour {
                    key: "inner".into(),
                    closed: true,
                    collapse: [100., 100.],
                },
            ],
        },
    );
    let explicit = ContourBindings::from([("motion-audit".into(), vec!["outer".into()])]);
    validate_motion_scene(&scene, &options, &explicit).unwrap();
    let mut model = SurfaceModel::new_with_interaction(
        scene.clone(),
        SurfaceOptions {
            picking: Picking::Nearest { distance: 20. },
            ..Default::default()
        },
        Some(binding.clone()),
    )
    .unwrap();
    let p = binding.sites[1].position;
    model
        .apply(SurfaceInput::Move {
            id: 1,
            point: (p[0], p[1]),
        })
        .unwrap();
    let hover = model.hovered().to_vec();
    let mut incompatible = scene.clone();
    incompatible.marks.last_mut().unwrap().geometry = Geometry::Rect {
        x: 90.,
        y: 90.,
        width: 20.,
        height: 20.,
    };
    let candidate = SceneInteraction {
        scene: incompatible,
        interaction: Some(binding.clone()),
        plot: Some(ClipRect::plot(&config())),
    };
    assert!(
        model
            .set_layout_for_motion(candidate, &options, &ContourBindings::new())
            .is_err()
    );
    assert_eq!(*model.scene(), scene);
    assert_eq!(model.interaction_data(), Some(&binding));
    assert_eq!(model.hovered(), hover);
    model
        .set_layout_for_motion(
            SceneInteraction {
                scene,
                interaction: Some(binding),
                plot: Some(ClipRect::plot(&config())),
            },
            &options,
            &explicit,
        )
        .unwrap();
}

fn budget_scene(prefix: &str, count: usize) -> Scene {
    let mut scene = ChartConfig {
        width: 120.,
        height: 100.,
        margin: Margin {
            left: 0.,
            right: 0.,
            top: 0.,
            bottom: 0.,
        },
        ..Default::default()
    }
    .scene()
    .unwrap();
    for i in 0..count {
        let mut mark = Mark::new(
            format!("{prefix}{i}"),
            Geometry::Circle {
                x: 60.,
                y: 50.,
                radius: 0.,
            },
            Color(0, 0, 0, 0),
        );
        mark.label.clear();
        scene.marks.push(mark);
    }
    scene
}
fn assert_union_budget<M: clock::SceneMotion + Clone>(motion: M, target: Scene, accepted: bool) {
    let old = motion.scene().clone();
    let mut candidate = motion.clone();
    candidate.retarget(target).unwrap();
    let count = candidate.scene().marks.len();
    let mut decorated = candidate.scene().clone();
    let result = append_crosshair(
        &mut decorated,
        ClipRect {
            x: 0.,
            y: 0.,
            width: 120.,
            height: 100.,
        },
        [60., 50.],
        &CrosshairOptions {
            kind: CrosshairType::Cross,
            ..Default::default()
        },
        None,
    );
    assert_eq!(result.is_ok(), accepted);
    if accepted {
        assert_eq!(decorated.marks.len(), MAX_MARKS);
    } else {
        assert_eq!(count, MAX_MARKS);
        assert_eq!(decorated.marks.len(), count);
    }
    assert_eq!(
        *motion.scene(),
        old,
        "a rejected candidate cannot change original spring state"
    );
    for dt in [0., 1. / 60., 0.08, 0.4] {
        candidate.tick(dt).unwrap();
        assert!(
            candidate.scene().marks.len() <= count,
            "zero-presence incoming/ghost tracks must already be counted by retarget"
        );
    }
}
#[test]
fn scene_and_property_motion_full_union_requires_overlay_reservation_before_commit() {
    for property in [false, true] {
        let old = budget_scene("old", 10_000);
        let target = budget_scene("new", 10_000);
        if property {
            assert_union_budget(
                motion::PropertyMotion::new(old, Default::default()).unwrap(),
                target,
                false,
            );
        } else {
            assert_union_budget(
                motion::SpringMotion::new(old, Default::default()).unwrap(),
                target,
                false,
            );
        }
    }
}
#[test]
fn scene_and_property_motion_exact_reserved_boundary_is_accepted() {
    for property in [false, true] {
        let old = budget_scene("old", 9_999);
        let target = budget_scene("new", 9_999);
        if property {
            assert_union_budget(
                motion::PropertyMotion::new(old, Default::default()).unwrap(),
                target,
                true,
            );
        } else {
            assert_union_budget(
                motion::SpringMotion::new(old, Default::default()).unwrap(),
                target,
                true,
            );
        }
    }
}

#[test]
fn decorative_layer_rejects_collision_and_limits_before_mutation() {
    let mut scene = config().scene().unwrap();
    append_crosshair(
        &mut scene,
        ClipRect::plot(&config()),
        [200., 150.],
        &CrosshairOptions::default(),
        None,
    )
    .unwrap();
    let before = scene.clone();
    assert!(
        append_crosshair(
            &mut scene,
            ClipRect::plot(&config()),
            [200., 150.],
            &CrosshairOptions::default(),
            None
        )
        .is_err()
    );
    assert_eq!(scene, before);
    let mut fresh = config().scene().unwrap();
    assert!(
        append_crosshair(
            &mut fresh,
            ClipRect::plot(&config()),
            [200., 150.],
            &CrosshairOptions {
                opacity: f64::NAN,
                ..Default::default()
            },
            None
        )
        .is_err()
    );
    assert!(fresh.marks.is_empty());
    let mut full = config().scene().unwrap();
    for i in 0..MAX_MARKS - 1 {
        let mut mark = Mark::new(
            i.to_string(),
            Geometry::Circle {
                x: 100.,
                y: 100.,
                radius: 0.,
            },
            Color(0, 0, 0, 0),
        );
        mark.label.clear();
        full.marks.push(mark);
    }
    let count = full.marks.len();
    assert!(
        append_crosshair(
            &mut full,
            ClipRect::plot(&config()),
            [200., 150.],
            &CrosshairOptions::default(),
            None
        )
        .is_err()
    );
    assert_eq!(full.marks.len(), count);
}

#[cfg(feature = "raster")]
#[test]
fn actual_cpu_crosshair_ink_stops_at_plot_and_has_no_fake_data() {
    use kartis::render::raster::{FontRegistry, RasterOptions, render_raster};
    let mut scene = config().scene().unwrap();
    append_crosshair(
        &mut scene,
        ClipRect::plot(&config()),
        [200., 150.],
        &CrosshairOptions {
            kind: CrosshairType::Cross,
            stroke: glory::render::GraphicStroke {
                color: Color(230, 30, 20, 255),
                width: 2.,
                cap: glory::render::StrokeCap::Butt,
            },
            dash: None,
            opacity: 1.,
        },
        None,
    )
    .unwrap();
    let image = render_raster(&scene, &FontRegistry::new(), &RasterOptions::default()).unwrap();
    let pixel = |x: usize, y: usize| {
        let i = (y * image.width as usize + x) * 4;
        &image.rgba[i..i + 4]
    };
    assert_eq!(pixel(200, 100), [230, 30, 20, 255]);
    assert_eq!(pixel(100, 150), [230, 30, 20, 255]);
    assert_eq!(pixel(200, 20), [255, 255, 255, 255]);
    let mut model = SurfaceModel::new(scene, SurfaceOptions::default()).unwrap();
    model
        .apply(SurfaceInput::Move {
            id: 1,
            point: (200., 100.),
        })
        .unwrap();
    assert!(model.hovered().is_empty());
}
