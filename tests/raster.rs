#![cfg(feature = "raster")]
use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as Cmd, StrokeCap};
use kartis::render::Canvas;
use kartis::render::raster::*;
use kartis::*;

fn fonts() -> FontRegistry {
    let mut fonts = FontRegistry::new();
    fonts
        .register(include_bytes!("fonts/ABeeZee-Regular.ttf").as_slice(), 0)
        .unwrap();
    fonts
}
fn scene() -> Scene {
    ChartConfig {
        width: 100.,
        height: 100.,
        ..Default::default()
    }
    .scene()
    .unwrap()
}
fn rect(id: &str, x: f64, y: f64, width: f64, height: f64, color: Color) -> Mark {
    Mark::new(
        id,
        Geometry::Rect {
            x,
            y,
            width,
            height,
        },
        color,
    )
}
fn pixel(image: &RenderedImage, x: u32, y: u32) -> [u8; 4] {
    image.rgba[((y * image.width + x) * 4) as usize..((y * image.width + x) * 4 + 4) as usize]
        .try_into()
        .unwrap()
}
fn label(text: &str, x: f64, y: f64, anchor: TextAnchor, rotation: f64) -> Label {
    Label {
        text: text.into(),
        x,
        y,
        anchor,
        rotation,
        size: 24.,
        color: Color(0, 0, 0, 255),
    }
}
#[test]
fn source_over_alpha_and_custom_scene_order_produce_straight_rgba() {
    let mut s = scene();
    s.background = Color(0, 0, 0, 0);
    s.marks
        .push(rect("red", 10., 10., 60., 60., Color(255, 0, 0, 255)));
    let mut blue = rect("blue", 30., 30., 60., 60., Color(0, 0, 255, 255));
    blue.opacity = 0.5;
    s.marks.push(blue);
    let image = render_raster(&s, &FontRegistry::new(), &Default::default()).unwrap();
    assert_eq!(pixel(&image, 20, 20), [255, 0, 0, 255]);
    let overlap = pixel(&image, 40, 40);
    assert!(overlap[0].abs_diff(127) <= 1 && overlap[2].abs_diff(128) <= 1);
    assert_eq!(overlap[3], 255);
    let translucent = pixel(&image, 80, 80);
    assert_eq!(&translucent[..3], &[0, 0, 255]);
    assert!(translucent[3].abs_diff(128) <= 1);
    assert_eq!(pixel(&image, 0, 0), [0, 0, 0, 0]);
    s.draw_order = vec![DrawItem::Mark(1), DrawItem::Mark(0)];
    let reversed = render_raster(&s, &FontRegistry::new(), &Default::default()).unwrap();
    assert_eq!(pixel(&reversed, 40, 40), [255, 0, 0, 255]);
}
#[test]
fn cubic_paths_clip_stroke_caps_and_holes_are_rasterized() {
    let mut s = scene();
    let mut fill = rect("clipped", 0., 0., 100., 100., Color(220, 20, 30, 255));
    fill.clip = Some(ClipRect {
        x: 20.,
        y: 20.,
        width: 20.,
        height: 20.,
    });
    s.marks.push(fill);
    let mut curve = Mark::new(
        "curve",
        Geometry::Path(vec![
            Cmd::Move(P::new(10., 70.)),
            Cmd::Cubic {
                first: P::new(10., 40.),
                second: P::new(90., 40.),
                end: P::new(90., 70.),
            },
        ]),
        Color(0, 0, 0, 255),
    );
    curve.fill = None;
    curve.stroke = Some(GraphicStroke {
        color: Color(0, 80, 200, 255),
        width: 8.,
        cap: StrokeCap::Round,
    });
    s.marks.push(curve);
    let image = render_raster(&s, &FontRegistry::new(), &Default::default()).unwrap();
    assert_eq!(pixel(&image, 30, 30), [220, 20, 30, 255]);
    assert_eq!(pixel(&image, 10, 10), [255, 255, 255, 255]);
    assert_eq!(pixel(&image, 50, 48), [0, 80, 200, 255]);
    assert_eq!(pixel(&image, 8, 70), [0, 80, 200, 255]);
    assert_eq!(pixel(&image, 50, 70), [255, 255, 255, 255]);
    let mut zero = s.clone();
    zero.marks[0].clip = Some(ClipRect {
        x: 20.,
        y: 20.,
        width: 0.,
        height: 20.,
    });
    let image = render_raster(&zero, &FontRegistry::new(), &Default::default()).unwrap();
    assert_eq!(pixel(&image, 30, 30), [255, 255, 255, 255]);
    let ring = vec![
        Cmd::Move(P::new(10., 10.)),
        Cmd::Line(P::new(90., 10.)),
        Cmd::Line(P::new(90., 90.)),
        Cmd::Line(P::new(10., 90.)),
        Cmd::Close,
        Cmd::Move(P::new(30., 30.)),
        Cmd::Line(P::new(30., 70.)),
        Cmd::Line(P::new(70., 70.)),
        Cmd::Line(P::new(70., 30.)),
        Cmd::Close,
    ];
    s.marks = vec![Mark::new(
        "ring",
        Geometry::Path(ring),
        Color(0, 80, 200, 255),
    )];
    let image = render_raster(&s, &FontRegistry::new(), &Default::default()).unwrap();
    assert_eq!(pixel(&image, 20, 20), [0, 80, 200, 255]);
    assert_eq!(pixel(&image, 50, 50), [255, 255, 255, 255]);
}
#[test]
fn dpr_view_transform_and_clip_share_physical_coordinates() {
    let mut s = scene();
    let mut r = rect("view", 10., 10., 20., 20., Color(0, 100, 0, 255));
    r.clip = Some(ClipRect {
        x: 15.,
        y: 15.,
        width: 5.,
        height: 5.,
    });
    s.marks.push(r);
    let options = RasterOptions {
        pixel_ratio: 2.,
        view: ZoomTransform::new(2., 10., 5.).unwrap(),
        ..Default::default()
    };
    let image = render_raster(&s, &FontRegistry::new(), &options).unwrap();
    assert_eq!((image.width, image.height), (200, 200));
    assert_eq!(
        (image.logical_width, image.logical_height, image.pixel_ratio),
        (100., 100., 2.)
    );
    assert_eq!(pixel(&image, 90, 80), [0, 100, 0, 255]);
    assert_eq!(pixel(&image, 70, 60), [255, 255, 255, 255]);
    assert_eq!(pixel(&image, 105, 80), [255, 255, 255, 255]);
}
#[test]
fn png_data_url_roundtrip_preserves_straight_pixels() {
    let mut s = scene();
    s.background = Color(20, 40, 80, 128);
    let image = render_raster(&s, &FontRegistry::new(), &Default::default()).unwrap();
    let bytes = image.png().unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut pixels).unwrap();
    assert_eq!(&pixels[..info.buffer_size()], image.rgba.as_slice());
    assert!(
        image
            .data_url()
            .unwrap()
            .starts_with("data:image/png;base64,iVBORw0KGgo")
    );
}
#[test]
fn text_shapes_kerning_ligatures_combining_marks_and_positions_anchors() {
    let mut fonts = FontRegistry::new();
    fonts
        .register(include_bytes!("fonts/Lato-Regular.ttf").as_slice(), 0)
        .unwrap();
    let advance = |text: &str| {
        fonts
            .measure(text, 24., MissingGlyphPolicy::Error)
            .unwrap()
            .advance
    };
    assert!(
        advance("AV") < advance("A") + advance("V"),
        "font kerning must survive shaping"
    );
    let fi = fonts.measure("fi", 24., MissingGlyphPolicy::Error).unwrap();
    assert_eq!(fi.glyph_count, 1, "standard fi ligature");
    assert!(
        (advance("e\u{301}") - advance("é")).abs() < 1e-8,
        "combining acute forms the same advance"
    );
    let mut combining = scene();
    combining
        .labels
        .push(label("e\u{301}", 25., 55., TextAnchor::Start, 0.));
    let mut precomposed = combining.clone();
    precomposed.labels[0].text = "é".into();
    assert_eq!(
        render_raster(&combining, &fonts, &Default::default())
            .unwrap()
            .rgba,
        render_raster(&precomposed, &fonts, &Default::default())
            .unwrap()
            .rgba,
        "combining mark placement must match the equivalent precomposed outline, not only advance width"
    );
    let width = advance("AV fi");
    let mut a = scene();
    a.labels
        .push(label("AV fi", 15., 55., TextAnchor::Start, 0.));
    let mut b = scene();
    b.labels.push(label(
        "AV fi",
        15. + width / 2.,
        55.,
        TextAnchor::Middle,
        0.,
    ));
    let mut c = scene();
    c.labels
        .push(label("AV fi", 15. + width, 55., TextAnchor::End, 0.));
    let image = render_raster(&a, &fonts, &Default::default()).unwrap();
    assert_eq!(
        image.rgba,
        render_raster(&b, &fonts, &Default::default()).unwrap().rgba
    );
    assert_eq!(
        image.rgba,
        render_raster(&c, &fonts, &Default::default()).unwrap().rgba
    );
    assert!(
        image
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[0] < 200)
            .count()
            > 150,
        "text must paint glyph outline pixels"
    );
    a.labels[0].rotation = 90.;
    a.labels[0].x = 70.;
    a.labels[0].y = 20.;
    let rotated = render_raster(&a, &fonts, &Default::default()).unwrap();
    let bounds = |image: &RenderedImage| {
        let mut b = [u32::MAX, u32::MAX, 0, 0];
        for y in 0..image.height {
            for x in 0..image.width {
                if pixel(image, x, y)[0] < 200 {
                    b[0] = b[0].min(x);
                    b[1] = b[1].min(y);
                    b[2] = b[2].max(x);
                    b[3] = b[3].max(y);
                }
            }
        }
        b
    };
    let original = bounds(&image);
    let turned = bounds(&rotated);
    assert!((i64::from(original[2] - original[0]) - i64::from(turned[3] - turned[1])).abs() <= 1);
    assert!((i64::from(original[3] - original[1]) - i64::from(turned[2] - turned[0])).abs() <= 1);
}
#[test]
fn fonts_missing_glyph_and_complex_text_profiles_are_explicit() {
    let mut fonts = fonts();
    let count = fonts.len();
    assert!(fonts.register([0_u8; 20].as_slice(), 0).is_err());
    assert_eq!(fonts.len(), count);
    let mut s = scene();
    s.labels
        .push(label("Hello", 10., 40., TextAnchor::Start, 0.));
    assert!(render_raster(&s, &FontRegistry::new(), &Default::default()).is_err());
    s.labels[0].text = "界".into();
    assert!(render_raster(&s, &fonts, &Default::default()).is_err());
    let skip = RasterOptions {
        missing_glyph: MissingGlyphPolicy::Skip,
        ..Default::default()
    };
    let image = render_raster(&s, &fonts, &skip).unwrap();
    assert!(
        image
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| *p == [255, 255, 255, 255])
    );
    let notdef = RasterOptions {
        missing_glyph: MissingGlyphPolicy::Notdef,
        ..Default::default()
    };
    let image = render_raster(&s, &fonts, &notdef).unwrap();
    assert!(image.rgba.as_chunks::<4>().0.iter().any(|p| p[0] < 200));
    s.labels[0].text = "مرحبا".into();
    assert!(
        render_raster(&s, &fonts, &notdef).is_err(),
        "RTL is never mislabeled as shaped LTR"
    );
}
#[test]
fn rejected_scene_pixels_operations_and_outline_failures_keep_previous_frame() {
    let mut canvas = RasterCanvas::new(fonts(), Default::default()).unwrap();
    let mut s = scene();
    s.marks
        .push(rect("red", 5., 5., 90., 90., Color(255, 0, 0, 255)));
    canvas.render(&s).unwrap();
    let first = canvas.frame().unwrap().clone();
    let mut invalid = s.clone();
    invalid.marks[0].opacity = f64::NAN;
    assert!(canvas.render(&invalid).is_err());
    assert_eq!(canvas.frame(), Some(&first));
    invalid = s.clone();
    invalid
        .labels
        .push(label("界", 5., 20., TextAnchor::Start, 0.));
    assert!(canvas.render(&invalid).is_err());
    assert_eq!(canvas.frame(), Some(&first));
    canvas
        .set_options(RasterOptions {
            max_pixels: 10,
            ..Default::default()
        })
        .unwrap();
    assert!(canvas.render(&s).is_err());
    assert_eq!(canvas.frame(), Some(&first));
    canvas
        .set_options(RasterOptions {
            max_operations: 10_010,
            ..Default::default()
        })
        .unwrap();
    assert!(canvas.render(&s).is_err());
    assert_eq!(canvas.frame(), Some(&first));
    canvas
        .set_options(RasterOptions {
            view: ZoomTransform::new(1_000_000., 0., 0.).unwrap(),
            ..Default::default()
        })
        .unwrap();
    assert!(canvas.render(&s).is_err());
    assert_eq!(canvas.frame(), Some(&first));
    canvas.set_options(Default::default()).unwrap();
    canvas.render(&s).unwrap();
    assert_eq!(canvas.frame(), Some(&first));
    // Canvas adapter admission also leaves the old frame after primitive faults.
    canvas.begin(100., 100., Color(0, 0, 0, 255)).unwrap();
    canvas.begin_path().unwrap();
    assert!(canvas.line_to(f64::NAN, 0.).is_err());
    assert!(canvas.finish().is_err());
    assert_eq!(canvas.frame(), Some(&first));
}
#[test]
fn nested_clip_intersections_and_restore_do_not_leak_between_paths() {
    let mut canvas = RasterCanvas::new(FontRegistry::new(), Default::default()).unwrap();
    canvas.begin(100., 100., Color(255, 255, 255, 255)).unwrap();
    canvas
        .push_clip(ClipRect {
            x: 10.,
            y: 10.,
            width: 80.,
            height: 80.,
        })
        .unwrap();
    canvas
        .push_clip(ClipRect {
            x: 20.,
            y: 20.,
            width: 20.,
            height: 20.,
        })
        .unwrap();
    let paint = |canvas: &mut RasterCanvas, color: Color| {
        canvas.begin_path().unwrap();
        canvas.move_to(0., 0.).unwrap();
        canvas.line_to(100., 0.).unwrap();
        canvas.line_to(100., 100.).unwrap();
        canvas.line_to(0., 100.).unwrap();
        canvas.close_path().unwrap();
        canvas.fill(color, 1.).unwrap();
    };
    paint(&mut canvas, Color(200, 0, 0, 255));
    canvas.pop_clip().unwrap();
    paint(&mut canvas, Color(0, 0, 200, 128));
    canvas.pop_clip().unwrap();
    canvas.finish().unwrap();
    let image = canvas.frame().unwrap();
    assert_eq!(pixel(image, 5, 5), [255, 255, 255, 255]);
    assert!(pixel(image, 15, 15)[2] > 200);
    assert!(pixel(image, 30, 30)[0] > 80 && pixel(image, 30, 30)[0] < 120);
}

