use glory::render::GraphicStroke;
use kartis::render::{self, Canvas};
use kartis::*;
#[test]
fn rounded_bar_styles_keep_metadata_and_use_real_curved_geometry() {
    let c = ChartConfig::default();
    let categories = vec!["A".into(), "B".into()];
    let data = vec![BarSeries::new("series", vec![Some(10.), Some(-8.)])];
    let border = GraphicStroke {
        color: Color(10, 20, 30, 255),
        width: 2.,
        cap: glory::render::StrokeCap::Round,
    };
    for horizontal in [false, true] {
        let base = bar(
            &c,
            &categories,
            &data,
            &BarOptions {
                horizontal,
                ..Default::default()
            },
        )
        .unwrap();
        let styled = bar(
            &c,
            &categories,
            &data,
            &BarOptions {
                horizontal,
                corner_radius: 18.,
                border: Some(border),
                ..Default::default()
            },
        )
        .unwrap();
        for m in styled.marks.iter().filter(|m| !m.series.is_empty()) {
            let old = base.marks.iter().find(|o| o.id == m.id).unwrap();
            assert_eq!(m.value, old.value);
            assert_eq!(m.fill, old.fill);
            assert_eq!(m.stroke, Some(border));
            let Geometry::Path(commands) = &m.geometry else {
                panic!("rounded bar is not a path")
            };
            assert_eq!(commands.len(), 10);
            assert_eq!(
                commands
                    .iter()
                    .filter(|c| matches!(c, glory::render::PathCommand::Cubic { .. }))
                    .count(),
                4
            );
            let Geometry::Rect {
                x,
                y,
                width,
                height,
            } = old.geometry
            else {
                panic!()
            };
            let mut mark_scene = c.scene().unwrap();
            mark_scene.marks.push(m.clone());
            assert!(
                mark_scene
                    .hit_test(x + width / 2., y + height / 2.)
                    .is_some()
            );
            assert!(mark_scene.hit_test(x + 0.1, y + 0.1).is_none());
        }
        render::to_svg(&styled).unwrap();
    }
    assert!(
        bar(
            &c,
            &[],
            &[],
            &BarOptions {
                corner_radius: f64::NAN,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        bar(
            &c,
            &[],
            &[],
            &BarOptions {
                border: Some(GraphicStroke {
                    width: 0.,
                    ..border
                }),
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[test]
fn combo_rounded_bars_and_gradients_share_renderers_without_reordering() {
    let mut s = combo(
        &ChartConfig::default(),
        &[ComboSeries::new(
            "series",
            ComboKind::Bar,
            vec![ComboPoint::new(1., 10.), ComboPoint::new(2., 20.)],
        )],
        &ComboOptions {
            bar_corner_radius: 500.,
            bar_border: Some(GraphicStroke {
                color: Color(20, 20, 20, 255),
                width: 1.,
                cap: glory::render::StrokeCap::Butt,
            }),
            ..Default::default()
        },
    )
    .unwrap();
    let ids = s.marks.iter().map(|m| m.id.clone()).collect::<Vec<_>>();
    for m in s.marks.iter_mut().filter(|m| !m.series.is_empty()) {
        assert!(matches!(m.geometry, Geometry::Path(_)));
        m.fill_paint = Some(paint());
    }
    assert_eq!(
        s.marks.iter().map(|m| m.id.clone()).collect::<Vec<_>>(),
        ids
    );
    s.validate().unwrap();
    assert!(render::to_svg(&s).unwrap().contains("linearGradient"));
}
fn stops() -> Vec<GradientStop> {
    vec![
        GradientStop::new(0., Color(255, 0, 0, 255)),
        GradientStop::new(1., Color(0, 0, 255, 255)),
    ]
}
fn paint() -> FillPaint {
    FillPaint::Linear {
        start: [0., 0.],
        end: [100., 0.],
        stops: stops(),
    }
}
fn scene() -> Scene {
    let mut s = ChartConfig {
        width: 100.,
        height: 100.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    let mut m = Mark::new(
        "fill",
        Geometry::Rect {
            x: 0.,
            y: 0.,
            width: 100.,
            height: 100.,
        },
        Color(0, 0, 0, 255),
    );
    m.fill = None;
    m.fill_paint = Some(paint());
    m.series = "data".into();
    s.marks.push(m);
    s
}
#[derive(Default)]
struct Unsupported {
    begun: bool,
}
impl Canvas for Unsupported {
    fn begin(&mut self, _: f64, _: f64, _: Color) -> Result<()> {
        self.begun = true;
        Ok(())
    }
    fn begin_path(&mut self) -> Result<()> {
        Ok(())
    }
    fn move_to(&mut self, _: f64, _: f64) -> Result<()> {
        Ok(())
    }
    fn line_to(&mut self, _: f64, _: f64) -> Result<()> {
        Ok(())
    }
    fn cubic_to(&mut self, _: f64, _: f64, _: f64, _: f64, _: f64, _: f64) -> Result<()> {
        Ok(())
    }
    fn close_path(&mut self) -> Result<()> {
        Ok(())
    }
    fn fill(&mut self, _: Color, _: f64) -> Result<()> {
        Ok(())
    }
    fn stroke(&mut self, _: GraphicStroke, _: f64) -> Result<()> {
        Ok(())
    }
    fn text(&mut self, _: &Label) -> Result<()> {
        Ok(())
    }
    fn finish(&mut self) -> Result<()> {
        Ok(())
    }
}
#[test]
fn default_canvas_rejects_translucent_zero_dash_union_before_begin() {
    let mut s = ChartConfig::default().scene().unwrap();
    let mut mark = Mark::new(
        "dash",
        Geometry::Path(vec![
            glory::render::PathCommand::Move(glory::render::GraphicPoint::new(10., 50.)),
            glory::render::PathCommand::Line(glory::render::GraphicPoint::new(90., 50.)),
        ]),
        Color(255, 0, 0, 128),
    );
    mark.fill = None;
    mark.stroke = Some(GraphicStroke {
        color: Color(255, 0, 0, 128),
        width: 10.,
        cap: glory::render::StrokeCap::Round,
    });
    mark.stroke_dash = Some(StrokeDash {
        segments: vec![0., 6., 8., 6.],
        offset: 0.,
    });
    s.marks.push(mark);
    let mut backend = Unsupported::default();
    assert!(
        render::draw(&s, &mut backend)
            .unwrap_err()
            .0
            .contains("translucent zero-dash")
    );
    assert!(!backend.begun);
    s.marks[0].stroke.as_mut().unwrap().color.3 = 255;
    render::draw(&s, &mut backend).unwrap();
    assert!(backend.begun);
}
#[test]
fn full_validation_and_backend_admission_happen_before_begin() {
    let s = scene();
    s.validate().unwrap();
    let mut backend = Unsupported::default();
    assert!(render::draw(&s, &mut backend).is_err());
    assert!(!backend.begun);
    for invalid in [
        FillPaint::Linear {
            start: [0., 0.],
            end: [0., 0.],
            stops: stops(),
        },
        FillPaint::Radial {
            center: [0., 0.],
            radius: 0.,
            stops: stops(),
        },
        FillPaint::Linear {
            start: [0., 0.],
            end: [1., 0.],
            stops: vec![
                GradientStop::new(f64::NAN, Color(0, 0, 0, 0)),
                GradientStop::new(1., Color(0, 0, 0, 0)),
            ],
        },
        FillPaint::Linear {
            start: [0., 0.],
            end: [1., 0.],
            stops: vec![
                GradientStop::new(0.5, Color(0, 0, 0, 0)),
                GradientStop::new(1., Color(0, 0, 0, 0)),
            ],
        },
    ] {
        let mut bad = s.clone();
        bad.marks[0].fill_paint = Some(invalid);
        assert!(bad.validate().is_err());
    }
    let mut many = s.clone();
    let stops = (0..32)
        .map(|i| GradientStop::new(i as f64 / 31., Color(30, 60, 90, 255)))
        .collect::<Vec<_>>();
    many.marks[0].fill_paint = Some(FillPaint::Linear {
        start: [0., 0.],
        end: [100., 0.],
        stops,
    });
    many.marks = (0..512)
        .map(|i| {
            let mut m = many.marks[0].clone();
            m.id = i.to_string();
            m
        })
        .collect();
    many.validate().unwrap();
    let mut extra = many.marks[0].clone();
    extra.id = "overflow".into();
    many.marks.push(extra);
    assert!(many.validate().is_err());
}
#[test]
fn svg_deduplicates_paints_deterministically_and_distinguishes_embedded_definitions() {
    let mut s = scene();
    let mut second = s.marks[0].clone();
    second.id = "second".into();
    s.marks.push(second);
    let svg = render::to_svg(&s).unwrap();
    assert_eq!(svg, render::to_svg(&s).unwrap());
    assert_eq!(svg.matches("<linearGradient ").count(), 1);
    assert_eq!(svg.matches("fill=\"url(#").count(), 2);
    if let Some(FillPaint::Linear { start, .. }) = &mut s.marks[1].fill_paint {
        start[0] = -0.;
    }
    assert_eq!(
        render::to_svg(&s)
            .unwrap()
            .matches("<linearGradient ")
            .count(),
        1
    );
    assert!(svg.contains("gradientUnits=\"userSpaceOnUse\""));
    s.marks[1].fill_paint = Some(FillPaint::Radial {
        center: [50., 50.],
        radius: 40.,
        stops: stops(),
    });
    let svg = render::to_svg(&s).unwrap();
    assert_eq!(svg.matches("<radialGradient ").count(), 1);
    assert_eq!(svg.matches("<stop ").count(), 4);
    let script = render::to_canvas_script(&s, "target").unwrap();
    assert!(
        script.contains("createLinearGradient(0,0,100,0)")
            && script.contains("createRadialGradient(50,50,0,50,50,40)")
    );
}
#[test]
fn paint_only_marks_participate_in_hit_and_explicit_style_changes_fade() {
    let s = scene();
    let mut state = interaction::Interaction::default();
    assert_eq!(state.pointer(&s, 50., 50.), Some("fill"));
    assert!(s.interpolate(&s, 0.5).is_ok());
    let mut target = s.clone();
    target.marks[0].fill_paint = Some(FillPaint::Radial {
        center: [50., 50.],
        radius: 50.,
        stops: stops(),
    });
    assert!(s.interpolate(&target, 0.5).is_err());
    let mut motion = motion::PropertyMotion::new(s.clone(), Default::default()).unwrap();
    motion.retarget(target.clone()).unwrap();
    assert_eq!(motion.stats().marks, 2);
    motion.finish();
    assert_eq!(motion.scene(), &target);
    let mut legacy = motion::SpringMotion::new(s, Default::default()).unwrap();
    legacy.retarget(target.clone()).unwrap();
    legacy.tick(0.1).unwrap().validate().unwrap();
    legacy.finish();
    assert_eq!(legacy.scene(), &target);
}
#[cfg(feature = "raster")]
mod raster {
    use render::raster::*;

    use super::*;
    fn pixel(im: &RenderedImage, x: u32, y: u32) -> [u8; 4] {
        im.rgba[((y * im.width + x) * 4) as usize..((y * im.width + x) * 4 + 4) as usize]
            .try_into()
            .unwrap()
    }
    #[test]
    fn linear_radial_and_alpha_render_real_rgba_in_scene_coordinates() {
        let mut s = scene();
        let image = render_raster(&s, &FontRegistry::new(), &Default::default()).unwrap();
        let p = pixel(&image, 50, 50);
        assert!(p[0].abs_diff(126) < 3 && p[2].abs_diff(129) < 3 && p[1] == 0);
        s.marks[0].fill_paint = Some(FillPaint::Radial {
            center: [50., 50.],
            radius: 40.,
            stops: stops(),
        });
        let image = render_raster(&s, &FontRegistry::new(), &Default::default()).unwrap();
        assert!(pixel(&image, 50, 50)[0] > 245);
        assert!(pixel(&image, 90, 50)[2] > 250);
        s.marks[0].fill_paint = Some(FillPaint::Linear {
            start: [0., 0.],
            end: [100., 0.],
            stops: vec![
                GradientStop::new(0., Color(255, 0, 0, 255)),
                GradientStop::new(1., Color(255, 0, 0, 0)),
            ],
        });
        s.marks[0].opacity = 0.5;
        let image = render_raster(&s, &FontRegistry::new(), &Default::default()).unwrap();
        let p = pixel(&image, 50, 50);
        assert_eq!(p[0], 255);
        assert!(p[1].abs_diff(192) < 3 && p[2].abs_diff(192) < 3);
    }
    #[test]
    fn gradient_transform_clip_and_failed_frame_are_atomic() {
        let mut s = scene();
        s.marks[0].clip = Some(ClipRect {
            x: 10.,
            y: 10.,
            width: 50.,
            height: 50.,
        });
        let options = RasterOptions {
            pixel_ratio: 2.,
            view: ZoomTransform::new(1., 20., 0.).unwrap(),
            ..Default::default()
        };
        let mut canvas = RasterCanvas::new(FontRegistry::new(), options).unwrap();
        let im = canvas.render(&s).unwrap().clone();
        assert_eq!(pixel(&im, 20, 40), [255; 4]);
        let p = pixel(&im, 140, 40);
        assert!(p[2] > 120 && p[0] < 140);
        assert_eq!(pixel(&im, 170, 40), [255; 4]);
        let mut bad = s.clone();
        bad.marks[0].fill_paint = Some(FillPaint::Radial {
            center: [999_999., 0.],
            radius: 10.,
            stops: stops(),
        });
        assert!(canvas.render(&bad).is_err());
        assert_eq!(canvas.frame(), Some(&im));
    }
}
