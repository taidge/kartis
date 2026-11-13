use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use kartis::*;
fn scene() -> Scene {
    ChartConfig {
        width: 96.,
        height: 64.,
        margin: Margin {
            left: 0.,
            right: 0.,
            top: 0.,
            bottom: 0.,
        },
        ..Default::default()
    }
    .scene()
    .unwrap()
}
fn mark() -> Mark {
    let mut m = Mark::new(
        "m",
        Geometry::Rect {
            x: 0.,
            y: 0.,
            width: 96.,
            height: 64.,
        },
        Color(240, 30, 180, 255),
    );
    m.opacity = 0.4;
    m
}
fn path(svg: &str) -> &str {
    svg.split("<path data-id=\"m\"")
        .nth(1)
        .unwrap()
        .split('>')
        .next()
        .unwrap()
}
fn stroke() -> GraphicStroke {
    GraphicStroke {
        color: Color(10, 20, 30, 255),
        width: 8.,
        cap: StrokeCap::Round,
    }
}
fn gradient() -> FillPaint {
    FillPaint::Linear {
        start: [0., 0.],
        end: [96., 0.],
        stops: vec![
            GradientStop::new(0., Color(240, 30, 180, 255)),
            GradientStop::new(1., Color(10, 80, 120, 80)),
        ],
    }
}

