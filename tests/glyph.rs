use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use kartis::*;
fn shape(name: &str) -> GlyphShape {
    match name {
        "fill-circle" => GlyphShape::Filled(FilledGlyph::Circle),
        "fill-cross" => GlyphShape::Filled(FilledGlyph::Cross),
        "fill-diamond" => GlyphShape::Filled(FilledGlyph::Diamond),
        "fill-square" => GlyphShape::Filled(FilledGlyph::Square),
        "fill-star" => GlyphShape::Filled(FilledGlyph::Star),
        "fill-triangle" => GlyphShape::Filled(FilledGlyph::Triangle),
        "fill-wye" => GlyphShape::Filled(FilledGlyph::Wye),
        "stroke-circle" => GlyphShape::Stroked(StrokedGlyph::Circle),
        "stroke-plus" => GlyphShape::Stroked(StrokedGlyph::Plus),
        "stroke-times" => GlyphShape::Stroked(StrokedGlyph::Times),
        "stroke-triangle" => GlyphShape::Stroked(StrokedGlyph::Triangle),
        "stroke-asterisk" => GlyphShape::Stroked(StrokedGlyph::Asterisk),
        "stroke-square" => GlyphShape::Stroked(StrokedGlyph::Square),
        "stroke-diamond" => GlyphShape::Stroked(StrokedGlyph::Diamond),
        _ => panic!("unknown oracle type"),
    }
}
fn names() -> [&'static str; 14] {
    [
        "fill-circle",
        "fill-cross",
        "fill-diamond",
        "fill-square",
        "fill-star",
        "fill-triangle",
        "fill-wye",
        "stroke-circle",
        "stroke-plus",
        "stroke-times",
        "stroke-triangle",
        "stroke-asterisk",
        "stroke-square",
        "stroke-diamond",
    ]
}
fn close(a: f64, b: f64) {
    assert!(
        (a - b).abs() <= 2e-12 * a.abs().max(b.abs()).max(1.),
        "{a} != {b}"
    );
}
fn geometry(name: &str, size: f64, rotation: f64) -> GlyphGeometry {
    glyph_geometry(
        [100., 100.],
        &GlyphOptions {
            shape: shape(name),
            size,
            rotation,
        },
    )
    .unwrap()
}
fn stroke() -> GraphicStroke {
    GraphicStroke {
        color: Color(30, 30, 30, 255),
        width: 1.5,
        cap: StrokeCap::Round,
    }
}
fn mark(g: &GlyphGeometry) -> Mark {
    glyph_mark(
        "glyph",
        g,
        if g.paint_mode == GlyphPaintMode::Fill {
            Some(Color(30, 30, 30, 255))
        } else {
            None
        },
        if g.paint_mode == GlyphPaintMode::Stroke {
            Some(stroke())
        } else {
            None
        },
    )
    .unwrap()
}
#[test]
fn actual_d3_recording_context_252_profiles_1440_records() {
    let mut profiles = 0;
    let mut current = usize::MAX;
    let mut index = 0;
    let mut glyph = None;
    let mut records = 0;
    for row in include_str!("fixtures/glyph-d3.csv").lines().skip(2) {
        let v = row.split('|').collect::<Vec<_>>();
        let id: usize = v[0].parse().unwrap();
        let size = v[2].parse().unwrap();
        let cx = v[3].parse().unwrap();
        let cy = v[4].parse().unwrap();
        let rotation = v[5].parse().unwrap();
        if id != current {
            current = id;
            index = 0;
            profiles += 1;
            glyph = Some(
                glyph_geometry(
                    [cx, cy],
                    &GlyphOptions {
                        shape: shape(v[1]),
                        size,
                        rotation,
                    },
                )
                .unwrap(),
            );
        }
        let g = glyph.as_ref().unwrap();
        let coordinates = if v[7].is_empty() {
            vec![]
        } else {
            v[7].split(',')
                .map(|s| s.parse::<f64>().unwrap())
                .collect::<Vec<_>>()
        };
        match &g.geometry {
            Geometry::Circle { x, y, radius } => match v[6] {
                "A" => {
                    close(*x, coordinates[0]);
                    close(*y, coordinates[1]);
                    close(*radius, coordinates[2]);
                }
                "M" => close((coordinates[0] - x).hypot(coordinates[1] - y), *radius),
                _ => panic!(),
            },
            Geometry::Path(p) => {
                match (&p[index], v[6]) {
                    (C::Move(p), "M") | (C::Line(p), "L") => {
                        close(p.x, coordinates[0]);
                        close(p.y, coordinates[1]);
                    }
                    (C::Close, "Z") => {}
                    _ => panic!("oracle topology mismatch {}", v[1]),
                };
                index += 1;
            }
            _ => panic!(),
        }
        records += 1;
    }
    assert_eq!(profiles, 252);
    assert_eq!(records, 1440);
}
fn area(g: &GlyphGeometry) -> f64 {
    match &g.geometry {
        Geometry::Circle { radius, .. } => std::f64::consts::PI * radius * radius,
        Geometry::Path(p) => {
            let v = p
                .iter()
                .filter_map(|c| match c {
                    C::Move(p) | C::Line(p) => Some([p.x - g.anchor[0], p.y - g.anchor[1]]),
                    _ => None,
                })
                .collect::<Vec<_>>();
            v.iter()
                .enumerate()
                .map(|(i, p)| {
                    let q = v[(i + 1) % v.len()];
                    p[0] * q[1] - p[1] * q[0]
                })
                .sum::<f64>()
                .abs()
                / 2.
        }
        _ => panic!(),
    }
}
#[test]
fn filled_area_rotation_real_origin_and_support_centroid() {
    for name in names().into_iter().take(7) {
        for size in [1., 7., 64., 1000.] {
            for rotation in [0., 0.37, -1.2] {
                let g = geometry(name, size, rotation);
                close(area(&g), size);
                g.validate().unwrap();
                if let Geometry::Path(ref path) = g.geometry {
                    let support =
                        InteractionAnchor::path_points(&g.geometry, g.anchor, &g.anchor_indices)
                            .unwrap();
                    assert_eq!(support.position(), [100., 100.]);
                    let changed = Geometry::Path(
                        path.iter()
                            .map(|c| match c {
                                C::Move(p) => C::Move(P::new(p.x + 17., p.y - 9.)),
                                C::Line(p) => C::Line(P::new(p.x + 17., p.y - 9.)),
                                C::Close => C::Close,
                                _ => panic!(),
                            })
                            .collect(),
                    );
                    let resolved = support.resolve(&changed).unwrap();
                    close(resolved[0], 117.);
                    close(resolved[1], 91.);
                }
            }
        }
    }
    for name in ["fill-triangle", "fill-star", "fill-wye"] {
        let g = geometry(name, 1000., 0.);
        assert!(
            (g.bounds.y + g.bounds.height / 2. - g.anchor[1]).abs() > 1.,
            "D3 origin must not be replaced by bbox center"
        );
    }
}
#[test]
fn actual_path_fill_concavity_and_open_stroke_gaps_are_pickable() {
    let mut scene = ChartConfig {
        width: 300.,
        height: 250.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    let star = geometry("fill-star", 3000., 0.);
    scene.marks.push(mark(&star));
    assert!(scene.hit_test(100., 100.).is_some());
    let r = (3000_f64 * 0.890_813_091_529_285_2).sqrt();
    let angle = std::f64::consts::PI / 5.;
    assert!(
        scene
            .hit_test(100. + r * 0.8 * angle.sin(), 100. - r * 0.8 * angle.cos())
            .is_none()
    );
    let plus = geometry("stroke-plus", 64., 0.);
    scene.marks = vec![mark(&plus)];
    assert!(scene.hit_test(100., 104.).is_some());
    assert!(
        scene.hit_test(104., 104.).is_none(),
        "Move must not join separate stroke arms or create filled ink"
    );
}
#[test]
fn checked_paint_metadata_bad_inputs_and_zero_size_contract() {
    for name in names() {
        let g = geometry(name, 0., 0.);
        assert_eq!(g.bounds.width, 0.);
        assert_eq!(g.bounds.height, 0.);
        g.validate().unwrap();
        let m = mark(&g);
        assert_eq!(m.geometry, g.geometry);
        if g.paint_mode == GlyphPaintMode::Fill {
            let mut scene = ChartConfig::default().scene().unwrap();
            scene.marks.push(m);
            assert!(
                scene.hit_test(100., 100.).is_none(),
                "zero filled glyph must not fabricate an Exact hit: {name}"
            );
        }
    }
    for size in [-1., f64::NAN, f64::INFINITY] {
        assert!(
            glyph_geometry(
                [0., 0.],
                &GlyphOptions {
                    size,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
    assert!(
        glyph_geometry(
            [1e6, 1e6],
            &GlyphOptions {
                size: 1e-50,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(glyph_geometry([f64::NAN, 0.], &Default::default()).is_err());
    let g = geometry("fill-star", 64., 0.);
    assert!(glyph_mark("x", &g, None, Some(stroke())).is_err());
    let s = geometry("stroke-plus", 64., 0.);
    assert!(glyph_mark("x", &s, Some(Color(0, 0, 0, 255)), Some(stroke())).is_err());
    assert!(glyph_mark("x", &s, None, None).is_err());
    let mut forged = g.clone();
    forged.anchor[1] += 1.;
    assert!(glyph_mark("x", &forged, Some(Color(0, 0, 0, 255)), None).is_err());
    let mut forged = g;
    forged.anchor_indices.pop();
    assert!(glyph_mark("x", &forged, Some(Color(0, 0, 0, 255)), None).is_err());
}
#[cfg(feature = "raster")]
#[test]
fn cpu_actual_ink_filled_area_and_stroke_nominal_size_are_different() {
    use kartis::render::raster::*;
    for name in names() {
        let g = geometry(name, 1400., 0.23);
        let mut scene = ChartConfig {
            width: 200.,
            height: 200.,
            ..Default::default()
        }
        .scene()
        .unwrap();
        scene.marks.push(mark(&g));
        let raster = render_raster(&scene, &FontRegistry::new(), &Default::default()).unwrap();
        let area = raster
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| (255. - p[0] as f64) / 225.)
            .sum::<f64>();
        if g.paint_mode == GlyphPaintMode::Fill {
            assert!((area - 1400.).abs() < 45., "{name} area {area}");
        } else {
            assert!(
                (100. ..500.).contains(&area),
                "{name} stroke area {area} must not be called nominal filled area"
            );
        }
        let svg = render::to_svg(&scene).unwrap();
        let canvas = render::to_canvas_script(&scene, "test").unwrap();
        assert!(
            svg.contains("data-id=\"glyph\"")
                && canvas.contains("ctx.stroke") == (g.paint_mode == GlyphPaintMode::Stroke)
        );
    }
}

#[cfg(feature = "raster")]
#[test]
fn zero_open_stroke_cap_footprints_match_the_declared_cap() {
    use kartis::render::raster::*;
    for cap in [StrokeCap::Butt, StrokeCap::Round, StrokeCap::Square] {
        let g = geometry("stroke-plus", 0., 0.);
        let mut m = glyph_mark(
            "zero",
            &g,
            None,
            Some(GraphicStroke {
                cap,
                width: 10.,
                ..stroke()
            }),
        )
        .unwrap();
        m.label.clear();
        let mut s = ChartConfig {
            width: 200.,
            height: 200.,
            ..Default::default()
        }
        .scene()
        .unwrap();
        s.marks.push(m);
        let image = render_raster(&s, &FontRegistry::new(), &Default::default()).unwrap();
        let area = image
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| (255. - p[0] as f64) / 225.)
            .sum::<f64>();
        let expected = match cap {
            StrokeCap::Butt => 0.,
            StrokeCap::Round => 25. * std::f64::consts::PI,
            StrokeCap::Square => 100.,
        };
        assert!(
            (area - expected).abs() < 3.,
            "{cap:?}: actual{area}, expected{expected}"
        );
    }
}
