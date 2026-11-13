use glory::render::{GraphicPoint, GraphicStroke, PathCommand, StrokeCap};
use kartis::{
    surface::{Picking, SurfaceInput, SurfaceModel, SurfaceOptions},
    *,
};

fn config() -> ChartConfig {
    ChartConfig {
        width: 400.,
        height: 320.,
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 20.,
            bottom: 20.,
        },
        ..Default::default()
    }
}

fn stroke() -> GraphicStroke {
    GraphicStroke {
        color: Color(30, 40, 50, 255),
        width: 3.,
        cap: StrokeCap::Butt,
    }
}

fn pie() -> PieLayout {
    let mut options = PieLayoutOptions {
        fit: PieFit::Centered,
        ..Default::default()
    };
    options.pie.inner_radius = 0.45;
    options.pie.pad_angle = 0.06;
    options.pie.labels = false;
    options.pie.hidden.insert("hidden".into());
    options.pie.style.corner_radius = 8.;
    options.pie.style.active_ids.insert("a".into());
    options.pie.style.active_inner_radius_offset = 10.;
    options.pie.style.active_outer_radius_offset = 20.;
    options.pie.style.link_labels.enabled = true;
    let mut rows = vec![
        RadialDatum::new("a", 2.),
        RadialDatum::new("b", 3.),
        RadialDatum::new("hidden", 4.),
    ];
    rows[0].label = "Alpha".into();
    pie_layout(&config(), &rows, &options).unwrap()
}

fn radial(styled: bool) -> StackedRadialBarLayout {
    let categories = vec!["a".into(), "b".into(), "hidden".into(), "zero".into()];
    let rows = vec![
        BarSeries::new("first", vec![Some(1.), Some(3.), None, Some(0.)]),
        BarSeries::new("second", vec![Some(2.), None, Some(5.), Some(0.)]),
    ];
    let options = StackedRadialBarOptions {
        start_angle: 2.7,
        end_angle: -0.3,
        pad_angle: 0.05,
        value_format: ValueFormat::Fixed(2),
        hidden: ["hidden".into()].into_iter().collect(),
        ..Default::default()
    };
    if styled {
        layout_stacked_radial_bar_with_arc_style(
            &config(),
            &categories,
            &rows,
            &options,
            &StackedRadialArcStyle {
                corner_radius: 7.,
                pad_radius: PiePadRadius::Fixed(90.),
            },
        )
        .unwrap()
    } else {
        layout_stacked_radial_bar(&config(), &categories, &rows, &options).unwrap()
    }
}

fn translate(scene: &mut Scene, id: &str, dx: f64, dy: f64) {
    let mark = scene.marks.iter_mut().find(|m| m.id == id).unwrap();
    let Geometry::Path(path) = &mut mark.geometry else {
        panic!("expected real arc Path")
    };
    let move_point = |p: &mut GraphicPoint| {
        p.x += dx;
        p.y += dy;
    };
    for command in path {
        match command {
            PathCommand::Move(p) | PathCommand::Line(p) => move_point(p),
            PathCommand::Cubic { first, second, end } => {
                move_point(first);
                move_point(second);
                move_point(end);
            }
            PathCommand::Close => {}
        }
    }
}