#[test]
fn single_fill_uses_exact_original_factor_once_and_keeps_rgba_and_geometry() {
    for opacity in [-0., 0., 0.1, 0.4, 0.45, 0.5, 0.8] {
        for alpha in [0, 64, 160, 255] {
            let mut s = scene();
            let mut m = mark();
            m.opacity = opacity;
            m.fill = Some(Color(240, 30, 180, alpha));
            s.marks.push(m);
            let original = s.clone();
            let svg = render::to_svg(&s).unwrap();
            let p = path(&svg);
            assert!(p.contains(&format!(" fill-opacity=\"{opacity}\"")));
            assert!(!p.contains(" opacity=\"") && !p.contains("stroke-opacity"));
            assert!(p.contains(&format!(
                "fill=\"{}\"",
                render::css_color(Color(240, 30, 180, alpha))
            )));
            assert!(p.contains("M0 0L96 0L96 64L0 64Z"));
            assert_eq!(s, original);
        }
    }
}
#[test]
fn opacity_one_and_no_fill_keep_existing_element_attribute() {
    for (fill, opacity) in [(Some(Color(1, 2, 3, 255)), 1.), (None, 0.4)] {
        let mut s = scene();
        let mut m = mark();
        m.fill = fill;
        m.opacity = opacity;
        s.marks.push(m);
        let svg = render::to_svg(&s).unwrap();
        assert!(path(&svg).contains(&format!(" opacity=\"{opacity}\"")));
        assert!(!path(&svg).contains("fill-opacity"));
    }
}
#[test]
fn stroke_some_is_excluded_even_transparent_and_stroke_only_or_dashed() {
    for fill in [None, Some(Color(200, 50, 20, 255))] {
        for alpha in [0, 255] {
            for dashed in [false, true] {
                let mut s = scene();
                let mut m = mark();
                m.fill = fill;
                m.stroke = Some(GraphicStroke {
                    color: Color(10, 20, 30, alpha),
                    ..stroke()
                });
                if dashed {
                    m.stroke_dash = Some(StrokeDash {
                        segments: vec![4., 2.],
                        offset: 1.,
                    });
                }
                s.marks.push(m);
                let svg = render::to_svg(&s).unwrap();
                assert!(path(&svg).contains(" opacity=\"0.4\""));
                assert!(!path(&svg).contains("fill-opacity"));
            }
        }
    }
}
#[test]
fn gradients_patterns_and_all_non_normal_blends_keep_original_element_opacity() {
    let paints = [
        gradient(),
        FillPaint::Radial {
            center: [48., 32.],
            radius: 25.,
            stops: vec![
                GradientStop::new(0., Color(20, 30, 40, 255)),
                GradientStop::new(1., Color(40, 50, 60, 80)),
            ],
        },
        FillPaint::Pattern(PatternFill::default()),
    ];
    for paint in paints {
        let mut s = scene();
        let mut m = mark();
        m.fill_paint = Some(paint);
        s.marks.push(m);
        let svg = render::to_svg(&s).unwrap();
        assert!(path(&svg).contains(" opacity=\"0.4\""));
        assert!(!path(&svg).contains("fill-opacity"));
    }
    for mode in [
        BlendMode::Multiply,
        BlendMode::Screen,
        BlendMode::Overlay,
        BlendMode::Darken,
        BlendMode::Lighten,
        BlendMode::ColorDodge,
        BlendMode::ColorBurn,
        BlendMode::HardLight,
        BlendMode::SoftLight,
        BlendMode::Difference,
        BlendMode::Exclusion,
        BlendMode::Hue,
        BlendMode::Saturation,
        BlendMode::Color,
        BlendMode::Luminosity,
    ] {
        let mut s = scene();
        s.marks.push(mark());
        let blends = MarkBlends::new(&s, [("m", mode)]).unwrap();
        let svg = render::to_svg_with_blends(&s, &blends).unwrap();
        assert!(path(&svg).contains(" opacity=\"0.4\""));
        assert!(!path(&svg).contains("fill-opacity"));
        assert!(svg.contains(&format!("mix-blend-mode:{}", mode.css_name())));
    }
}
#[test]
fn clip_path_rect_clip_namespaces_and_multi_subpath_remain_one_fill() {
    let mut s = scene();
    let mut m = mark();
    m.geometry = Geometry::Path(vec![
        C::Move(P::new(0., 0.)),
        C::Line(P::new(64., 0.)),
        C::Line(P::new(64., 64.)),
        C::Close,
        C::Move(P::new(32., 0.)),
        C::Line(P::new(96., 0.)),
        C::Line(P::new(96., 64.)),
        C::Close,
    ]);
    m.clip = Some(ClipRect {
        x: 5.,
        y: 6.,
        width: 80.,
        height: 50.,
    });
    m.clip_path = Some(Box::new(ClipPath {
        geometry: Geometry::Circle {
            x: 48.,
            y: 32.,
            radius: 28.,
        },
        rule: ClipRule::EvenOdd,
    }));
    s.marks.push(m);
    let original = s.clone();
    let svg = render::to_svg_with_namespace(&s, "single").unwrap();
    assert!(path(&svg).contains(" fill-opacity=\"0.4\"") && !path(&svg).contains(" opacity=\""));
    assert!(
        svg.contains("clip-rule=\"evenodd\"") && svg.contains("clip-path=\"url(#kartis-ns-single-")
    );
    assert!(svg.contains("viewBox=\"5 6 80 50\"") && path(&svg).matches('M').count() == 2);
    assert_eq!(s, original);
}
#[test]
fn final_normalized_single_point_stroke_is_a_single_fill_but_blend_stays_excluded() {
    let mut s = scene();
    let mut m = mark();
    m.geometry = Geometry::Circle {
        x: 48.,
        y: 32.,
        radius: 0.,
    };
    m.fill = None;
    m.stroke = Some(stroke());
    s.marks.push(m);
    let svg = render::to_svg(&s).unwrap();
    assert!(path(&svg).contains(" fill-opacity=\"0.4\"") && !path(&svg).contains("stroke-width"));
    assert!(path(&svg).contains(&render::css_color(stroke().color)));
    let blend = MarkBlends::new(&s, [("m", BlendMode::Multiply)]).unwrap();
    let svg = render::to_svg_with_blends(&s, &blend).unwrap();
    assert!(path(&svg).contains(" opacity=\"0.4\""));
    assert!(!path(&svg).contains("fill-opacity"));
}
#[test]
fn invalid_scene_scope_is_not_reclassified_or_silently_repaired() {
    for opacity in [f64::NAN, f64::INFINITY, -1., 1.01] {
        let mut s = scene();
        let mut m = mark();
        m.opacity = opacity;
        s.marks.push(m);
        assert!(render::to_svg(&s).is_err());
        assert!(render::to_canvas_script(&s, "c").is_err());
    }
}
