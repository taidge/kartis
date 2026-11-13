use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use kartis::render::Canvas;
use kartis::surface::*;
use kartis::*;

fn scene(cap: StrokeCap, width: f64, offset: Option<f64>, translucent: bool, path: u8) -> Scene {
    let mut s = ChartConfig {
        width: 256.,
        height: 256.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    let geometry = if path == 1 {
        Geometry::Path(vec![
            C::Move(P::new(128., 128.)),
            C::Line(P::new(128., 128.)),
            C::Move(P::new(128., 128.)),
            C::Line(P::new(128., 128.)),
        ])
    } else if path == 3 {
        Geometry::Path(vec![C::Move(P::new(128., 128.)), C::Close])
    } else if path == 4 {
        Geometry::Path(vec![
            C::Move(P::new(128., 128.)),
            C::Cubic {
                first: P::new(128., 128.),
                second: P::new(128., 128.),
                end: P::new(128., 128.),
            },
            C::Close,
        ])
    } else if path == 2 {
        Geometry::Rect {
            x: 128.,
            y: 128.,
            width: 0.,
            height: 0.,
        }
    } else {
        Geometry::Circle {
            x: 128.,
            y: 128.,
            radius: 0.,
        }
    };
    let mut m = Mark::new("zero", geometry, Color(255, 0, 0, 255));
    m.fill = None;
    m.series = "caps".into();
    m.label = "zero-size cap".into();
    m.stroke = Some(GraphicStroke {
        color: Color(255, 0, 0, if translucent { 128 } else { 255 }),
        width,
        cap,
    });
    m.opacity = if translucent { 0.5 } else { 1. };
    m.stroke_dash = offset.map(|offset| StrokeDash {
        segments: vec![3., 3.],
        offset,
    });
    s.marks.push(m);
    s
}
fn model(s: &Scene) -> SurfaceModel {
    let mut data = InteractionBinding {
        sites: vec![InteractionSite {
            mark_id: "zero".into(),
            series: "caps".into(),
            label: "zero".into(),
            value: Some(0.),
            formatted_value: None,
            color: Color(255, 0, 0, 255),
            position: [128., 128.],
        }],
        ..Default::default()
    };
    if let Geometry::Path(commands) = &s.marks[0].geometry {
        data.anchors.insert(
            "zero".into(),
            InteractionAnchor::path_points(
                &s.marks[0].geometry,
                [128., 128.],
                &commands
                    .iter()
                    .enumerate()
                    .filter_map(|(i, c)| matches!(c, C::Move(_) | C::Line(_)).then_some(i))
                    .collect::<Vec<_>>(),
            )
            .unwrap(),
        );
    }
    SurfaceModel::new_with_interaction(s.clone(), Default::default(), Some(data)).unwrap()
}
fn hit(model: &SurfaceModel, point: (f64, f64)) -> bool {
    let mut model = model.clone();
    model.apply(SurfaceInput::Move { id: 1, point }).unwrap();
    !model.hovered().is_empty()
}

#[test]
fn typed_zero_caps_cover_square_corners_and_keep_round_butt_and_dash_off_empty() {
    for path in [0, 1, 2, 3, 4] {
        let square = model(&scene(StrokeCap::Square, 100., None, false, path));
        let round = model(&scene(StrokeCap::Round, 100., None, false, path));
        let butt = model(&scene(StrokeCap::Butt, 100., None, false, path));
        assert!(
            hit(&square, (173., 173.)),
            "true thick square corner was lost"
        );
        assert!(!hit(&round, (173., 173.)));
        assert!(!hit(&butt, (128., 128.)));
        assert!(hit(&round, (128., 128.)));
        assert!(hit(&square, (128., 128.)));
        assert!(!hit(&square, (179., 128.)));
        for cap in [StrokeCap::Round, StrokeCap::Square] {
            assert!(hit(
                &model(&scene(cap, 100., Some(1.), false, path)),
                (128., 128.)
            ));
            assert!(!hit(
                &model(&scene(cap, 100., Some(4.), false, path)),
                (128., 128.)
            ));
        }
    }
    let mut move_only = scene(StrokeCap::Round, 100., None, false, 0);
    move_only.marks[0].geometry = Geometry::Path(vec![C::Move(P::new(128., 128.))]);
    assert!(
        !hit(&model(&move_only), (128., 128.)),
        "Move-only must not paint a cap"
    );
    // Bare Scene retains its documented legacy radial stroke-slop behavior.
    assert!(
        scene(StrokeCap::Butt, 100., None, false, 0)
            .hit_test(128., 128.)
            .is_some()
    );
}
#[test]
fn typed_cap_picking_retains_rect_and_path_masks_and_alpha() {
    let mut s = scene(StrokeCap::Square, 100., None, false, 0);
    s.marks[0].clip = Some(ClipRect {
        x: 100.,
        y: 100.,
        width: 40.,
        height: 40.,
    });
    assert!(hit(&model(&s), (120., 120.)));
    assert!(!hit(&model(&s), (173., 173.)));
    s.marks[0].clip_path = Some(Box::new(ClipPath::circle(128., 128., 10.)));
    assert!(!hit(&model(&s), (137., 137.)));
    assert!(hit(&model(&s), (128., 128.)));
    s.marks[0].stroke.as_mut().unwrap().color.3 = 0;
    assert!(!hit(&model(&s), (128., 128.)));
}
#[derive(Default)]
struct Recorder {
    path: Vec<C>,
    fills: Vec<(Vec<C>, Color, f64)>,
    strokes: usize,
    begin: usize,
    finish: usize,
}
impl Canvas for Recorder {
    fn begin(&mut self, _: f64, _: f64, _: Color) -> Result<()> {
        self.begin += 1;
        Ok(())
    }
    fn begin_path(&mut self) -> Result<()> {
        self.path.clear();
        Ok(())
    }
    fn move_to(&mut self, x: f64, y: f64) -> Result<()> {
        self.path.push(C::Move(P::new(x, y)));
        Ok(())
    }
    fn line_to(&mut self, x: f64, y: f64) -> Result<()> {
        self.path.push(C::Line(P::new(x, y)));
        Ok(())
    }
    fn cubic_to(&mut self, x: f64, y: f64, a: f64, b: f64, c: f64, d: f64) -> Result<()> {
        self.path.push(C::Cubic {
            first: P::new(x, y),
            second: P::new(a, b),
            end: P::new(c, d),
        });
        Ok(())
    }
    fn close_path(&mut self) -> Result<()> {
        self.path.push(C::Close);
        Ok(())
    }
    fn fill(&mut self, c: Color, o: f64) -> Result<()> {
        self.fills.push((self.path.clone(), c, o));
        Ok(())
    }
    fn stroke(&mut self, _: GraphicStroke, _: f64) -> Result<()> {
        self.strokes += 1;
        Ok(())
    }
    fn text(&mut self, _: &Label) -> Result<()> {
        Ok(())
    }
    fn finish(&mut self) -> Result<()> {
        self.finish += 1;
        Ok(())
    }
}
#[test]
fn common_canvas_replays_one_footprint_with_alpha_and_dash_phase() {
    for path in [0, 1, 2, 3, 4] {
        for cap in [StrokeCap::Butt, StrokeCap::Round, StrokeCap::Square] {
            for offset in [
                None,
                Some(0.),
                Some(1.),
                Some(3.),
                Some(4.),
                Some(6.),
                Some(-1.),
            ] {
                let s = scene(cap, 10., offset, true, path);
                let mut target = Recorder::default();
                render::draw(&s, &mut target).unwrap();
                let painted = cap != StrokeCap::Butt && !matches!(offset, Some(4.) | Some(-1.));
                assert_eq!(target.fills.len(), usize::from(painted));
                assert_eq!(
                    target.strokes, 0,
                    "a second native stroke would double-composite cap alpha"
                );
                assert_eq!((target.begin, target.finish), (1, 1));
                if painted {
                    assert_eq!(target.fills[0].1, Color(255, 0, 0, 128));
                    assert_eq!(target.fills[0].2, 0.5);
                    assert!(
                        target.fills[0]
                            .0
                            .iter()
                            .any(|c| matches!(c,C::Line(p)|C::Move(p)if p.x!=128.||p.y!=128.))
                    );
                }
            }
        }
    }
}
#[test]
fn zero_stroke_svg2_dash_endpoints_dots_and_all_zero_are_shared() {
    for path in [0, 1, 2, 3, 4] {
        for (segments, cases) in [
            (
                vec![3., 3.],
                vec![
                    (0., true),
                    (1., true),
                    (3., true),
                    (4., false),
                    (6., true),
                    (-1., false),
                ],
            ),
            (
                vec![0., 3.],
                vec![
                    (0., true),
                    (1., false),
                    (3., true),
                    (4., false),
                    (6., true),
                    (-1., false),
                ],
            ),
            (
                vec![0., 0.],
                vec![(0., true), (1., true), (3., true), (-1., true)],
            ),
        ] {
            for cap in [StrokeCap::Butt, StrokeCap::Round, StrokeCap::Square] {
                for &(offset, on) in &cases {
                    let mut s = scene(cap, 100., None, true, path);
                    s.marks[0].stroke_dash = Some(StrokeDash {
                        segments: segments.clone(),
                        offset,
                    });
                    let expected = on && cap != StrokeCap::Butt;
                    assert_eq!(
                        hit(&model(&s), (128., 128.)),
                        expected,
                        "{segments:?}/{offset}/{cap:?}"
                    );
                    let mut canvas = Recorder::default();
                    render::draw(&s, &mut canvas).unwrap();
                    assert_eq!(canvas.fills.len(), usize::from(expected));
                    assert_eq!(canvas.strokes, 0);
                    let svg = render::to_svg(&s).unwrap();
                    assert!(!svg.contains("stroke-dasharray"));
                    #[cfg(feature = "raster")]
                    {
                        let frame = render::raster::render_raster(
                            &s,
                            &Default::default(),
                            &Default::default(),
                        )
                        .unwrap();
                        let green = frame.rgba[(128 * 256 + 128) * 4 + 1];
                        assert_eq!(green < 255, expected);
                        if expected {
                            assert!((i16::from(green) - 191).abs() <= 1);
                        }
                    }
                }
            }
        }
    }
}
#[test]
fn malformed_point_stroke_is_rejected_before_any_canvas_mutation() {
    let mut s = scene(StrokeCap::Square, 10., None, false, 0);
    s.marks[0].stroke.as_mut().unwrap().width = f64::NAN;
    let mut target = Recorder::default();
    assert!(render::draw(&s, &mut target).is_err());
    assert_eq!(target.begin, 0);
}
#[cfg(feature = "raster")]
#[test]
fn cpu_cap_pixels_have_single_alpha_round_square_corner_and_dash_off() {
    let image = |s: &Scene| {
        render::raster::render_raster(s, &Default::default(), &Default::default()).unwrap()
    };
    for path in [0, 1, 2, 3, 4] {
        for cap in [StrokeCap::Butt, StrokeCap::Round, StrokeCap::Square] {
            let s = scene(cap, 100., None, true, path);
            let frame = image(&s);
            let pixel = |x: usize, y: usize| &frame.rgba[(y * frame.width as usize + x) * 4..][..4];
            if cap == StrokeCap::Butt {
                assert_eq!(pixel(128, 128), [255, 255, 255, 255]);
            } else {
                assert_eq!(pixel(128, 128)[0], 255);
                assert!(
                    (i16::from(pixel(128, 128)[1]) - 191).abs() <= 1,
                    "cap alpha painted more than once"
                );
            }
            assert_eq!(
                pixel(173, 173) == [255, 255, 255, 255],
                cap != StrokeCap::Square
            );
            let off = image(&scene(cap, 100., Some(4.), false, path));
            assert!(
                off.rgba
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .all(|p| *p == [255, 255, 255, 255])
            );
        }
    }
}
#[test]
fn svg_and_script_canvas_assets_keep_all_three_caps_phase_and_alpha() {
    let mut html = String::from("<!doctype html><html lang='en'><body>");
    for geometry in 0..5 {
        for mode in 0..12 {
            let translucent = matches!(mode, 1 | 3 | 5 | 7);
            let (segments, offset, on) = match mode {
                0 | 1 => (vec![], 0., true),
                2 | 3 => (vec![3., 3.], 1., true),
                4 | 5 => (vec![3., 3.], 4., false),
                6 | 7 => (vec![3., 3.], 3., true),
                8 => (vec![0., 3.], 0., true),
                9 => (vec![0., 3.], 1., false),
                10 => (vec![0., 0.], 1., true),
                _ => (vec![0., 3.], 3., true),
            };
            let mut s = scene(StrokeCap::Butt, 100., None, translucent, 0);
            s.marks[0].stroke_dash = Some(StrokeDash { segments, offset });
            s.width = 512.;
            for (i, cap) in [StrokeCap::Butt, StrokeCap::Round, StrokeCap::Square]
                .into_iter()
                .enumerate()
            {
                let mut mark = s.marks[0].clone();
                mark.id = format!("cap-{i}");
                let x = [64., 192., 384.][i];
                mark.geometry = match geometry {
                    0 => Geometry::Circle {
                        x,
                        y: 128.,
                        radius: 0.,
                    },
                    1 => Geometry::Path(vec![
                        C::Move(P::new(x, 128.)),
                        C::Line(P::new(x, 128.)),
                        C::Move(P::new(x, 128.)),
                        C::Line(P::new(x, 128.)),
                    ]),
                    3 => Geometry::Path(vec![C::Move(P::new(x, 128.)), C::Close]),
                    4 => Geometry::Path(vec![
                        C::Move(P::new(x, 128.)),
                        C::Cubic {
                            first: P::new(x, 128.),
                            second: P::new(x, 128.),
                            end: P::new(x, 128.),
                        },
                        C::Close,
                    ]),
                    _ => Geometry::Rect {
                        x,
                        y: 128.,
                        width: 0.,
                        height: 0.,
                    },
                };
                mark.stroke.as_mut().unwrap().cap = cap;
                if i == 0 {
                    s.marks[0] = mark;
                } else {
                    s.marks.push(mark);
                }
            }
            let script =
                render::to_canvas_script(&s, &format!("canvas-{geometry}-{mode}")).unwrap();
            assert_eq!(
                script.matches("ctx.fill();").count(),
                if on { 2 } else { 0 }
            );
            assert!(!script.contains("ctx.stroke();"));
            let svg =
                render::to_svg_with_namespace(&s, &format!("zero-cap-{geometry}-{mode}")).unwrap();
            html.push_str(&format!("<section data-geometry='{geometry}' data-mode='{mode}'>{svg}<canvas id='canvas-{geometry}-{mode}'></canvas><script>{script}</script></section>"));
        }
    }
    html.push_str("</body></html>");
    println!("POINT_CAP_HTML_BEGIN{html}POINT_CAP_HTML_END");
}
