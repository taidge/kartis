use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use kartis::surface::{Picking, SurfaceInput, SurfaceModel, SurfaceOptions};
use kartis::*;

const INK: Color = Color(174, 34, 97, 255);
const PALETTE: Color = Color(41, 125, 176, 255);

fn config() -> ChartConfig {
    ChartConfig {
        width: 120.,
        height: 120.,
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 20.,
            bottom: 20.,
        },
        ..Default::default()
    }
}

fn options() -> LineOptions {
    let mut options = LineOptions {
        x_domain: Some([0., 10.]),
        y_domain: Some([0., 10.]),
        ..Default::default()
    };
    options.axes.x.visible = false;
    options.axes.x.grid = false;
    options.axes.y.visible = false;
    options.axes.y.grid = false;
    options
}

fn isolated_style() -> LineStyleOptions {
    LineStyleOptions {
        series: LineSeriesStyle {
            width: 0.,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> Geometry {
    Geometry::Path(vec![
        C::Move(P::new(x, y)),
        C::Line(P::new(x + width, y)),
        C::Line(P::new(x + width, y + height)),
        C::Line(P::new(x, y + height)),
        C::Close,
    ])
}

fn offsets(value: f64) -> LineLayout {
    line_layout_with_point_factory(
        &config(),
        &[Series::new(
            "s",
            vec![Point::new(5., value), Point::new(10., 8.)],
        )],
        &options(),
        &isolated_style(),
        None,
        |point| {
            let [x, y] = point.position;
            if point.source_index == 0 {
                let mut geometry = point.inherit(rect(x + 16., y - 6., 12., 12.));
                geometry.fill = None;
                geometry.stroke = Some(GraphicStroke {
                    color: INK,
                    width: 2.,
                    cap: StrokeCap::Butt,
                });
                Ok(geometry)
            } else {
                let mut geometry = point.inherit(rect(x - 6., y - 5., 18., 10.));
                geometry.fill = Some(INK);
                Ok(geometry)
            }
        },
    )
    .unwrap()
}

fn surface(layout: &LineLayout, picking: Picking) -> SurfaceModel {
    SurfaceModel::new_with_interaction(
        layout.scene.clone(),
        SurfaceOptions {
            picking,
            plot: Some(layout.plot),
            ..Default::default()
        },
        Some(layout.interaction_binding()),
    )
    .unwrap()
}

fn hover(model: &mut SurfaceModel, point: (f64, f64)) -> &[String] {
    model.apply(SurfaceInput::Move { id: 1, point }).unwrap();
    model.hovered()
}

#[test]
fn exact_pick_uses_authored_ink_while_nearest_and_slices_use_data_positions() {
    let layout = offsets(5.);
    let first = &layout.sites[0];
    let second = &layout.sites[1];
    assert_eq!(first.position, [60., 60.]);
    assert_eq!(second.position, [100., 36.]);
    let mark = layout
        .scene
        .marks
        .iter()
        .find(|mark| mark.id == first.mark_id)
        .unwrap();
    assert_eq!(mark.geometry, rect(76., 54., 12., 12.));
    assert_eq!(mark.clip, Some(layout.plot));
    let mut exact = surface(&layout, Picking::Geometry);
    assert_eq!(
        hover(&mut exact, (76., 60.)),
        std::slice::from_ref(&first.mark_id)
    );
    let active = exact.active_interaction(exact.hovered()).unwrap();
    assert_eq!(active.anchor, first.position);
    assert_eq!(active.rows[0].color, first.series_color);
    assert!(hover(&mut exact, (82., 60.)).is_empty());
    assert!(hover(&mut exact, (60., 60.)).is_empty());
    assert_eq!(
        hover(&mut exact, (97., 36.)),
        std::slice::from_ref(&second.mark_id)
    );
    assert!(layout.scene.hit_test(103., 36.).is_none());

    let mut nearest = surface(&layout, Picking::Nearest { distance: 1. });
    assert_eq!(
        hover(&mut nearest, (60., 60.)),
        std::slice::from_ref(&first.mark_id)
    );
    assert!(hover(&mut nearest, (76., 60.)).is_empty());
    let mut slice = surface(&layout, Picking::SliceX { distance: 1. });
    assert_eq!(
        hover(&mut slice, (60., 73.)),
        std::slice::from_ref(&first.mark_id)
    );
    assert_eq!(layout.slices_x[0].coordinate, 60.);
    assert!(hover(&mut slice, (76., 73.)).is_empty());
    let svg = render::to_svg(&layout.scene).unwrap();
    assert!(svg.contains("M76 54L88 54L88 66L76 66Z"));
    assert!(svg.contains(r#"stroke="rgba(174,34,97,1)""#));
    assert!(svg.contains(r#"viewBox="20 20 80 80""#));
}

#[test]
fn configured_factory_preserves_scale_mapping_ordinal_style_and_source_indices() {
    let c = config();
    let mut o = options();
    o.x_domain = Some([1., 1000.]);
    o.y_domain = Some([-10., 10.]);
    o.hidden.insert("hidden".into());
    o.axes.x.format = ValueFormat::Fixed(1);
    let stroke = GraphicStroke {
        color: INK,
        width: 2.,
        cap: StrokeCap::Round,
    };
    let mut style = isolated_style();
    style.connect_missing = true;
    style.interactive_sites = true;
    style.series.point_radius = 0.;
    style.series.point_border = Some(stroke);
    style.series.point_labels = Some(LinePointLabels::default());
    style.series_styles.insert(
        "quiet".into(),
        LineSeriesStyle {
            points: Some(false),
            color: Some(INK),
            ..style.series.clone()
        },
    );
    let scales = LineScaleOptions {
        x: ContinuousScaleOptions {
            kind: ScaleKind::Log,
            reverse: true,
            clamp: true,
            round: true,
            ..Default::default()
        },
        y: ContinuousScaleOptions {
            kind: ScaleKind::SymLog { constant: 2. },
            ..Default::default()
        },
    };
    let colors = OrdinalColorOptions {
        palette: OrdinalColorPalette::Custom(vec![PALETTE]),
        domain: Some(vec![
            CategoryKey::from("visible"),
            CategoryKey::from("quiet"),
        ]),
        ..Default::default()
    }
    .resolve([])
    .unwrap();
    let data = [
        Series::new("hidden", vec![Point::new(1., 0.)]),
        Series::new(
            "visible",
            vec![
                Point::new(10., 0.),
                Point::gap(100.),
                Point::new(10000., 2.),
            ],
        ),
        Series::new("quiet", vec![Point::new(1., 1.)]),
    ];
    let mut calls = Vec::new();
    let result = line_layout_with_scale_options_and_point_factory(
        &c,
        &data,
        &o,
        &style,
        &scales,
        Some(&colors),
        |point| {
            assert_eq!(point.series_id, "visible");
            assert_eq!(point.series_index, 1);
            assert_eq!(point.series_color, PALETTE);
            assert_eq!(point.fill, Some(PALETTE));
            assert_eq!(point.stroke, Some(stroke));
            let raw = data[1].data[point.source_index].x;
            assert_eq!(point.raw_x, LinePointX::Continuous(raw));
            assert_eq!(point.formatted_x, format!("{raw:.1}"));
            assert_eq!(point.style.point_radius, 0.);
            calls.push(point.source_index);
            let [x, y] = point.position;
            Ok(point.inherit(Geometry::Circle { x, y, radius: 4. }))
        },
    )
    .unwrap();
    assert_eq!(calls, [0, 2]);
    let sites = &result.layout.sites;
    assert_eq!(sites.len(), 3);
    assert_eq!(sites[0].position, [73., 60.]);
    assert_eq!(sites[1].position[0], 20.);
    assert_eq!(sites[1].raw_x, 10000.);
    assert_eq!(sites[2].series_id, "quiet");
    assert_eq!(sites[2].series_color, INK);
    assert_eq!(
        result.layout.scene.labels[sites[1].label_index.unwrap()].x,
        sites[1].position[0]
    );
    assert_eq!(result.layout.scene.legend.len(), 3);
}

#[test]
fn point_line_factory_and_labels_receive_typed_keys_even_with_a_fractional_pixel_adapter() {
    let c = config();
    let format = ValueFormat::date_time(
        "%Y-%m-%d",
        &kartis::datetime::TimeLocale::default(),
        kartis::datetime::TimeZone::Utc,
    )
    .unwrap();
    let mut line = options();
    line.x_domain = None;
    line.axes.x.format = format.clone();
    let day = CategoryKey::from(86_400_000.);
    let text = CategoryKey::from("86400000");
    let options = PointLineOptions {
        line,
        domain: Some(vec![day.clone(), text.clone(), CategoryKey::from("tail")]),
        window: Some([0., 1.5]),
        ..Default::default()
    };
    let mut style = isolated_style();
    style.connect_missing = true;
    style.series.point_labels = Some(LinePointLabels {
        text: LinePointLabelText::Coordinates {
            x: format,
            y: ValueFormat::Auto,
        },
        ..Default::default()
    });
    let data = [PointLineSeries::new(
        "s",
        vec![
            PointLineDatum::new(day.clone(), 5.),
            PointLineDatum::new("omitted", 5.),
            PointLineDatum::new(text.clone(), 6.),
            PointLineDatum::new("tail", 7.),
        ],
    )];
    let mut seen = Vec::new();
    let result = point_line_layout_with_point_factory(&c, &data, &options, &style, None, |point| {
        let LinePointX::Category(key) = point.raw_x else {
            panic!("numeric adapter escaped");
        };
        seen.push((
            point.source_index,
            key.clone(),
            point.formatted_x.to_owned(),
        ));
        let [x, y] = point.position;
        Ok(point.inherit(rect(x + 2., y - 2., 4., 4.)))
    })
    .unwrap();
    assert_eq!(
        seen,
        [
            (0, day.clone(), "1970-01-02".into()),
            (2, text.clone(), "86400000".into())
        ]
    );
    assert_eq!(result.sites[0].position[0], 20.);
    assert!((result.sites[1].position[0] - 73.33333333333333).abs() < 1e-9);
    assert_eq!(result.sites[0].raw_x, day);
    assert_eq!(result.sites[1].raw_x, text);
    assert_eq!(result.sites[1].mark_id, "line-point:1:s:2");
    assert_eq!(result.anchors.len(), 2);
    assert_eq!(
        result.scene.labels[result.sites[1].label_index.unwrap()].text,
        "86400000, 6"
    );
    assert!(point_line_layout(&c, &data, &options, &style).is_ok());
}

#[test]
fn live_path_anchor_survives_interpolation_camera_and_atomic_rejection() {
    let old = offsets(5.);
    let target = offsets(7.);
    let mut model = surface(&old, Picking::Nearest { distance: 1. });
    let live = old.scene.interpolate(&target.scene, 0.5).unwrap();
    let binding = target.interaction_binding();
    model
        .set_interaction_frame(&target.scene, Some(binding.clone()), live.clone())
        .unwrap();
    model
        .apply(SurfaceInput::Wheel {
            point: (60., 60.),
            delta: -45.,
        })
        .unwrap();
    let camera = model.paint_transform();
    let origin = camera.project((60., 52.)).unwrap();
    let ink = camera.project((76., 52.)).unwrap();
    let id = &target.sites[0].mark_id;
    assert_eq!(hover(&mut model, origin), std::slice::from_ref(id));
    let active = model.active_interaction(model.hovered()).unwrap();
    assert_eq!(active.rows[0].value, Some(7.));
    assert_eq!(active.rows[0].position, [60., 52.]);
    assert!(hover(&mut model, ink).is_empty());
    let mut bad = live.clone();
    let mark = bad.marks.iter_mut().find(|mark| &mark.id == id).unwrap();
    if let Geometry::Path(path) = &mut mark.geometry {
        path.pop();
    }
    assert!(
        model
            .set_interaction_frame(&target.scene, Some(binding), bad)
            .is_err()
    );
    assert_eq!(model.scene(), &live);
    assert_eq!(hover(&mut model, origin), std::slice::from_ref(id));
}

#[test]
fn invalid_geometry_and_factory_errors_reject_the_layout() {
    let c = config();
    let data = [Series::new("s", vec![Point::new(5., 5.)])];
    let o = options();
    let style = isolated_style();
    assert!(
        line_layout_with_point_factory(&c, &data, &o, &style, None, |point| {
            let [x, y] = point.position;
            Ok(point.inherit(Geometry::Circle {
                x: x + 1.,
                y,
                radius: 3.,
            }))
        })
        .is_err()
    );
    assert!(
        line_layout_with_point_factory(&c, &data, &o, &style, None, |point| {
            let mut geometry = point.inherit(rect(40., 40., 8., 8.));
            geometry.anchor_commands = Some(vec![0, 0]);
            Ok(geometry)
        })
        .is_err()
    );
    assert!(
        line_layout_with_point_factory(&c, &data, &o, &style, None, |point| {
            Ok(point.inherit(Geometry::Path(vec![C::Move(P::new(60., 60.)); 200_001])))
        })
        .is_err()
    );
    assert!(
        line_layout_with_point_factory(&c, &data, &o, &style, None, |_| {
            Err(ChartError("point rejected".into()))
        })
        .is_err_and(|error| error.0 == "point rejected")
    );
    assert!(line_layout_with_style(&c, &data, &o, &style).is_ok());
}

#[cfg(feature = "raster")]
#[test]
fn cpu_pixels_render_the_custom_hole_and_plot_edge() {
    use render::raster::{FontRegistry, render_raster};

    let layout = offsets(5.);
    let image = render_raster(&layout.scene, &FontRegistry::new(), &Default::default()).unwrap();
    let pixel = |x: usize, y: usize| {
        let i = (y * image.width as usize + x) * 4;
        &image.rgba[i..i + 4]
    };
    assert_eq!(pixel(76, 60), [174, 34, 97, 255]);
    assert_eq!(pixel(82, 60), [255, 255, 255, 255]);
    assert_eq!(pixel(60, 60), [255, 255, 255, 255]);
    assert_eq!(pixel(97, 36), [174, 34, 97, 255]);
    assert_eq!(pixel(103, 36), [255, 255, 255, 255]);
}
