use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use kartis::render::{
    self, Canvas, CanvasScriptFallback, CanvasScriptOptions, CanvasScriptSurface,
};
use kartis::*;

fn fixture() -> Scene {
    let mut scene = ChartConfig {
        width: 100.,
        height: 80.,
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
    let mut m = Mark::new(
        "mixed",
        Geometry::Rect {
            x: 20.,
            y: 20.,
            width: 60.,
            height: 40.,
        },
        Color(0, 180, 70, 255),
    );
    m.stroke = Some(GraphicStroke {
        color: Color(200, 20, 180, 190),
        width: 6.,
        cap: StrokeCap::Round,
    });
    m.opacity = 0.5;
    scene.marks.push(m);
    scene
}
#[derive(Default)]
struct Probe {
    group: bool,
    fail_stroke: bool,
    events: Vec<String>,
    paint_opacity: Vec<f64>,
    composites: Vec<(BlendMode, f64, bool)>,
}
impl Canvas for Probe {
    fn supports_group_opacity(&self) -> bool {
        self.group
    }
    // Deliberately models an old adapter accepting empty Normal profiles.
    fn prepare_mark_blends(&mut self, _: &Scene, _: &MarkBlends) -> Result<()> {
        self.events.push("prepare".into());
        Ok(())
    }
    fn begin_mark_blend(&mut self, _: usize) -> Result<bool> {
        self.events.push("layer".into());
        Ok(true)
    }
    fn end_mark_blend(&mut self, m: BlendMode, o: f64, c: bool) -> Result<()> {
        self.composites.push((m, o, c));
        Ok(())
    }
    fn supports_fill_paint(&self) -> bool {
        true
    }
    fn fill_paint(&mut self, _: &FillPaint, o: f64) -> Result<()> {
        self.paint_opacity.push(o);
        Ok(())
    }
    fn supports_clipping(&self) -> bool {
        true
    }
    fn push_clip(&mut self, _: ClipRect) -> Result<()> {
        self.events.push("clip".into());
        Ok(())
    }
    fn pop_clip(&mut self) -> Result<()> {
        self.events.push("pop".into());
        Ok(())
    }
    fn begin(&mut self, _: f64, _: f64, _: Color) -> Result<()> {
        self.events.push("begin".into());
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
    fn fill(&mut self, _: Color, o: f64) -> Result<()> {
        self.paint_opacity.push(o);
        Ok(())
    }
    fn stroke(&mut self, _: GraphicStroke, o: f64) -> Result<()> {
        self.paint_opacity.push(o);
        if self.fail_stroke {
            Err(ChartError("stroke failed".into()))
        } else {
            Ok(())
        }
    }
    fn text(&mut self, _: &Label) -> Result<()> {
        Ok(())
    }
    fn finish(&mut self) -> Result<()> {
        self.events.push("finish".into());
        Ok(())
    }
}
#[test]
fn legacy_override_cannot_touch_destination_before_explicit_group_capability() {
    let scene = fixture();
    let mut legacy = Probe::default();
    assert!(
        render::draw(&scene, &mut legacy)
            .unwrap_err()
            .0
            .contains("object opacity")
    );
    assert!(legacy.events.is_empty());
    assert!(legacy.paint_opacity.is_empty());
}
#[test]
fn normal_and_artistic_groups_draw_each_paint_at_one_and_composite_once() -> Result<()> {
    for mode in [BlendMode::Normal, BlendMode::Multiply] {
        let scene = fixture();
        let modes = MarkBlends::new(&scene, [("mixed", mode)])?;
        let mut target = Probe {
            group: mode == BlendMode::Normal,
            ..Default::default()
        };
        render::draw_with_blends(&scene, &mut target, &modes)?;
        assert_eq!(target.paint_opacity, vec![1., 1.]);
        assert_eq!(target.composites, vec![(mode, 0.5, true)]);
        assert_eq!(target.events, vec!["prepare", "begin", "layer", "finish"]);
    }
    Ok(())
}
#[test]
fn rejected_source_discards_group_and_restores_destination_clip() {
    let mut scene = fixture();
    scene.marks[0].clip = Some(ClipRect {
        x: 10.,
        y: 10.,
        width: 80.,
        height: 60.,
    });
    let mut target = Probe {
        group: true,
        fail_stroke: true,
        ..Default::default()
    };
    assert!(render::draw(&scene, &mut target).is_err());
    assert_eq!(target.composites, vec![(BlendMode::Normal, 0.5, false)]);
    assert_eq!(
        target.events,
        vec!["prepare", "begin", "clip", "layer", "pop", "finish"]
    );
}
#[test]
fn single_paint_zero_opacity_and_normalized_points_keep_legacy_fast_path() -> Result<()> {
    let mut variants = Vec::new();
    for mode in 0..9 {
        let mut s = fixture();
        let m = &mut s.marks[0];
        match mode {
            0 => m.stroke = None,
            1 => m.fill = None,
            2 => m.opacity = 0.,
            3 => m.opacity = 1.,
            4 => m.stroke.as_mut().unwrap().color.3 = 0,
            5 => m.fill = Some(Color(9, 8, 7, 0)),
            6 => {
                m.geometry = Geometry::Rect {
                    x: 20.,
                    y: 20.,
                    width: 0.,
                    height: 40.,
                }
            }
            7 => {
                m.geometry = Geometry::Circle {
                    x: 40.,
                    y: 40.,
                    radius: 0.,
                }
            }
            _ => {
                m.geometry = Geometry::Path(vec![
                    C::Move(P::new(30., 30.)),
                    C::Cubic {
                        first: P::new(30., 30.),
                        second: P::new(30., 30.),
                        end: P::new(30., 30.),
                    },
                    C::Close,
                ])
            }
        }
        variants.push(s);
    }
    for s in variants {
        let mut target = Probe::default();
        render::draw(&s, &mut target)?;
        assert!(target.composites.is_empty());
        let script = render::to_canvas_script(&s, "c")?;
        assert!(!script.contains("blendScratch"));
    }
    Ok(())
}
#[test]
fn normal_scripts_admit_scratch_before_resize_and_charge_offscreen_aggregate() -> Result<()> {
    let scene = fixture();
    let direct = render::to_canvas_script(&scene, "c")?;
    assert!(direct.contains("const blendModes=['source-over',]"));
    assert!(
        direct.find("blend scratch pixel budget").unwrap() < direct.find("canvas.width=").unwrap()
    );
    assert!(
        direct.find("blendSource.getImageData").unwrap() < direct.find("canvas.width=").unwrap()
    );
    assert_eq!(direct.matches("ctx.drawImage(blendScratch").count(), 1);
    assert!(direct.contains("ctx.globalAlpha=0.5;ctx.imageSmoothingEnabled=false;ctx.drawImage"));
    for fallback in [CanvasScriptFallback::Direct, CanvasScriptFallback::Error] {
        let script = render::to_canvas_script_with_options(
            &scene,
            "c",
            &CanvasScriptOptions {
                surface: CanvasScriptSurface::Offscreen2d { fallback },
                max_extra_buffer_pixels: 1,
            },
        )?;
        assert!(script.contains("blendMaxW"));
        assert!(script.contains("extra buffer budget"));
        assert_eq!(script.matches("ctx.drawImage(blendScratch").count(), 1);
    }
    Ok(())
}
#[test]
fn paint_server_and_stroke_use_the_same_normal_group_protocol() -> Result<()> {
    let mut scene = fixture();
    scene.marks[0].fill = None;
    scene.marks[0].fill_paint = Some(FillPaint::Linear {
        start: [0., 0.],
        end: [1., 0.],
        stops: vec![
            GradientStop::new(0., Color(0, 180, 70, 255)),
            GradientStop::new(1., Color(0, 180, 70, 255)),
        ],
    });
    let mut target = Probe {
        group: true,
        ..Default::default()
    };
    render::draw(&scene, &mut target)?;
    assert_eq!(target.paint_opacity, vec![1., 1.]);
    assert_eq!(target.composites, vec![(BlendMode::Normal, 0.5, true)]);
    Ok(())
}

#[cfg(feature = "raster")]
mod cpu {
    use render::raster::*;

    use super::*;
    fn pixel(image: &RenderedImage, x: usize, y: usize) -> [u8; 4] {
        image.rgba[(y * image.width as usize + x) * 4..(y * image.width as usize + x) * 4 + 4]
            .try_into()
            .unwrap()
    }
    fn close(a: [u8; 4], b: [u8; 4]) {
        assert!(
            a.into_iter().zip(b).all(|(a, b)| a.abs_diff(b) <= 1),
            "{a:?} != {b:?}"
        );
    }
    #[test]
    fn overlap_applies_object_alpha_once_over_opaque_and_transparent_backgrounds() -> Result<()> {
        let mut scene = fixture();
        for transparent in [false, true] {
            if transparent {
                scene.background = Color(0, 0, 0, 0);
            }
            let image = render_raster(&scene, &FontRegistry::new(), &Default::default())?;
            // Exact source-over: stroke(200,20,180,190) over opaque
            // fill(0,180,70), then object alpha .5. Old per-paint alpha
            // gives the distinctly different opaque result [155,144,169].
            let overlap = pixel(&image, 50, 21);
            if transparent {
                assert_eq!(overlap[3], 128);
                // Tiny-skia stores eight-bit premultiplied channels. At alpha
                // 128, one premultiplied unit can become two straight-RGB
                // units on readback. Check the independent exact source-over
                // numerators in their native premultiplied units instead.
                let expected_premul = [
                    200. * 190.,
                    20. * 190. + 180. * 65.,
                    180. * 190. + 70. * 65.,
                ];
                for (channel, numerator) in expected_premul.into_iter().enumerate() {
                    let actual = f64::from(overlap[channel]) * f64::from(overlap[3]) / 255.;
                    assert!((actual - numerator / 510.).abs() <= 1.);
                }
            } else {
                close(overlap, [202, 158, 203, 255]);
            }
            close(
                pixel(&image, 50, 40),
                if transparent {
                    [0, 180, 70, 128]
                } else {
                    [128, 218, 163, 255]
                },
            );
        }
        Ok(())
    }
    #[test]
    fn rect_and_path_masks_apply_only_on_final_group_composite() -> Result<()> {
        let mut scene = fixture();
        let m = &mut scene.marks[0];
        m.clip = Some(ClipRect {
            x: 25.,
            y: 0.,
            width: 70.,
            height: 80.,
        });
        m.clip_path = Some(Box::new(ClipPath {
            geometry: Geometry::Path(vec![
                C::Move(P::new(20., 0.)),
                C::Line(P::new(100., 0.)),
                C::Line(P::new(20., 80.)),
                C::Close,
            ]),
            rule: ClipRule::NonZero,
        }));
        let mut reference = scene.clone();
        let m = &mut reference.marks[0];
        m.fill = Some(Color(149, 61, 152, 255));
        m.stroke = None;
        let actual = render_raster(&scene, &FontRegistry::new(), &Default::default())?;
        let expected = render_raster(&reference, &FontRegistry::new(), &Default::default())?;
        // Diagonal mask crosses the fully covered fill/stroke overlap.
        for x in 75..81 {
            close(pixel(&actual, x, 21), pixel(&expected, x, 21));
        }
        assert_eq!(pixel(&actual, 22, 40), [255, 255, 255, 255]);
        Ok(())
    }
    #[test]
    fn normal_group_budget_failure_preserves_previous_accepted_frame() -> Result<()> {
        let mut canvas = RasterCanvas::new(FontRegistry::new(), Default::default())?;
        let accepted = canvas.render(&fixture())?.clone();
        let mut large = fixture();
        large.width = 5000.;
        large.height = 4000.;
        large.marks[0].geometry = Geometry::Rect {
            x: 0.,
            y: 0.,
            width: 5000.,
            height: 4000.,
        };
        assert!(
            canvas
                .render(&large)
                .unwrap_err()
                .0
                .contains("scratch pixel budget")
        );
        assert_eq!(canvas.frame(), Some(&accepted));
        canvas.set_options(RasterOptions {
            max_operations: 1,
            ..Default::default()
        })?;
        assert!(canvas.render(&fixture()).is_err());
        assert_eq!(canvas.frame(), Some(&accepted));
        canvas.set_options(Default::default())?;
        assert_eq!(canvas.render(&fixture())?, &accepted);
        Ok(())
    }
    #[test]
    fn uniform_gradient_group_equals_solid_and_normal_explicit_map_is_unchanged() -> Result<()> {
        let mut scene = fixture();
        let plain = render_raster(&scene, &FontRegistry::new(), &Default::default())?;
        scene.marks[0].fill_paint = Some(FillPaint::Linear {
            start: [0., 0.],
            end: [1., 0.],
            stops: vec![
                GradientStop::new(0., Color(0, 180, 70, 255)),
                GradientStop::new(1., Color(0, 180, 70, 255)),
            ],
        });
        let image = render_raster(&scene, &FontRegistry::new(), &Default::default())?;
        assert_eq!(image.rgba, plain.rgba);
        let modes = MarkBlends::new(&scene, [("mixed", BlendMode::Normal)])?;
        assert_eq!(
            render_raster_with_blends(&scene, &FontRegistry::new(), &Default::default(), &modes)?
                .rgba,
            image.rgba
        );
        Ok(())
    }
}
