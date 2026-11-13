use std::collections::BTreeSet;

use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use kartis::surface::{Picking, SurfaceInput, SurfaceModel, SurfaceOptions};
use kartis::*;

fn frame() -> ChartConfig {
    ChartConfig {
        width: 100.,
        height: 100.,
        margin: Margin {
            left: 10.,
            right: 10.,
            top: 10.,
            bottom: 10.,
        },
        ..Default::default()
    }
}

fn axis() -> ScatterAxisScale {
    ScatterAxisScale::Continuous {
        options: ContinuousScaleOptions::default(),
        min: ScatterBound::Fixed(0.),
        max: ScatterBound::Fixed(10.),
    }
}

fn options() -> ScatterLayoutOptions {
    ScatterLayoutOptions {
        scales: ScatterScaleOptions {
            x: axis(),
            y: axis(),
        },
        axes: ScatterAxes {
            top: None,
            right: None,
            bottom: None,
            left: None,
        },
        ..Default::default()
    }
}

fn data(x: f64, y: f64) -> [ScatterSeries; 1] {
    [ScatterSeries::new(
        "series",
        vec![ScatterDatum::new("datum", x, y)],
    )]
}

fn rectangle(x: f64, y: f64, w: f64, h: f64) -> Vec<C> {
    vec![
        C::Move(P::new(x, y)),
        C::Line(P::new(x + w, y)),
        C::Line(P::new(x + w, y + h)),
        C::Line(P::new(x, y + h)),
        C::Close,
    ]
}

