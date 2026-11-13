use kartis::*;
use std::f64::consts::TAU;
fn config() -> ChartConfig {
    ChartConfig {
        width: 400.,
        height: 400.,
        margin: Margin {
            left: 40.,
            right: 40.,
            top: 40.,
            bottom: 40.,
        },
        ..Default::default()
    }
}
fn axes(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("axis{i}")).collect()
}
fn options() -> RadarLayoutOptions {
    RadarLayoutOptions {
        domain: Some([0., 10.]),
        radius_ratio: 0.8,
        ..Default::default()
    }
}
fn data() -> Vec<RadarSeries> {
    vec![
        RadarSeries::new("A", vec![1., 7., 0., 5.]),
        RadarSeries::new("B", vec![4., 2., 0., 5.]),
        RadarSeries::new("C", vec![3., 9., 0., 1.]),
    ]
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
fn at(center: [f64; 2], radius: f64, angle: f64) -> [f64; 2] {
    [
        center[0] + radius * angle.sin(),
        center[1] - radius * angle.cos(),
    ]
}
#[test]
fn actual_nivo_partition_and_sort_statements_match_metadata() {
    let mut count = 0;
    for row in include_str!("fixtures/radar-slice-angles.csv")
        .lines()
        .skip(2)
    {
        let v = row.split('|').collect::<Vec<_>>();
        let n = v[0].parse().unwrap();
        let rotation = v[1].parse().unwrap();
        let axis: usize = v[2].parse().unwrap();
        let layout = radar_layout(
            &config(),
            &axes(n),
            &[],
            &RadarLayoutOptions {
                rotation,
                ..options()
            },
            &Default::default(),
        )
        .unwrap();
        close(layout.slices[axis].start_angle, v[3].parse().unwrap());
        close(layout.slices[axis].end_angle, v[4].parse().unwrap());
        count += 1;
    }
    assert_eq!(count, 636);
    let cases = [
        [1., 2., 3.],
        [1., 1., 0.],
        [0., -0., 0.],
        [-4., 2., -4.],
        [f64::MAX, -f64::MAX, 1.],
    ];
    let layouts = cases
        .into_iter()
        .map(|values| {
            let data = values
                .into_iter()
                .enumerate()
                .map(|(i, v)| RadarSeries::new(format!("series{i}"), vec![v; 3]))
                .collect::<Vec<_>>();
            radar_layout(
                &config(),
                &axes(3),
                &data,
                &RadarLayoutOptions {
                    domain: Some([-f64::MAX, f64::MAX]),
                    ..options()
                },
                &Default::default(),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    for row in include_str!("fixtures/radar-slice-sort.csv")
        .lines()
        .skip(2)
    {
        let v = row.split('|').collect::<Vec<_>>();
        let c: usize = v[0].parse().unwrap();
        let i: usize = v[1].parse().unwrap();
        assert_eq!(layouts[c].slices[0].rows[i].series_id, v[2]);
    }
}
#[test]
fn true_polar_index_uses_angle_radius_direction_and_half_open_boundaries() {
    for direction in [
        PolarSliceDirection::Clockwise,
        PolarSliceDirection::CounterClockwise,
    ] {
        for rotation in [0., 0.73, -TAU - 0.4, 12345.67] {
            let layout = radar_layout(
                &config(),
                &axes(4),
                &data(),
                &RadarLayoutOptions {
                    rotation,
                    ..options()
                },
                &RadarInteractionOptions {
                    direction,
                    ..Default::default()
                },
            )
            .unwrap();
            let sign = if direction == PolarSliceDirection::Clockwise {
                1.
            } else {
                -1.
            };
            for (i, slice) in layout.slices.iter().enumerate() {
                let p = at(layout.center, layout.radius * 0.95, slice.angle);
                assert_eq!(layout.polar.pick(p[0], p[1]).unwrap().axis_index, i);
                let tip = at(layout.center, layout.radius, slice.angle);
                assert_eq!(layout.polar.pick(tip[0], tip[1]).unwrap().axis_index, i);
                for delta in [-1e-6, 0., 1e-6] {
                    let p = at(layout.center, 50., slice.end_angle + sign * delta);
                    let expected = if delta < 0. { i } else { (i + 1) % 4 };
                    assert_eq!(layout.polar.pick(p[0], p[1]).unwrap().axis_index, expected);
                }
            }
            assert_eq!(
                layout
                    .polar
                    .pick(layout.center[0], layout.center[1])
                    .unwrap()
                    .axis_index,
                0
            );
            assert!(
                layout
                    .polar
                    .pick(layout.center[0] + layout.radius + 1e-6, layout.center[1])
                    .is_none()
            );
            assert!(layout.polar.pick(f64::NAN, 0.).is_none());
        }
    }
}
#[test]
fn zero_values_and_disabled_points_keep_real_zero_ink_sites_and_current_rows() {
    let mut o = options();
    o.points.enabled = false;
    o.points.value_format = ValueFormat::Fixed(2);
    let layout = radar_layout(&config(), &axes(4), &data(), &o, &Default::default()).unwrap();
    let row = &layout.slices[0].rows[0];
    assert_eq!(row.series_id, "B");
    assert_eq!(row.formatted_value, "4.00");
    for s in &layout.slices {
        for r in &s.rows {
            let m = layout
                .scene
                .marks
                .iter()
                .find(|m| m.id == r.point_mark_id)
                .unwrap();
            assert!(matches!(m.geometry, Geometry::Circle { radius: 0., .. }));
            assert_eq!(m.fill, Some(Color(0, 0, 0, 0)));
            assert!(m.stroke.is_none());
            assert_eq!(m.value, r.raw_value);
            assert_eq!(layout.polar.slice_for_mark(&m.id).unwrap().id, s.id);
        }
    }
    assert_eq!(
        layout.slices[2]
            .rows
            .iter()
            .map(|r| r.series_id.as_str())
            .collect::<Vec<_>>(),
        ["C", "B", "A"]
    );
    assert!(
        layout.slices[2]
            .rows
            .iter()
            .all(|r| r.position == layout.center)
    );
    layout
        .interaction_binding()
        .validate(&layout.scene)
        .unwrap();
}
#[test]
fn glyph_shapes_keep_standard_ids_and_bind_actual_path_ink() {
    for shape in [
        RadarPointShape::Circle,
        RadarPointShape::Square,
        RadarPointShape::Diamond,
        RadarPointShape::Triangle,
        RadarPointShape::Cross,
    ] {
        let mut o = options();
        o.points.shape = shape;
        let layout = radar_layout(&config(), &axes(4), &data(), &o, &Default::default()).unwrap();
        layout
            .interaction_binding()
            .validate(&layout.scene)
            .unwrap();
        for r in &layout.slices[0].rows {
            assert!(r.point_mark_id.starts_with("radar-point:"));
            let point = layout
                .scene
                .marks
                .iter()
                .find(|m| m.id == r.point_mark_id)
                .unwrap();
            assert_eq!(r.glyph_mark_id.as_ref(), Some(&r.point_mark_id));
            assert!(!point.label.is_empty());
            if shape == RadarPointShape::Circle {
                assert!(matches!(point.geometry, Geometry::Circle { radius, .. } if radius>0.));
            } else {
                assert!(matches!(point.geometry, Geometry::Path(_)));
                assert!(layout.anchors.contains_key(&point.id));
            }
            assert_eq!(
                layout
                    .scene
                    .marks
                    .iter()
                    .filter(|m| m.id == point.id)
                    .count(),
                1
            );
        }
    }
}
#[test]
fn hidden_relayout_and_current_values_keep_domains_palette_and_axis_ids() {
    let mut o = options();
    o.domain = None;
    let old = radar_layout(&config(), &axes(4), &data(), &o, &Default::default()).unwrap();
    o.hidden.insert("B".into());
    let mut d = data();
    d[0].values[0] = 8.;
    let current = radar_layout(&config(), &axes(4), &d, &o, &Default::default()).unwrap();
    assert_eq!(old.domain, current.domain);
    assert_eq!(old.slices[0].id, current.slices[0].id);
    assert_eq!(current.slices[0].rows[0].series_id, "A");
    assert_eq!(current.slices[0].rows[0].raw_value, 8.);
    assert!(
        current
            .slices
            .iter()
            .all(|s| s.rows.iter().all(|r| r.series_id != "B"))
    );
    assert!(current.scene.legend.iter().any(|l| l.id == "B"));
    let stale = old.interaction_binding();
    assert!(stale.validate(&current.scene).is_err());
    let old_color = old.slices[0]
        .rows
        .iter()
        .find(|r| r.series_id == "A")
        .unwrap()
        .color;
    assert_eq!(old_color, current.slices[0].rows[0].color);
}
#[test]
fn malformed_index_and_frame_inputs_reject_without_changing_existing_binding() {
    let layout = radar_layout(
        &config(),
        &axes(4),
        &data(),
        &options(),
        &Default::default(),
    )
    .unwrap();
    let original = layout.polar.clone();
    let mut slices = original.slices().to_vec();
    let duplicate = slices[1].member_ids[0].clone();
    slices[0].member_ids.push(duplicate);
    assert!(
        PolarSliceIndex::new(layout.center, layout.radius, 0., Default::default(), slices).is_err()
    );
    let mut slices = original.slices().to_vec();
    slices[0].anchor[0] += 2.;
    assert!(
        PolarSliceIndex::new(layout.center, layout.radius, 0., Default::default(), slices).is_err()
    );
    assert_eq!(layout.polar, original);
    let mut tiny_slices = original.slices().to_vec();
    for slice in &mut tiny_slices {
        slice.anchor = [1e6, 1e6];
    }
    let tiny =
        PolarSliceIndex::new([1e6, 1e6], 1e-100, 0., Default::default(), tiny_slices).unwrap();
    assert!(tiny.pick(1e6, 1e6).is_some());
    assert!(
        tiny.pick(1e6 + 1e-9, 1e6).is_none(),
        "absolute coordinate ulps must not inflate a tiny radius"
    );
    assert!(
        radar_layout(
            &config(),
            &axes(4),
            &data(),
            &options(),
            &RadarInteractionOptions {
                spoke: Some(glory::render::GraphicStroke {
                    color: Color(0, 0, 0, 255),
                    width: f64::NAN,
                    cap: glory::render::StrokeCap::Butt
                }),
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[cfg(feature = "raster")]
#[test]
fn zero_sites_do_not_paint_and_spoke_is_true_shared_geometry() {
    use render::raster::*;
    let mut o = options();
    o.points.enabled = false;
    o.axis_labels = false;
    let layout = radar_layout(&config(), &axes(4), &data(), &o, &Default::default()).unwrap();
    let base = radar_with_options(&config(), &axes(4), &data(), &o).unwrap();
    let render = |s: &Scene| render_raster(s, &FontRegistry::new(), &Default::default()).unwrap();
    let plain = render(&base);
    assert_eq!(plain.rgba, render(&layout.scene).rgba);
    let mut scene = layout.scene.clone();
    let mut spoke = Mark::new(
        "probe-spoke",
        layout.polar.spoke(&layout.slices[0].id).unwrap(),
        Color(220, 10, 20, 255),
    );
    spoke.label.clear();
    spoke.fill = None;
    spoke.stroke = layout.spoke;
    scene.marks.push(spoke);
    let painted = render(&scene);
    assert_ne!(plain.rgba, painted.rgba);
    let i = (120 * 400 + 200) * 4;
    assert_ne!(&plain.rgba[i..i + 4], &painted.rgba[i..i + 4]);
}

#[test]
fn shared_surface_polar_membership_keyboard_hidden_zoom_and_atomic_target() {
    use kartis::surface::*;
    use std::collections::BTreeSet;
    let mut o = options();
    o.points.enabled = false;
    let layout = radar_layout(&config(), &axes(4), &data(), &o, &Default::default()).unwrap();
    let mut model = SurfaceModel::new_with_interaction(
        layout.scene.clone(),
        SurfaceOptions::default(),
        Some(layout.interaction_binding()),
    )
    .unwrap();
    // This angle lies in axis2's sector even though all three sites are at
    // radius0, far from the probe. Nearest-point behavior would be incorrect.
    let probe = at(
        layout.center,
        layout.radius * 0.75,
        std::f64::consts::PI + 0.15,
    );
    model
        .apply(SurfaceInput::Move {
            id: 1,
            point: (probe[0], probe[1]),
        })
        .unwrap();
    let active = model.active_interaction(model.hovered()).unwrap();
    assert_eq!(
        active.slice_id.as_deref(),
        Some(layout.slices[2].id.as_str())
    );
    assert_eq!(
        active
            .rows
            .iter()
            .map(|r| r.series.as_str())
            .collect::<Vec<_>>(),
        ["C", "B", "A"]
    );
    assert!(
        active
            .rows
            .iter()
            .all(|r| r.value == Some(0.) && r.position == layout.center)
    );
    let overlays = model.interaction_overlays(model.hovered(), None).unwrap();
    assert_eq!(overlays.len(), 1);
    assert_eq!(overlays[0].id, POLAR_SPOKE_ID);
    assert_eq!(
        overlays[0].geometry,
        layout.polar.spoke(&layout.slices[2].id).unwrap()
    );
    model.set_hidden(BTreeSet::from(["B".into()])).unwrap();
    assert_eq!(
        model
            .active_interaction(model.hovered())
            .unwrap()
            .rows
            .iter()
            .map(|r| r.series.as_str())
            .collect::<Vec<_>>(),
        ["C", "A"]
    );
    model.apply(SurfaceInput::Clear).unwrap();
    for (input, index) in [
        (SurfaceInput::First, 0),
        (SurfaceInput::Next, 1),
        (SurfaceInput::Last, 3),
        (SurfaceInput::Previous, 2),
    ] {
        model.apply(input).unwrap();
        let active = model
            .active_interaction(&[model.selected().unwrap().to_owned()])
            .unwrap();
        assert_eq!(
            active.slice_id.as_deref(),
            Some(layout.slices[index].id.as_str())
        );
        assert_eq!(active.rows.len(), 2);
    }
    model.apply(SurfaceInput::Clear).unwrap();
    model
        .apply(SurfaceInput::Wheel {
            point: (layout.center[0], layout.center[1]),
            delta: -100.,
        })
        .unwrap();
    assert!(model.view().scale > 1.);
    let screen = model.view().project((probe[0], probe[1])).unwrap();
    model
        .apply(SurfaceInput::Move {
            id: 1,
            point: screen,
        })
        .unwrap();
    assert_eq!(
        model
            .active_interaction(model.hovered())
            .unwrap()
            .slice_id
            .as_deref(),
        Some(layout.slices[2].id.as_str())
    );
    let old_scene = model.scene().clone();
    let old_hover = model.hovered().to_vec();
    let mut changed = data();
    changed[0].values[0] = 8.;
    let current = radar_layout(&config(), &axes(4), &changed, &o, &Default::default()).unwrap();
    assert!(
        model
            .set_scene_with_interaction(current.scene.clone(), Some(layout.interaction_binding()))
            .is_err()
    );
    assert_eq!(model.scene(), &old_scene);
    assert_eq!(model.hovered(), old_hover);
    model.set_layout(current.clone().into()).unwrap();
    model.apply(SurfaceInput::First).unwrap();
    let active = model
        .active_interaction(&[model.selected().unwrap().to_owned()])
        .unwrap();
    assert_eq!(
        (active.rows[0].series.as_str(), active.rows[0].value),
        ("A", Some(8.))
    );
    model.set_scene(current.scene).unwrap();
    assert!(model.interaction_data().is_none());
    assert!(
        model
            .interaction_overlays(model.hovered(), None)
            .unwrap()
            .is_empty()
    );
}
