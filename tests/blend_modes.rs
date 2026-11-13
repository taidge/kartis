use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use kartis::*;

fn rect(x: f64, y: f64, width: f64, height: f64) -> Geometry {
    Geometry::Rect {
        x,
        y,
        width,
        height,
    }
}

fn fixture() -> Scene {
    let mut scene = ChartConfig {
        width: 120.,
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
    scene.background = Color(160, 120, 80, 255);
    let mut mark = Mark::new("mixed", rect(34., 20., 42., 40.), Color(200, 100, 50, 128));
    mark.opacity = 0.5;
    mark.stroke = Some(GraphicStroke {
        color: Color(40, 200, 240, 128),
        width: 12.,
        cap: StrokeCap::Butt,
    });
    mark.clip = Some(ClipRect {
        x: 30.,
        y: 8.,
        width: 60.,
        height: 64.,
    });
    mark.clip_path = Some(Box::new(ClipPath {
        geometry: Geometry::Path(vec![
            C::Move(P::new(22., 10.)),
            C::Line(P::new(95., 10.)),
            C::Line(P::new(95., 70.)),
            C::Line(P::new(22., 70.)),
            C::Close,
            C::Move(P::new(33., 36.)),
            C::Line(P::new(38., 36.)),
            C::Line(P::new(38., 44.)),
            C::Line(P::new(33., 44.)),
            C::Close,
        ]),
        rule: ClipRule::EvenOdd,
    }));
    scene.marks = vec![
        mark,
        Mark::new("normal", rect(98., 20., 10., 10.), Color(22, 41, 80, 255)),
    ];
    scene
}

#[test]
fn profile_admission_uses_business_keys_and_keeps_normal_output_identical() {
    let scene = fixture();
    let normal = MarkBlends::new(&scene, [("mixed", BlendMode::Normal)]).unwrap();
    assert!(normal.is_empty());
    assert!(MarkBlends::new(&scene, [("absent", BlendMode::Multiply)]).is_err());
    assert!(
        MarkBlends::new(
            &scene,
            [("mixed", BlendMode::Normal), ("mixed", BlendMode::Multiply)]
        )
        .is_err()
    );
    assert_eq!(
        render::to_svg_with_blends(&scene, &normal).unwrap(),
        render::to_svg(&scene).unwrap()
    );
    assert_eq!(
        render::to_canvas_script_with_blends(&scene, "plot", &normal).unwrap(),
        render::to_canvas_script(&scene, "plot").unwrap()
    );

    let multiply = MarkBlends::new(&scene, [("mixed", BlendMode::Multiply)]).unwrap();
    let mut filtered = scene.clone();
    filtered.marks.remove(0);
    assert!(render::to_svg_with_blends(&filtered, &multiply).is_err());
    assert!(multiply.for_scene(&filtered).is_empty());
}

#[test]
fn svg_layers_wrap_both_clips_and_canvas_prepares_before_resizing() {
    let scene = fixture();
    let profile = MarkBlends::new(&scene, [("mixed", BlendMode::Multiply)]).unwrap();
    let svg = render::to_svg_with_namespace_and_blends(&scene, "plot", &profile).unwrap();
    let group = svg.find("<g style=\"mix-blend-mode:multiply\">").unwrap();
    let mask = svg.find("<g clip-path=\"url(#kartis-ns-plot-").unwrap();
    let clip = svg.find("<svg x=\"30\"").unwrap();
    let ink = svg.find("<path data-id=\"mixed\"").unwrap();
    assert!(group < mask && mask < clip && clip < ink);
    assert!(svg[..group].contains("isolation:isolate"));
    let script = render::to_canvas_script_with_blends(&scene, "plot", &profile).unwrap();
    let resize = script.find("canvas.width=").unwrap();
    assert!(script.find("blend scratch pixel budget").unwrap() < resize);
    assert!(script.find("blendSource.getImageData").unwrap() < resize);
    assert_eq!(script.matches("ctx.drawImage(blendScratch").count(), 1);
}

#[cfg(feature = "raster")]
mod cpu {
    use render::raster::{
        FontRegistry, RasterCanvas, RasterOptions, RenderedImage, render_raster,
        render_raster_with_blends,
    };

    use super::*;

    fn pixel(image: &RenderedImage, x: usize, y: usize) -> [u8; 4] {
        let offset = (y * image.width as usize + x) * 4;
        image.rgba[offset..offset + 4].try_into().unwrap()
    }

    fn close(actual: [u8; 4], expected: [u8; 4]) {
        assert!(
            actual
                .into_iter()
                .zip(expected)
                .all(|(a, b)| a.abs_diff(b) <= 2),
            "{actual:?} != {expected:?}"
        );
    }

    #[test]
    fn transformed_compound_mark_blends_once_and_preserves_clip_holes() {
        let scene = fixture();
        let options = RasterOptions {
            pixel_ratio: 2.,
            view: ZoomTransform::new(1., 2., 1.).unwrap(),
            ..Default::default()
        };
        let profile = MarkBlends::new(&scene, [("mixed", BlendMode::Multiply)]).unwrap();
        let image =
            render_raster_with_blends(&scene, &FontRegistry::new(), &options, &profile).unwrap();
        let p = |x: usize, y: usize| pixel(&image, 2 * (x + 2), 2 * (y + 1));
        // Independent opaque-backdrop result. Fill+stroke have group alpha
        // .75 before Mark.opacity=.5, with unpremultiplied source RGB
        // [70,125,132.5]/.75. Applying Multiply to each paint gives a darker
        // and distinctly incorrect [119,96,63] at this overlap.
        close(p(35, 32), [122, 104, 71, 255]);
        close(p(32, 32), [126, 113, 79, 255]);
        close(p(52, 40), [151, 102, 64, 255]);
        assert_eq!(p(35, 40), [160, 120, 80, 255]);
        assert_eq!(p(28, 32), [160, 120, 80, 255]);
        assert_eq!(p(101, 22), [22, 41, 80, 255]);
    }

    #[test]
    fn all_artistic_modes_match_independent_canvas_samples() {
        // Chrome 157.0.8103.39. A 1×1 Canvas source was filled twice with
        // the fixture's 128/255-alpha colors. A separate Canvas destination
        // received one drawImage at globalAlpha=.5 and the given operation.
        // The two recorded RGBA values use opaque and half-alpha backdrops.
        let oracle = [
            (
                BlendMode::Multiply,
                [122, 104, 71, 255],
                [113, 120, 99, 176],
            ),
            (
                BlendMode::Screen,
                [173, 153, 125, 255],
                [151, 156, 139, 176],
            ),
            (
                BlendMode::Overlay,
                [150, 134, 91, 255],
                [135, 142, 114, 176],
            ),
            (BlendMode::Darken, [135, 120, 80, 255], [123, 132, 106, 176]),
            (
                BlendMode::Lighten,
                [160, 137, 116, 255],
                [141, 145, 132, 176],
            ),
            (
                BlendMode::ColorDodge,
                [195, 171, 146, 255],
                [167, 170, 154, 176],
            ),
            (BlendMode::ColorBurn, [99, 93, 50, 255], [97, 112, 84, 176]),
            (
                BlendMode::HardLight,
                [144, 135, 105, 255],
                [129, 143, 123, 176],
            ),
            (
                BlendMode::SoftLight,
                [154, 126, 89, 255],
                [136, 136, 112, 176],
            ),
            (
                BlendMode::Difference,
                [125, 92, 86, 255],
                [116, 112, 110, 176],
            ),
            (
                BlendMode::Exclusion,
                [151, 124, 105, 255],
                [135, 135, 123, 176],
            ),
            (BlendMode::Hue, [129, 130, 109, 255], [119, 139, 128, 176]),
            (
                BlendMode::Saturation,
                [160, 120, 79, 255],
                [142, 132, 106, 176],
            ),
            (BlendMode::Color, [128, 131, 109, 255], [117, 141, 128, 176]),
            (
                BlendMode::Luminosity,
                [167, 127, 87, 255],
                [146, 138, 110, 176],
            ),
        ];
        let fonts = FontRegistry::new();
        let options = RasterOptions::default();
        let mut scene = fixture();
        for (mode, solid, translucent) in oracle {
            let blend = MarkBlends::new(&scene, [("mixed", mode)]).unwrap();
            for (alpha, expected) in [(255, solid), (128, translucent)] {
                scene.background.3 = alpha;
                let image = render_raster_with_blends(&scene, &fonts, &options, &blend).unwrap();
                let actual = pixel(&image, 35, 32);
                assert!(
                    actual
                        .into_iter()
                        .zip(expected)
                        .all(|(a, b)| a.abs_diff(b) <= 2),
                    "{mode:?}, backdrop alpha {alpha}: {actual:?} != {expected:?}"
                );
            }
        }
    }

    #[test]
    fn blend_over_a_transparent_backdrop_retains_real_source_and_alpha() {
        let mut scene = fixture();
        scene.marks = vec![
            Mark::new("back", rect(20., 20., 40., 40.), Color(160, 120, 80, 128)),
            Mark::new("front", rect(40., 20., 40., 40.), Color(200, 100, 50, 128)),
        ];
        scene.background = Color(0, 0, 0, 0);
        let modes = MarkBlends::new(&scene, [("front", BlendMode::Multiply)]).unwrap();
        let img =
            render_raster_with_blends(&scene, &FontRegistry::new(), &Default::default(), &modes)
                .unwrap();
        close(pixel(&img, 50, 40), [162, 89, 49, 192]);
        close(pixel(&img, 75, 40), [200, 100, 50, 128]);
        assert_eq!(pixel(&img, 90, 40), [0, 0, 0, 0]);
    }

    #[test]
    fn scratch_origin_does_not_rebase_a_world_gradient_or_pattern() {
        let mut scene = fixture();
        scene.background = Color(0, 0, 0, 0);
        scene.marks = vec![
            Mark::new("gradient", rect(35., 13., 48., 18.), Color(0, 0, 0, 255)),
            Mark::new("pattern", rect(45., 41., 34., 24.), Color(0, 0, 0, 255)),
        ];
        scene.marks[0].fill_paint = Some(FillPaint::Linear {
            start: [10., 0.],
            end: [115., 0.],
            stops: vec![
                GradientStop::new(0., Color(40, 0, 0, 255)),
                GradientStop::new(1., Color(220, 200, 100, 255)),
            ],
        });
        scene.marks[1].fill_paint = Some(FillPaint::Pattern(PatternFill {
            offset: [3., 5.],
            ..Default::default()
        }));
        let options = RasterOptions {
            pixel_ratio: 2.,
            view: ZoomTransform::new(1.25, -3., -6.).unwrap(),
            ..Default::default()
        };
        let fonts = FontRegistry::new();
        let modes = MarkBlends::new(
            &scene,
            [
                ("gradient", BlendMode::Screen),
                ("pattern", BlendMode::Difference),
            ],
        )
        .unwrap();
        let normal = render_raster(&scene, &fonts, &options).unwrap();
        let blended = render_raster_with_blends(&scene, &fonts, &options, &modes).unwrap();
        let mut sum = 0u64;
        for (&a, &b) in normal.rgba.iter().zip(&blended.rgba) {
            sum += u64::from(a.abs_diff(b));
        }
        assert!((sum as f64 / normal.rgba.len() as f64) < 0.05);
        assert!(
            normal
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|p| p[3] > 0)
                .count()
                > 1000
        );
    }

    #[test]
    fn rejected_scratch_budget_keeps_the_accepted_raster_frame() {
        let mut canvas = RasterCanvas::new(FontRegistry::new(), RasterOptions::default()).unwrap();
        let accepted = canvas.render(&fixture()).unwrap().clone();
        let mut scene = ChartConfig {
            width: 5000.,
            height: 4000.,
            ..Default::default()
        }
        .scene()
        .unwrap();
        scene.marks.push(Mark::new(
            "too-large",
            rect(0., 0., 5000., 4000.),
            Color(0, 0, 0, 255),
        ));
        let modes = MarkBlends::new(&scene, [("too-large", BlendMode::Screen)]).unwrap();
        assert!(
            canvas
                .render_with_blends(&scene, &modes)
                .unwrap_err()
                .0
                .contains("scratch pixel budget")
        );
        assert_eq!(canvas.frame(), Some(&accepted));
    }
}