fn surface(layout: &ScatterLayout, picking: Picking) -> SurfaceModel {
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

fn hover(surface: &mut SurfaceModel, x: f64, y: f64) -> Vec<String> {
    surface
        .apply(SurfaceInput::Move {
            id: 1,
            point: (x, y),
        })
        .unwrap();
    surface.hovered().to_vec()
}

#[test]
fn factory_receives_typed_sources_formatted_values_order_and_inherited_paint() {
    let mut first = ScatterDatum::new("skip", "north", 2.);
    first.label = Some("Missing size".into());
    let mut visible = ScatterDatum::new("visible", "south", 5.);
    visible.size_value = Some(50.);
    visible.label = Some("Original <label>".into());
    visible.color = Some(Color(80, 20, 210, 255));
    let mut series = ScatterSeries::new("shown", vec![first, visible]);
    series.label = "User-facing series".into();
    let source = [
        ScatterSeries::new("hidden", vec![ScatterDatum::new("hidden", "north", 8.)]),
        series,
    ];
    let settings = ScatterLayoutOptions {
        scales: ScatterScaleOptions {
            x: ScatterAxisScale::Point {
                options: Default::default(),
                domain: Some(vec!["north".into(), "south".into()]),
                window: None,
            },
            y: axis(),
        },
        node: ScatterNodeStyle {
            shape: ScatterNodeShape::Circle {
                diameter: ScatterSize::Linear {
                    values: [0., 100.],
                    sizes: [2., 10.],
                    clamp: false,
                    missing: ScatterMissingSize::Skip,
                },
            },
            opacity: 0.7,
            ..Default::default()
        },
        hidden: BTreeSet::from(["hidden".into()]),
        labels: ScatterLabels {
            enabled: true,
            text: ScatterLabelText::Coordinates,
            ..Default::default()
        },
        y_format: ValueFormat::Fixed(1),
        ..options()
    };
    let mut called = 0;
    let layout = scatter_layout_with_node_factory(&frame(), &source, &settings, |ctx| {
        called += 1;
        assert_eq!(ctx.series_id, "shown");
        assert_eq!(ctx.series_label, "User-facing series");
        assert_eq!(
            (ctx.series_index, ctx.source_index, ctx.global_index),
            (1, 1, 2)
        );
        assert_eq!(ctx.datum.x, CategoryKey::String("south".into()));
        assert_eq!(ctx.datum.y, CategoryKey::Number(5.));
        assert_eq!(ctx.datum.size_value, Some(50.));
        assert_eq!((ctx.formatted_x, ctx.formatted_y), ("south", "5.0"));
        assert_eq!(ctx.size, 6.);
        assert_eq!(ctx.size_unit, ScatterSizeUnit::CircleDiameter);
        assert_eq!(ctx.color, Color(80, 20, 210, 255));
        assert_eq!(ctx.opacity, 0.7);
        let [x, y] = ctx.position;
        Ok(ScatterNodeGeometry::inherit(
            Geometry::Rect {
                x: x - 3.,
                y: y - 2.,
                width: 6.,
                height: 4.,
            },
            ctx,
        ))
    })
    .unwrap();
    assert_eq!(called, 1);
    assert_eq!(layout.metadata.len(), 3);
    assert!(layout.metadata[0].hidden);
    assert!(layout.metadata[1].mark_index.is_none());
    let site = &layout.sites[0];
    let mark = &layout.scene.marks[site.mark_index];
    assert_eq!(mark.id, site.mark_id);
    assert_eq!(mark.fill, Some(Color(80, 20, 210, 255)));
    assert_eq!(mark.opacity, 0.7);
    assert!(mark.label.contains("Original <label>"));
    assert_eq!(
        layout.scene.label_series[&site.label_index.unwrap()],
        "shown"
    );
    assert_eq!(
        layout.scene.labels[site.label_index.unwrap()].text,
        "south, 5.0"
    );
}

#[test]
fn projected_fractional_rect_preserves_the_exact_data_origin() {
    let c = frame();
    let source = data(0.37, 5.);
    let o = options();
    let builtin = scatter_layout(&c, &source, &o).unwrap();
    let position = builtin.sites[0].position;
    let mut authored = None;
    let custom = scatter_layout_with_node_factory(&c, &source, &o, |ctx| {
        let [x, y] = ctx.position;
        let width = [62., 90., 128., 256., 512.]
            .into_iter()
            .find(|w| (x - w / 2.) + w / 2. != x)
            .expect("fractional center exercises reconstruction rounding");
        let rect = Geometry::Rect {
            x: x - width / 2.,
            y: y - 4.,
            width,
            height: 8.,
        };
        authored = Some(rect.clone());
        Ok(ScatterNodeGeometry::inherit(rect, ctx))
    })
    .unwrap();
    let site = &custom.sites[0];
    assert_eq!(
        custom.scene.marks[site.mark_index].geometry,
        authored.unwrap()
    );
    assert_eq!(site.position, position);
    assert_eq!(
        custom.anchors[&site.mark_id]
            .resolve(&custom.scene.marks[site.mark_index].geometry)
            .unwrap(),
        position
    );
    assert_eq!(
        custom
            .nearest(position[0], position[1], 0.)
            .unwrap()
            .unwrap()
            .mark_id,
        site.mark_id
    );
    let displaced = scatter_layout_with_node_factory(&c, &source, &o, |ctx| {
        let [x, y] = ctx.position;
        Ok(ScatterNodeGeometry::inherit(
            Geometry::Rect {
                x: x - 2.,
                y: y - 4.,
                width: 8.,
                height: 8.,
            },
            ctx,
        ))
    });
    assert!(displaced.is_err());
}

#[test]
fn visible_effective_paint_supplies_color_for_a_transparent_solid() {
    let draw = |gradient: bool| {
        scatter_layout_with_node_factory(&frame(), &data(5., 5.), &options(), |ctx| {
            let mut node = ScatterNodeGeometry::inherit(
                Geometry::Rect {
                    x: 40.,
                    y: 40.,
                    width: 20.,
                    height: 20.,
                },
                ctx,
            );
            node.fill = Some(Color(255, 0, 0, 0));
            node.stroke = Some(GraphicStroke {
                color: Color(30, 90, 150, 255),
                width: 2.,
                cap: StrokeCap::Butt,
            });
            if gradient {
                node.fill_paint = Some(FillPaint::Linear {
                    start: [40., 50.],
                    end: [60., 50.],
                    stops: vec![
                        GradientStop::new(0., Color(255, 0, 0, 0)),
                        GradientStop::new(1., Color(90, 180, 40, 255)),
                    ],
                });
            }
            Ok(node)
        })
        .unwrap()
    };
    let outlined = draw(false);
    let id = &outlined.sites[0].mark_id;
    assert_eq!(outlined.sites[0].color, Color(30, 90, 150, 255));
    let mut exact = surface(&outlined, Picking::Geometry);
    assert!(hover(&mut exact, 50., 50.).is_empty());
    assert_eq!(hover(&mut exact, 40., 50.), std::slice::from_ref(id));
    let graded = draw(true);
    assert_eq!(graded.sites[0].color, Color(90, 180, 40, 255));
    assert_eq!(
        hover(&mut surface(&graded, Picking::Geometry), 50., 50.),
        std::slice::from_ref(id)
    );
}

#[test]
fn authored_asymmetric_path_and_logical_anchor_are_distinct_and_clipped() {
    let mut expected = vec![C::Move(P::new(6., 6.))];
    expected.extend(rectangle(76., 44., 22., 12.));
    let layout = scatter_layout_with_node_factory(&frame(), &data(5., 5.), &options(), |ctx| {
        Ok(ScatterNodeGeometry::inherit(
            Geometry::Path(expected.clone()),
            ctx,
        ))
    })
    .unwrap();
    let site = &layout.sites[0];
    let mark = &layout.scene.marks[site.mark_index];
    assert_eq!(mark.geometry, Geometry::Path(expected));
    assert_eq!(site.position, [50., 50.]);
    assert_eq!(
        site.bounds,
        ClipRect {
            x: 76.,
            y: 44.,
            width: 22.,
            height: 12.
        }
    );
    assert_eq!(mark.clip, Some(layout.plot));
    let mut exact = surface(&layout, Picking::Geometry);
    let mut nearest = surface(&layout, Picking::Nearest { distance: 1. });
    assert_eq!(
        hover(&mut exact, 82., 50.),
        std::slice::from_ref(&site.mark_id)
    );
    assert!(hover(&mut exact, 50., 50.).is_empty());
    assert!(hover(&mut exact, 94., 50.).is_empty());
    assert!(layout.scene.hit_test(94., 50.).is_none());
    assert_eq!(
        hover(&mut nearest, 50., 50.),
        std::slice::from_ref(&site.mark_id)
    );
    assert!(hover(&mut nearest, 82., 50.).is_empty());
    assert_eq!(
        layout.nearest(50., 50., 0.).unwrap().unwrap().mark_id,
        site.mark_id
    );
}

#[test]
fn offset_single_point_stroke_keeps_real_cap_without_added_commands() {
    let expected = Geometry::Path(vec![C::Move(P::new(70., 50.)), C::Line(P::new(70., 50.))]);
    let layout = scatter_layout_with_node_factory(&frame(), &data(5., 5.), &options(), |ctx| {
        let mut node = ScatterNodeGeometry::inherit(expected.clone(), ctx);
        node.fill = None;
        node.stroke = Some(GraphicStroke {
            color: Color(180, 20, 50, 255),
            width: 12.,
            cap: StrokeCap::Round,
        });
        Ok(node)
    })
    .unwrap();
    let site = &layout.sites[0];
    assert_eq!(layout.scene.marks[0].geometry, expected);
    assert_eq!(site.color, Color(180, 20, 50, 255));
    let mut exact = surface(&layout, Picking::Geometry);
    assert_eq!(
        hover(&mut exact, 75., 50.),
        std::slice::from_ref(&site.mark_id)
    );
    assert!(hover(&mut exact, 77., 50.).is_empty());
    assert!(hover(&mut exact, 50., 50.).is_empty());
    let mut nearest = surface(&layout, Picking::Nearest { distance: 1. });
    assert_eq!(
        hover(&mut nearest, 50., 50.),
        std::slice::from_ref(&site.mark_id)
    );
    let svg = render::to_svg(&layout.scene).unwrap();
    assert!(svg.contains(&format!("data-id=\"{}\"", site.mark_id)));
    assert!(svg.contains("fill=\"rgba(180,20,50,1)\""));
    assert!(!svg.contains("stroke-linecap"));
}

#[test]
fn compatible_animation_follows_the_declared_real_support_displacement() {
    let layout = |size: f64| {
        scatter_layout_with_node_factory(&frame(), &data(5., 5.), &options(), |ctx| {
            let [x, y] = ctx.position;
            let mut commands = vec![C::Move(P::new(8. + size * 2., 8.))];
            commands.extend(rectangle(x + size, y - 4., size, 8.));
            Ok(ScatterNodeGeometry::inherit(Geometry::Path(commands), ctx))
        })
        .unwrap()
    };
    let old = layout(10.);
    let target = layout(20.);
    let id = &target.sites[0].mark_id;
    let binding = target.interaction_binding();
    assert_eq!(
        binding.anchors[id]
            .resolve(&old.scene.marks[0].geometry)
            .unwrap(),
        [35., 50.]
    );
    assert_eq!(
        binding.anchors[id]
            .resolve(&target.scene.marks[0].geometry)
            .unwrap(),
        [50., 50.]
    );
    let live = old.scene.interpolate(&target.scene, 0.5).unwrap();
    assert_eq!(
        binding.anchors[id]
            .resolve(&live.marks[0].geometry)
            .unwrap(),
        [42.5, 50.]
    );
    let mut pointer = surface(&old, Picking::Nearest { distance: 1. });
    pointer
        .set_interaction_frame(&target.scene, Some(binding.clone()), live)
        .unwrap();
    assert_eq!(hover(&mut pointer, 42.5, 50.), std::slice::from_ref(id));
    assert!(hover(&mut pointer, 50., 50.).is_empty());
    let mut wrong = target.scene.clone();
    if let Geometry::Path(path) = &mut wrong.marks[0].geometry {
        path.pop();
    }
    assert!(
        pointer
            .set_interaction_frame(&target.scene, Some(binding), wrong)
            .is_err()
    );
    assert_eq!(hover(&mut pointer, 42.5, 50.), std::slice::from_ref(id));
}

#[test]
fn move_only_path_keeps_logical_site_and_does_not_paint() {
    let expected = Geometry::Path(vec![C::Move(P::new(74., 50.))]);
    let layout = scatter_layout_with_node_factory(&frame(), &data(5., 5.), &options(), |ctx| {
        let mut node = ScatterNodeGeometry::inherit(expected.clone(), ctx);
        node.fill = None;
        node.stroke = Some(GraphicStroke {
            color: Color(180, 20, 50, 255),
            width: 12.,
            cap: StrokeCap::Round,
        });
        Ok(node)
    })
    .unwrap();
    assert_eq!(layout.scene.marks[0].geometry, expected);
    assert!(hover(&mut surface(&layout, Picking::Geometry), 74., 50.).is_empty());
    assert_eq!(
        layout.nearest(50., 50., 0.).unwrap().unwrap().position,
        [50., 50.]
    );
}

#[test]
fn rejected_geometry_and_callback_leave_the_last_candidate_available() {
    let config = frame();
    let source = data(5., 5.);
    let settings = options();
    let previous = scatter_layout(&config, &source, &settings).unwrap();
    let base = |geometry, ctx: &ScatterNodeContext<'_>| ScatterNodeGeometry::inherit(geometry, ctx);
    let candidates = [
        Geometry::Circle {
            x: 52.,
            y: 50.,
            radius: 3.,
        },
        Geometry::Rect {
            x: 50.,
            y: 50.,
            width: -1.,
            height: 2.,
        },
        Geometry::Path(vec![C::Line(P::new(20., 20.))]),
        Geometry::Path(vec![C::Move(P::new(f64::NAN, 0.))]),
        Geometry::Path(vec![C::Move(P::new(50., 50.)); 200_001]),
    ];
    for geometry in candidates {
        assert!(
            scatter_layout_with_node_factory(&config, &source, &settings, |ctx| Ok(base(
                geometry.clone(),
                ctx
            )))
            .is_err()
        );
    }
    let two = [ScatterSeries::new(
        "series",
        vec![
            ScatterDatum::new("one", 4., 5.),
            ScatterDatum::new("two", 6., 5.),
        ],
    )];
    let over_total = scatter_layout_with_node_factory(&config, &two, &settings, |ctx| {
        let [x, y] = ctx.position;
        Ok(base(
            Geometry::Path(vec![C::Move(P::new(x, y)); 100_001]),
            ctx,
        ))
    });
    assert!(matches!(over_total,Err(ChartError(message)) if message.contains("geometry budget")));
    let invalid_support = scatter_layout_with_node_factory(&config, &source, &settings, |ctx| {
        let mut node = base(Geometry::Path(rectangle(40., 40., 20., 20.)), ctx);
        node.anchor_commands = Some(vec![4]);
        Ok(node)
    });
    assert!(invalid_support.is_err());
    let denied = scatter_layout_with_node_factory(&config, &source, &settings, |_| {
        Err(ChartError("caller rejection".into()))
    });
    assert!(matches!(denied,Err(ChartError(msg)) if msg=="caller rejection"));
    let mut exact = surface(&previous, Picking::Geometry);
    assert_eq!(
        hover(&mut exact, 50., 50.),
        std::slice::from_ref(&previous.sites[0].mark_id)
    );
}

#[test]
fn factory_can_author_cubic_fill_stroke_gradient_and_dash() {
    let path = Geometry::Path(vec![
        C::Move(P::new(30., 55.)),
        C::Cubic {
            first: P::new(40., 18.),
            second: P::new(65., 20.),
            end: P::new(70., 55.),
        },
        C::Line(P::new(60., 68.)),
        C::Close,
    ]);
    let layout = scatter_layout_with_node_factory(&frame(), &data(5., 5.), &options(), |ctx| {
        let mut node = ScatterNodeGeometry::inherit(path.clone(), ctx);
        node.fill_paint = Some(FillPaint::Linear {
            start: [30., 50.],
            end: [70., 50.],
            stops: vec![
                GradientStop::new(0., Color(230, 60, 25, 255)),
                GradientStop::new(1., Color(35, 100, 200, 255)),
            ],
        });
        node.stroke = Some(GraphicStroke {
            color: Color(10, 20, 30, 255),
            width: 2.,
            cap: StrokeCap::Butt,
        });
        node.stroke_dash = Some(StrokeDash {
            segments: vec![6., 3.],
            offset: 1.,
        });
        node.opacity = 0.8;
        node.anchor_commands = Some(vec![0, 1]);
        Ok(node)
    })
    .unwrap();
    assert_eq!(layout.scene.marks[0].geometry, path);
    assert_eq!(
        layout.sites[0].bounds,
        ClipRect {
            x: 30.,
            y: 18.,
            width: 40.,
            height: 50.
        }
    );
    let svg = render::to_svg(&layout.scene).unwrap();
    assert!(svg.contains("<linearGradient"));
    assert!(svg.contains("stroke-dasharray=\"6 3\""));
    assert!(svg.contains("opacity=\"0.8\""));
    #[cfg(feature = "raster")]
    {
        let image =
            render::raster::render_raster(&layout.scene, &Default::default(), &Default::default())
                .unwrap();
        let pixel = |x: usize, y: usize| &image.rgba[(y * image.width as usize + x) * 4..][..4];
        let left = pixel(42, 49);
        let right = pixel(60, 49);
        assert!(left[0] > left[2] && right[0] < right[2]);
        assert_eq!(pixel(80, 50), [255, 255, 255, 255]);
    }
}