fn assert_nearest_and_live_anchor(candidate: SceneInteraction, id: &str, value: f64) {
    candidate.validate().unwrap();
    let binding = candidate.interaction.as_ref().unwrap();
    let site = binding
        .sites
        .iter()
        .find(|site| site.mark_id == id)
        .unwrap();
    let initial = site.position;
    let mut surface = SurfaceModel::new(
        candidate.scene.clone(),
        SurfaceOptions {
            picking: Picking::Nearest { distance: 12. },
            ..Default::default()
        },
    )
    .unwrap();
    surface.set_layout(candidate.clone()).unwrap();
    surface
        .apply(SurfaceInput::Move {
            id: 1,
            point: (initial[0], initial[1]),
        })
        .unwrap();
    assert_eq!(surface.hovered(), [id]);
    let active = surface.active_interaction(surface.hovered()).unwrap();
    assert_eq!(active.anchor, initial);
    assert_eq!(active.rows[0].value, Some(value));
    let mut live = candidate.scene.clone();
    translate(&mut live, id, 13., -7.);
    let mark = live.marks.iter().find(|m| m.id == id).unwrap();
    let resolved = binding.anchors[id].resolve(&mark.geometry).unwrap();
    assert!((resolved[0] - initial[0] - 13.).abs() < 1e-10);
    assert!((resolved[1] - initial[1] + 7.).abs() < 1e-10);
    surface
        .set_interaction_frame(&candidate.scene, candidate.interaction.clone(), live)
        .unwrap();
    surface
        .apply(SurfaceInput::Move {
            id: 1,
            point: (resolved[0], resolved[1]),
        })
        .unwrap();
    let active = surface.active_interaction(surface.hovered()).unwrap();
    assert_eq!(active.anchor, resolved);
    assert_eq!(active.rows[0].value, Some(value));
    assert!(
        binding.anchors[id]
            .resolve(&Geometry::Circle {
                x: 0.,
                y: 0.,
                radius: 1.
            })
            .is_err()
    );
}

#[test]
fn pie_binding_uses_raw_data_active_radii_and_the_real_arc_midpoint() {
    let layout = pie();
    let before = layout.scene.clone();
    let binding = layout.interaction_binding().unwrap();
    assert_eq!(binding.sites.len(), 2);
    assert_eq!(binding.anchors.len(), 2);
    assert_eq!(layout.scene, before);
    assert!(binding.sites.iter().all(|site| site.mark_id != "hidden"));
    let measurements = layout
        .link_candidates
        .iter()
        .map(|candidate| {
            candidate.measured(LayoutRect {
                x: -8.,
                y: -8.,
                width: 16.,
                height: 12.,
            })
        })
        .collect::<Vec<_>>();
    let placed = layout
        .place_link_labels(&measurements, &PieLinkPlacementOptions::default())
        .unwrap();
    assert_eq!(placed.interaction_binding().unwrap(), binding);
    for site in &binding.sites {
        let slice = layout.slices.iter().find(|s| s.id == site.mark_id).unwrap();
        let angle = slice.start_angle + (slice.end_angle - slice.start_angle) / 2.;
        let radius = (slice.inner_radius + slice.outer_radius) / 2.;
        assert_eq!(
            site.position,
            [
                layout.center[0] + radius * angle.sin(),
                layout.center[1] - radius * angle.cos()
            ]
        );
        assert_eq!(site.value, Some(slice.value));
        let mark = &layout.scene.marks[slice.mark_index.unwrap()];
        assert!(matches!(mark.geometry, Geometry::Path(_)));
        assert_eq!(
            binding.anchors[&site.mark_id]
                .resolve(&mark.geometry)
                .unwrap(),
            site.position
        );
    }
    let a = binding.sites.iter().find(|s| s.mark_id == "a").unwrap();
    assert_eq!(a.label, "Alpha");
    let slice = layout.slices.iter().find(|s| s.id == "a").unwrap();
    assert_eq!(slice.outer_radius, layout.radius + 20.);
    assert_eq!(slice.inner_radius, layout.inner_radius - 10.);
    let angle = (slice.start_angle + slice.end_angle) / 2.;
    let r = layout.radius + 8.;
    let outside = [
        layout.center[0] + r * angle.sin(),
        layout.center[1] - r * angle.cos(),
    ];
    assert!(!layout.bounds.contains(outside[0], outside[1]));
    assert_eq!(
        layout.scene.hit_test(outside[0], outside[1]).unwrap().id,
        "a"
    );
    let candidate = layout.clone().into_scene_interaction().unwrap();
    assert!(candidate.plot.is_none());
    let mut surface = SurfaceModel::new(candidate.scene.clone(), Default::default()).unwrap();
    surface.set_layout(candidate).unwrap();
    surface
        .apply(SurfaceInput::Move {
            id: 1,
            point: (outside[0], outside[1]),
        })
        .unwrap();
    assert_eq!(surface.hovered(), ["a"]);
    assert_nearest_and_live_anchor(layout.into_scene_interaction().unwrap(), "a", 2.);
}