#[test]
fn explicit_fallback_fonts_shape_their_own_runs_without_system_font_lookup() {
    let mut primary = fonts();
    assert!(
        primary
            .measure("Ω", 24., MissingGlyphPolicy::Error)
            .is_err()
    );
    primary
        .register(include_bytes!("fonts/Lato-Regular.ttf").as_slice(), 0)
        .unwrap();
    let mut fallback = FontRegistry::new();
    fallback
        .register(include_bytes!("fonts/Lato-Regular.ttf").as_slice(), 0)
        .unwrap();
    let fallback_width = fallback
        .measure("Ω", 24., MissingGlyphPolicy::Error)
        .unwrap()
        .advance;
    assert_eq!(
        primary
            .measure("Ω", 24., MissingGlyphPolicy::Error)
            .unwrap()
            .advance,
        fallback_width
    );
    let first = fonts()
        .measure("A", 24., MissingGlyphPolicy::Error)
        .unwrap()
        .advance;
    assert!(
        (primary
            .measure("AΩ", 24., MissingGlyphPolicy::Error)
            .unwrap()
            .advance
            - first
            - fallback_width)
            .abs()
            < 1e-8
    );
    let mut s = scene();
    s.labels.push(label("Ω", 20., 55., TextAnchor::Start, 0.));
    assert_eq!(
        render_raster(&s, &primary, &Default::default())
            .unwrap()
            .rgba,
        render_raster(&s, &fallback, &Default::default())
            .unwrap()
            .rgba
    );
}
