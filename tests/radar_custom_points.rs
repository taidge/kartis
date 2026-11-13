use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use kartis::surface::{Picking, SurfaceInput, SurfaceModel, SurfaceOptions};
use kartis::*;

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

fn axes() -> [String; 4] {
    ["north", "east", "south", "west"].map(str::to_owned)
}

fn options() -> RadarLayoutOptions {
    let mut options = RadarLayoutOptions {
        domain: Some([0., 10.]),
        radius_ratio: 1.,
        axis_labels: false,
        ..Default::default()
    };
    options.grid.line.enabled = false;
    options.grid.spokes.enabled = false;
    options.series.fill_enabled = false;
    options.series.border.enabled = false;
    options
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

fn paint() -> Color {
    Color(190, 24, 76, 255)
}

fn offsets(north_value: f64) -> RadarLayout {
    radar_layout_with_point_factory(
        &config(),
        &axes(),
        &[RadarSeries::new("radar", vec![north_value, 10., 0., 5.])],
        &options(),
        &RadarInteractionOptions {
            slices: false,
            ..Default::default()
        },
        |ctx| {
            let [x, y] = ctx.position;
            let mut point = match ctx.axis_id {
                "north" => {
                    let mut point = ctx.inherit(rect(x + 16., y - 6., 12., 12.));
                    point.fill = None;
                    point.stroke = Some(GraphicStroke {
                        color: paint(),
                        width: 2.,
                        cap: StrokeCap::Butt,
                    });
                    point
                }
                "east" => ctx.inherit(rect(x - 6., y - 5., 18., 10.)),
                _ => ctx.inherit(Geometry::Circle { x, y, radius: 0. }),
            };
            if ctx.axis_id == "east" {
                point.fill_paint = Some(FillPaint::Linear {
                    start: [x - 6., y],
                    end: [x + 12., y],
                    stops: vec![
                        GradientStop::new(0., paint()),
                        GradientStop::new(1., paint()),
                    ],
                });
            }
            Ok(point)
        },
    )
    .unwrap()
}

fn surface(layout: &RadarLayout, picking: Picking) -> SurfaceModel {
    SurfaceModel::new_with_interaction(
        layout.scene.clone(),
        SurfaceOptions {
            picking,
            plot: Some(ClipRect::plot(&config())),
            ..Default::default()
        },
        Some(layout.interaction_binding()),
    )
    .unwrap()
}

fn hover(model: &mut SurfaceModel, point: (f64, f64)) -> Vec<String> {
    model.apply(SurfaceInput::Move { id: 1, point }).unwrap();
    model.hovered().to_vec()
}

#[test]
fn custom_ink_clips_and_hits_independently_of_the_data_anchor() {
    let layout = offsets(5.);
    let north = &layout.slices[0].rows[0];
    let east = &layout.slices[1].rows[0];
    assert_eq!(north.position, [60., 40.]);
    assert_eq!(north.color, paint());
    assert_eq!(east.color, paint());
    let mark = layout
        .scene
        .marks
        .iter()
        .find(|mark| mark.id == north.point_mark_id)
        .unwrap();
    assert_eq!(mark.geometry, rect(76., 34., 12., 12.));
    assert_eq!(mark.clip, Some(ClipRect::plot(&config())));
    let mut exact = surface(&layout, Picking::Geometry);
    assert_eq!(
        hover(&mut exact, (76., 40.)),
        std::slice::from_ref(&north.point_mark_id)
    );
    assert!(hover(&mut exact, (82., 40.)).is_empty());
    assert!(hover(&mut exact, (60., 40.)).is_empty());
    assert_eq!(
        hover(&mut exact, (97., 60.)),
        std::slice::from_ref(&east.point_mark_id)
    );
    assert!(layout.scene.hit_test(103., 60.).is_none());
    let mut nearest = surface(&layout, Picking::Nearest { distance: 1. });
    assert_eq!(
        hover(&mut nearest, (60., 40.)),
        std::slice::from_ref(&north.point_mark_id)
    );
    assert!(hover(&mut nearest, (76., 40.)).is_empty());
    let svg = render::to_svg(&layout.scene).unwrap();
    let id = format!("data-id=\"{}\"", north.point_mark_id);
    let path = svg
        .split('<')
        .find(|tag| tag.starts_with("path ") && tag.contains(&id))
        .unwrap();
    assert!(path.contains("M76 34L88 34L88 46L76 46Z"));
    assert!(path.contains("fill=\"none\""));
    assert!(path.contains("stroke=\"rgba(190,24,76,1)\""));
    assert!(svg.contains("viewBox=\"20 20 80 80\""));
    assert!(svg.contains("<linearGradient"));
}

#[test]
fn factory_keeps_scale_axes_identity_and_labels_with_hidden_and_disabled_points() {
    let c = config();
    let axis_ids = axes();
    let data = [
        RadarSeries::new("hidden", vec![1., 10., 100., 1000.]),
        RadarSeries::new("visible", vec![1., 10., 100., 1000.]),
        RadarSeries::new("quiet", vec![1.; 4]),
    ];
    let mut o = options();
    o.domain = None;
    o.hidden.insert("hidden".into());
    o.axis_labels_by_id
        .insert("east".into(), "East label".into());
    o.points.labels = true;
    o.points.value_format = ValueFormat::Fixed(1);
    o.points.fill = Some(paint());
    o.points.border = RadarLineStyle {
        enabled: true,
        color: Some(Color(60, 130, 170, 180)),
        width: 3.,
        opacity: 0.5,
        ..Default::default()
    };
    o.series_styles.insert(
        "quiet".into(),
        RadarSeriesStyle {
            points: Some(RadarPointOptions {
                enabled: false,
                ..Default::default()
            }),
            ..o.series.clone()
        },
    );
    let scale = RadarValueScaleOptions {
        kind: RadarValueScaleKind::Log { base: 10. },
        ..Default::default()
    };
    let interaction = RadarInteractionOptions {
        direction: PolarSliceDirection::CounterClockwise,
        ..Default::default()
    };
    let mut calls = Vec::new();
    let result = radar_layout_with_value_scale_and_point_factory(
        &c,
        &axis_ids,
        &data,
        &o,
        &scale,
        &interaction,
        |ctx| {
            assert_eq!(ctx.series, &data[1]);
            assert_eq!(ctx.series_index, 1);
            assert_eq!(ctx.axis_id, axis_ids[ctx.axis_index]);
            assert_eq!(ctx.value, data[1].values[ctx.axis_index]);
            assert_eq!(ctx.formatted_value, format!("{:.1}", ctx.value));
            assert_eq!(ctx.color, c.theme.color(1));
            assert_eq!(ctx.fill, Some(paint()));
            assert_eq!(ctx.stroke.unwrap().color, Color(60, 130, 170, 90));
            assert_eq!(ctx.plot, ClipRect::plot(&c));
            if ctx.axis_id == "east" {
                assert_eq!(ctx.axis_label, "East label");
                assert!((ctx.position[0] - 46.66666666666667).abs() < 1e-12);
                assert_eq!(ctx.position[1], 60.);
            }
            calls.push((ctx.axis_index, ctx.mark_id.to_owned()));
            Ok(ctx.inherit(Geometry::Rect {
                x: ctx.position[0] - 1.7,
                y: ctx.position[1] - 1.8,
                width: 3.4,
                height: 3.6,
            }))
        },
    )
    .unwrap();
    assert_eq!(result.value_scale.source_domain(), [1., 1000.]);
    assert_eq!(
        calls.iter().map(|call| call.0).collect::<Vec<_>>(),
        [0, 3, 2, 1]
    );
    let legacy =
        radar_layout_with_value_scale(&c, &axis_ids, &data, &o, &scale, &interaction).unwrap();
    assert_eq!(result.layout.scene.labels, legacy.layout.scene.labels);
    assert_eq!(
        result.layout.scene.label_series,
        legacy.layout.scene.label_series
    );
    assert_eq!(result.layout.scene.legend, legacy.layout.scene.legend);
    let east = &result.layout.slices[1];
    assert_eq!(east.rows[0].point_mark_id, calls[3].1);
    assert_eq!(east.rows[0].glyph_mark_id.as_ref(), Some(&calls[3].1));
    assert_eq!(east.rows[0].color, paint());
    assert_eq!(east.rows[1].series_id, "quiet");
    assert_eq!(east.rows[1].glyph_mark_id, None);
    let mut model = surface(&result.layout, Picking::Geometry);
    assert_eq!(
        hover(&mut model, (32., 60.)),
        east.rows
            .iter()
            .map(|row| row.point_mark_id.clone())
            .collect::<Vec<_>>()
    );
    let active = model.active_interaction(model.hovered()).unwrap();
    assert_eq!(active.label.as_deref(), Some("East label"));
    assert_eq!(active.rows[0].value, Some(10.));
    assert_eq!(active.rows[0].position, east.rows[0].position);
}

#[test]
fn live_offset_paths_follow_the_physical_anchor_through_camera_mapping() {
    let old = offsets(5.);
    let target = offsets(7.);
    let mut model = surface(&old, Picking::Nearest { distance: 1. });
    let target_binding = target.interaction_binding();
    let live = old.scene.interpolate(&target.scene, 0.5).unwrap();
    model
        .set_interaction_frame(&target.scene, Some(target_binding.clone()), live.clone())
        .unwrap();
    model
        .apply(SurfaceInput::Wheel {
            point: (60., 60.),
            delta: -45.,
        })
        .unwrap();
    let camera = model.paint_transform();
    let origin = camera.project((60., 36.)).unwrap();
    let ink = camera.project((76., 36.)).unwrap();
    let id = &target.slices[0].rows[0].point_mark_id;
    assert_eq!(hover(&mut model, origin), std::slice::from_ref(id));
    let active = model.active_interaction(model.hovered()).unwrap();
    assert_eq!(active.rows[0].value, Some(7.));
    assert_eq!(active.rows[0].position, [60., 36.]);
    assert!(hover(&mut model, ink).is_empty());
    let mut bad = live.clone();
    let mark = bad.marks.iter_mut().find(|mark| &mark.id == id).unwrap();
    if let Geometry::Path(path) = &mut mark.geometry {
        path.pop();
    }
    assert!(
        model
            .set_interaction_frame(&target.scene, Some(target_binding), bad)
            .is_err()
    );
    assert_eq!(model.scene(), &live);
    assert_eq!(hover(&mut model, origin), std::slice::from_ref(id));
}

#[test]
fn bad_geometry_and_factory_errors_reject_the_complete_layout() {
    let c = config();
    let axis_ids = axes();
    let data = [RadarSeries::new("radar", vec![5.; 4])];
    let o = options();
    let interaction = RadarInteractionOptions::default();
    let displaced =
        radar_layout_with_point_factory(&c, &axis_ids, &data, &o, &interaction, |ctx| {
            Ok(ctx.inherit(Geometry::Circle {
                x: ctx.position[0] + 5.,
                y: ctx.position[1],
                radius: 4.,
            }))
        });
    assert!(displaced.is_err());
    let duplicate =
        radar_layout_with_point_factory(&c, &axis_ids, &data, &o, &interaction, |ctx| {
            let mut point = ctx.inherit(rect(40., 40., 12., 12.));
            point.anchor_commands = Some(vec![0, 0]);
            Ok(point)
        });
    assert!(duplicate.is_err());
    let refused = radar_layout_with_point_factory(&c, &axis_ids, &data, &o, &interaction, |_| {
        Err(ChartError("factory refused".into()))
    });
    assert!(refused.is_err_and(|error| error.to_string() == "factory refused"));
    let mut calls = 0;
    let over = radar_layout_with_point_factory(&c, &axis_ids, &data, &o, &interaction, |ctx| {
        calls += 1;
        Ok(ctx.inherit(Geometry::Path(vec![C::Move(P::new(30., 40.)); 200_000])))
    });
    assert!(over.is_err());
    assert_eq!(calls, 1);
    assert!(radar_layout(&c, &axis_ids, &data, &o, &interaction).is_ok());
}

#[cfg(feature = "raster")]
#[test]
fn cpu_pixels_use_authored_hole_clip_and_current_paint() {
    use render::raster::{FontRegistry, render_raster};
    let layout = offsets(5.);
    let image = render_raster(&layout.scene, &FontRegistry::new(), &Default::default()).unwrap();
    let pixel = |x: usize, y: usize| {
        let i = (y * image.width as usize + x) * 4;
        &image.rgba[i..i + 4]
    };
    assert_eq!(pixel(76, 40), [190, 24, 76, 255]);
    assert_eq!(pixel(82, 40), [255, 255, 255, 255]);
    assert_eq!(pixel(60, 40), [255, 255, 255, 255]);
    assert_eq!(pixel(97, 60), [190, 24, 76, 255]);
    assert_eq!(pixel(103, 60), [255, 255, 255, 255]);
}