#[test]
fn radial_binding_excludes_tracks_axes_hidden_zero_and_sparse_cells() {
    for styled in [false, true] {
        let layout = radial(styled);
        let before = layout.scene.clone();
        let binding = layout.interaction_binding().unwrap();
        assert_eq!(layout.scene, before);
        assert_eq!(binding.sites.len(), 3);
        assert!(layout.scene.marks.len() > binding.sites.len());
        assert!(
            layout
                .segments
                .iter()
                .any(|s| s.category == "hidden" && !s.visible)
        );
        assert!(layout.segments.iter().any(|s| s.value == 0. && !s.visible));
        for site in &binding.sites {
            let segment = layout
                .segments
                .iter()
                .find(|s| s.mark_id == site.mark_id)
                .unwrap();
            let expected = layout.frame.point(
                (segment.inner_radius + segment.outer_radius) / 2.,
                segment.angles[0] + (segment.angles[1] - segment.angles[0]) / 2.,
            );
            assert_eq!(site.position, expected);
            assert_eq!(site.series, segment.category);
            assert_eq!(site.value, Some(segment.value));
            assert_eq!(
                site.formatted_value.as_deref(),
                Some(
                    ValueFormat::Fixed(2)
                        .format(segment.value)
                        .unwrap()
                        .as_str()
                )
            );
            assert!(site.label.contains(&segment.ring_id));
            let mark = layout
                .scene
                .marks
                .iter()
                .find(|m| m.id == site.mark_id)
                .unwrap();
            assert!(matches!(mark.geometry, Geometry::Path(_)));
            assert_eq!(
                layout.scene.hit_test(expected[0], expected[1]).unwrap().id,
                site.mark_id
            );
        }
        let first = &binding.sites[0];
        assert_nearest_and_live_anchor(
            layout.clone().into_scene_interaction().unwrap(),
            &first.mark_id,
            first.value.unwrap(),
        );
    }
}

#[test]
fn real_collapsed_radial_strokes_retain_their_raw_datum_without_fill_sites() {
    let rows = [
        RadialDatum::new("zero", 0.),
        RadialDatum::new("positive", 1.),
    ];
    let mut options = PieLayoutOptions::default();
    options.pie.inner_radius = 0.4;
    options.pie.pad_angle = 0.4;
    options.pie.labels = false;
    let fill = pie_layout(&config(), &rows, &options).unwrap();
    assert!(
        fill.slices
            .iter()
            .find(|s| s.id == "zero")
            .unwrap()
            .outer_collapsed
    );
    assert!(
        fill.interaction_binding()
            .unwrap()
            .sites
            .iter()
            .all(|s| s.mark_id != "zero")
    );
    options.pie.style.border = Some(stroke());
    let outlined = pie_layout(&config(), &rows, &options).unwrap();
    let binding = outlined.interaction_binding().unwrap();
    let zero = binding.sites.iter().find(|s| s.mark_id == "zero").unwrap();
    assert_eq!(zero.value, Some(0.));
    assert_eq!(zero.color, stroke().color);
    assert_eq!(
        outlined
            .scene
            .hit_test(zero.position[0], zero.position[1])
            .unwrap()
            .id,
        "zero"
    );

    let categories = vec!["tiny".into(), "large".into()];
    let rows = vec![BarSeries::new("ring", vec![Some(1.), Some(5.)])];
    let mut options = StackedRadialBarOptions {
        start_angle: 0.,
        end_angle: 0.28,
        pad_angle: 0.1,
        inner_radius: 0.4,
        padding: 0.,
        ..Default::default()
    };
    let style = StackedRadialArcStyle {
        corner_radius: 10.,
        ..Default::default()
    };
    let fill =
        layout_stacked_radial_bar_with_arc_style(&config(), &categories, &rows, &options, &style)
            .unwrap();
    assert!(
        fill.interaction_binding()
            .unwrap()
            .sites
            .iter()
            .all(|s| s.series != "tiny")
    );
    options.border = Some(stroke());
    let outlined =
        layout_stacked_radial_bar_with_arc_style(&config(), &categories, &rows, &options, &style)
            .unwrap();
    let binding = outlined.interaction_binding().unwrap();
    let tiny = binding.sites.iter().find(|s| s.series == "tiny").unwrap();
    assert_eq!(tiny.value, Some(1.));
    assert_eq!(tiny.color, stroke().color);
    let target = tiny.mark_id.clone();
    let point = tiny.position;
    let candidate = outlined.into_scene_interaction().unwrap();
    let mut surface = SurfaceModel::new(candidate.scene.clone(), Default::default()).unwrap();
    surface.set_layout(candidate).unwrap();
    surface
        .apply(SurfaceInput::Move {
            id: 1,
            point: (point[0], point[1]),
        })
        .unwrap();
    assert_eq!(surface.hovered(), [target]);
}

