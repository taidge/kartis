use kartis::{annotations::Brush, mesh::PointMesh, surface::*, *};
fn shader_only() -> Scene {
    let mut scene = ChartConfig {
        width: 200.,
        height: 160.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    let mut mark = Mark::new(
        "shader",
        Geometry::Circle {
            x: 80.,
            y: 70.,
            radius: 12.,
        },
        Color(0, 0, 0, 0),
    );
    mark.fill = None;
    mark.series = "series".into();
    mark.fill_paint = Some(FillPaint::Linear {
        start: [60., 70.],
        end: [100., 70.],
        stops: vec![
            GradientStop::new(0., Color(255, 0, 0, 255)),
            GradientStop::new(1., Color(0, 0, 255, 255)),
        ],
    });
    scene.marks.push(mark);
    scene
}
#[test]
fn gradient_only_visible_marks_participate_in_mesh_and_brush() {
    let scene = shader_only();
    scene.validate().unwrap();
    assert_eq!(scene.hit_test(80., 70.).unwrap().id, "shader");
    let mesh = PointMesh::from_scene(&scene).unwrap();
    assert_eq!(mesh.points().len(), 1);
    assert_eq!(mesh.nearest(90., 70., 20.).unwrap().id, "shader");
    let brush = Brush::from_points((60., 50.), (100., 90.)).unwrap();
    assert_eq!(brush.select(&scene).unwrap(), ["shader"]);
    let mut outside = scene.clone();
    outside.marks[0].clip = Some(ClipRect {
        x: 0.,
        y: 0.,
        width: 50.,
        height: 160.,
    });
    assert!(PointMesh::from_scene(&outside).unwrap().points().is_empty());
    assert!(brush.select(&outside).unwrap().is_empty());
    assert!(
        PointMesh::from_scene(&scene.without_series(&["series".into()].into_iter().collect()))
            .unwrap()
            .points()
            .is_empty()
    );
    render::to_svg(&scene).unwrap();
    render::to_canvas_script(&scene, "gradient").unwrap();
}
#[test]
fn shader_only_mark_is_pickable_through_surface_mesh_and_both_slice_axes() {
    for picking in [
        Picking::Nearest { distance: 30. },
        Picking::SliceX { distance: 30. },
        Picking::SliceY { distance: 30. },
    ] {
        let mut surface = SurfaceModel::new(
            shader_only(),
            SurfaceOptions {
                picking,
                ..Default::default()
            },
        )
        .unwrap();
        surface
            .apply(SurfaceInput::Move {
                id: 1,
                point: (85., 75.),
            })
            .unwrap();
        assert_eq!(surface.hovered(), ["shader"], "{picking:?}");
        surface
            .apply(SurfaceInput::Down {
                id: 1,
                point: (85., 75.),
                brush: false,
            })
            .unwrap();
        surface
            .apply(SurfaceInput::Up {
                id: 1,
                point: (85., 75.),
            })
            .unwrap();
        assert_eq!(surface.selected(), Some("shader"));
    }
}
#[test]
fn grouped_bars_do_not_sum_independent_extreme_values_for_plain_or_rounded_styles() {
    let c = ChartConfig::default();
    let categories = vec!["A".into()];
    for sign in [-1., 1.] {
        let raw = sign * 0.75 * f64::MAX;
        let data = [
            BarSeries::new("one", vec![Some(raw)]),
            BarSeries::new("two", vec![Some(raw)]),
        ];
        for corner_radius in [0., 16384.] {
            for horizontal in [false, true] {
                let scene = bar(
                    &c,
                    &categories,
                    &data,
                    &BarOptions {
                        layout: BarLayout::Grouped,
                        value_domain: Some(if sign > 0. {
                            [0., f64::MAX]
                        } else {
                            [-f64::MAX, 0.]
                        }),
                        corner_radius,
                        horizontal,
                        ..Default::default()
                    },
                )
                .unwrap();
                let marks = scene
                    .marks
                    .iter()
                    .filter(|m| !m.series.is_empty())
                    .collect::<Vec<_>>();
                assert_eq!(marks.len(), 2);
                assert_eq!(
                    marks.iter().map(|m| m.series.as_str()).collect::<Vec<_>>(),
                    ["one", "two"]
                );
                assert!(
                    marks
                        .iter()
                        .all(|m| m.value.to_bits() == raw.to_bits() && m.label.contains('A'))
                );
                assert_eq!(marks[0].id, "bar-0-0");
                assert_eq!(marks[1].id, "bar-1-0");
                assert_eq!(scene.legend.len(), 2);
                scene.validate().unwrap();
                render::to_svg(&scene).unwrap();
                render::to_canvas_script(&scene, "bars").unwrap();
            }
        }
    }
}