#[test]
fn transparent_or_zero_opacity_paints_are_filtered_and_custom_items_stay_decorative() {
    let mut layout = pie();
    layout.scene.marks[0].opacity = 0.;
    assert!(
        layout
            .interaction_binding()
            .unwrap()
            .sites
            .iter()
            .all(|s| s.mark_id != "a")
    );
    let transparent = FillPaint::Linear {
        start: [0., 0.],
        end: [1., 0.],
        stops: vec![
            GradientStop::new(0., Color(0, 0, 0, 0)),
            GradientStop::new(1., Color(0, 0, 0, 0)),
        ],
    };
    layout.scene.marks[1].fill_paint = Some(transparent);
    assert!(layout.interaction_binding().unwrap().sites.is_empty());
    let mut layout = radial(true);
    for mark in &mut layout.scene.marks {
        if !mark.series.is_empty() {
            mark.fill = Some(Color(1, 2, 3, 0));
            mark.fill_paint = Some(FillPaint::Pattern(PatternFill {
                shape: PatternShape::Dots {
                    diameter: 0.,
                    stagger: false,
                },
                background: Color(0, 0, 0, 0),
                foreground: Color(0, 0, 0, 255),
                ..Default::default()
            }));
        }
    }
    assert!(layout.interaction_binding().unwrap().sites.is_empty());
}

#[test]
fn mutated_public_metadata_or_datum_paths_reject_bindings_without_panics() {
    let pie = pie();
    let original = pie.clone();
    let mut changed = pie.clone();
    changed.slices[0].value += 1.;
    assert!(changed.interaction_binding().is_err());
    let mut changed = pie.clone();
    changed.slices[0].mark_index = Some(usize::MAX);
    assert!(changed.interaction_binding().is_err());
    let mut changed = pie.clone();
    changed.slices[0].start_angle += 0.01;
    assert!(changed.interaction_binding().is_err());
    let mut changed = pie.clone();
    changed.center[0] += 1.;
    assert!(changed.interaction_binding().is_err());
    let mut changed = pie.clone();
    translate(&mut changed.scene, "a", 1., 0.);
    assert!(changed.interaction_binding().is_err());
    let mut changed = pie.clone();
    changed.scene.marks[0].value += 1.;
    assert!(changed.interaction_binding().is_err());
    assert_eq!(pie, original);

    let radial = radial(true);
    let original = radial.clone();
    let mut changed = radial.clone();
    changed.segments[0].value += 1.;
    assert!(changed.interaction_binding().is_err());
    let mut changed = radial.clone();
    changed.segments[0].visible = false;
    assert!(changed.interaction_binding().is_err());
    let mut changed = radial.clone();
    changed.rings[0].total += 1.;
    assert!(changed.interaction_binding().is_err());
    let mut changed = radial.clone();
    changed.frame.center[0] += 1.;
    assert!(changed.interaction_binding().is_err());
    let mut changed = radial.clone();
    changed.domain[1] += 1.;
    assert!(changed.interaction_binding().is_err());
    let id = radial
        .segments
        .iter()
        .find(|s| s.visible)
        .unwrap()
        .mark_id
        .clone();
    let mut changed = radial.clone();
    translate(&mut changed.scene, &id, 1., 0.);
    assert!(changed.interaction_binding().is_err());
    let mut changed = radial.clone();
    changed.scene.marks.retain(|m| m.id != id);
    assert!(changed.interaction_binding().is_err());
    assert_eq!(radial, original);
}
