use glory::render::{GraphicStroke, PathCommand as C, StrokeCap};
use kartis::*;
fn config() -> ChartConfig {
    ChartConfig {
        width: 640.,
        height: 360.,
        margin: Margin {
            left: 40.,
            right: 40.,
            top: 40.,
            bottom: 40.,
        },
        ..Default::default()
    }
}
fn land() -> Vec<GeoFeature> {
    vec![GeoFeature {
        id: "land".into(),
        label: "Land".into(),
        value: Some(1.),
        polygons: vec![vec![vec![
            [-30., -15.],
            [30., -15.],
            [30., 15.],
            [-30., 15.],
            [-30., -15.],
        ]]],
    }]
}
fn map(options: GeoMapOptions) -> GeoMapLayout {
    choropleth_layout(&config(), &land(), &options).unwrap()
}
fn options(shape: GlyphShape) -> GeoGlyphOptions {
    let mut o = GeoGlyphOptions {
        shape,
        ..Default::default()
    };
    o.symbols.domain = Some([0., 100.]);
    o.symbols.radius_range = [0., 12.];
    o.symbols.stroke = Some(GraphicStroke {
        color: Color(255, 255, 255, 255),
        width: 1.5,
        cap: StrokeCap::Round,
    });
    o.rotation = 0.37;
    o
}
fn all() -> Vec<GlyphShape> {
    vec![
        GlyphShape::Filled(FilledGlyph::Circle),
        GlyphShape::Filled(FilledGlyph::Cross),
        GlyphShape::Filled(FilledGlyph::Diamond),
        GlyphShape::Filled(FilledGlyph::Square),
        GlyphShape::Filled(FilledGlyph::Star),
        GlyphShape::Filled(FilledGlyph::Triangle),
        GlyphShape::Filled(FilledGlyph::Wye),
        GlyphShape::Stroked(StrokedGlyph::Circle),
        GlyphShape::Stroked(StrokedGlyph::Plus),
        GlyphShape::Stroked(StrokedGlyph::Times),
        GlyphShape::Stroked(StrokedGlyph::Triangle),
        GlyphShape::Stroked(StrokedGlyph::Asterisk),
        GlyphShape::Stroked(StrokedGlyph::Square),
        GlyphShape::Stroked(StrokedGlyph::Diamond),
    ]
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}
#[test]
fn every_glyph_uses_the_exact_map_context_and_nullable_typed_anchor() {
    for projection in [
        GeoProjection::Equirectangular,
        GeoProjection::Mercator,
        GeoProjection::TransverseMercator,
        GeoProjection::EqualEarth,
        GeoProjection::NaturalEarth,
        GeoProjection::Orthographic,
        GeoProjection::AzimuthalEqualArea,
        GeoProjection::AzimuthalEquidistant,
        GeoProjection::Stereographic,
        GeoProjection::Gnomonic,
    ] {
        for shape in all() {
            let mut m = map(GeoMapOptions {
                projection,
                center: [5., 3.],
                rotation: 15.,
                scale: Some(100.),
                translate: Some([310., 170.]),
                ..Default::default()
            });
            let original = m.projection.clone();
            let expected = m.projection.project([8., 2.]).unwrap().unwrap();
            let o = options(shape);
            let layer = m
                .add_glyphs(&[GeoSymbol::new("place", [8., 2.], Some(25.))], &o)
                .unwrap();
            assert_eq!(m.projection, original);
            assert_eq!(layer.sites.len(), 1);
            let site = &layer.sites[0];
            assert_eq!(site.glyph.anchor, expected);
            assert_eq!(site.mark_id, "geo-symbol:5:place");
            assert_eq!(site.source_index, 0);
            assert_eq!(site.value, Some(25.));
            close(site.glyph.size, std::f64::consts::PI * 36.);
            let mark = m
                .scene
                .marks
                .iter()
                .find(|mark| mark.id == site.mark_id)
                .unwrap();
            assert_eq!(mark.geometry, site.glyph.geometry);
            assert_eq!(mark.fill.is_none(), matches!(shape, GlyphShape::Stroked(_)));
            let binding = layer.interaction_binding(&m.scene).unwrap();
            assert_eq!(binding.sites[0].position, expected);
            assert_eq!(
                binding.anchors.contains_key(&site.mark_id),
                matches!(mark.geometry, Geometry::Path(_))
            );
            m.scene.validate().unwrap();
        }
    }
}
fn differences(geometry: &Geometry) -> Vec<[f64; 2]> {
    match geometry {
        Geometry::Circle { radius, .. } => vec![[*radius, 0.]],
        Geometry::Path(p) => {
            let v = p
                .iter()
                .filter_map(|c| match c {
                    C::Move(p) | C::Line(p) => Some([p.x, p.y]),
                    _ => None,
                })
                .collect::<Vec<_>>();
            v.iter().map(|p| [p[0] - v[0][0], p[1] - v[0][1]]).collect()
        }
        _ => panic!(),
    }
}
#[test]
fn size_and_color_legends_are_the_same_real_glyphs_not_circle_or_rect_surrogates() {
    for shape in all() {
        let mut m = map(Default::default());
        let mut o = options(shape);
        o.symbols.size_legend = Some(GeoSizeLegend {
            layout: GeoLegendOptions {
                position: [20., 20.],
                ..Default::default()
            },
            values: vec![25.],
        });
        o.symbols.color_legend = Some(GeoLegendOptions {
            position: [500., 20.],
            ..Default::default()
        });
        let layer = m
            .add_glyphs(&[GeoSymbol::new("place", [0., 0.], Some(25.))], &o)
            .unwrap();
        let site = &layer.sites[0];
        for id in ["geo-size-legend:0", "geo-color-legend:0"] {
            let mark = m.scene.marks.iter().find(|m| m.id == id).unwrap();
            assert!(mark.label.is_empty());
            assert_eq!(mark.fill.is_none(), matches!(shape, GlyphShape::Stroked(_)));
            let a = differences(&site.glyph.geometry);
            let b = differences(&mark.geometry);
            assert_eq!(a.len(), b.len());
            for (a, b) in a.iter().zip(b) {
                close(a[0], b[0]);
                close(a[1], b[1]);
            }
        }
    }
}
#[test]
fn hidden_zero_missing_palette_domain_and_existing_map_are_preserved() {
    let mut m = map(Default::default());
    m.scene.marks[0].series = "B".into();
    let base = m.scene.marks[0].clone();
    let mut symbols = vec![
        GeoSymbol::new("small", [0., 0.], Some(1.)),
        GeoSymbol::new("large", [10., 0.], Some(4.)),
        GeoSymbol::new("missing", [-10., 0.], None),
        GeoSymbol::new("zero", [0., 10.], Some(0.)),
    ];
    for s in &mut symbols {
        s.series = if s.id == "large" { "B" } else { "A" }.into();
    }
    let mut o = options(GlyphShape::Filled(FilledGlyph::Wye));
    o.symbols.domain = None;
    o.symbols.radius_range = [0., 16.];
    o.symbols.labels = true;
    o.symbols.missing = GeoMissingSymbol::Draw {
        radius: 5.,
        color: Color(230, 150, 20, 255),
        label: "missing".into(),
    };
    o.symbols.color_legend = Some(GeoLegendOptions::default());
    o.symbols.size_legend = Some(GeoSizeLegend {
        layout: GeoLegendOptions {
            position: [20., 120.],
            ..Default::default()
        },
        values: vec![0., 1., 4.],
    });
    o.hidden.insert("B".into());
    let layer = m.add_glyphs(&symbols, &o).unwrap();
    assert_eq!(layer.domain, [0., 4.]);
    assert_eq!(layer.sites.len(), 2);
    assert_eq!(m.scene.marks[0], base);
    assert!(layer.sites.iter().all(|s| s.series == "A"));
    assert!(m.scene.legend.iter().any(|l| l.id == "B"));
    assert!(
        !m.scene
            .marks
            .iter()
            .any(|m| m.id == "geo-symbol:4:zero" || m.id == "geo-symbol:5:large")
    );
    assert_eq!(
        m.scene
            .marks
            .iter()
            .filter(|m| m.id.starts_with("geo-size-legend:"))
            .count(),
        2
    );
    let binding = layer.interaction_binding(&m.scene).unwrap();
    assert_eq!(
        binding
            .sites
            .iter()
            .find(|s| s.mark_id.ends_with(":missing"))
            .unwrap()
            .value,
        None
    );
    assert_eq!(
        binding
            .sites
            .iter()
            .find(|s| s.mark_id.ends_with(":small"))
            .unwrap()
            .value,
        Some(1.)
    );
    assert!(m.scene.label_series.values().all(|s| s != "B"));
    let mut reverse = map(Default::default());
    symbols.reverse();
    let second = reverse.add_glyphs(&symbols, &o).unwrap();
    let first = layer.sites.iter().find(|s| s.id == "small").unwrap();
    let next = second.sites.iter().find(|s| s.id == "small").unwrap();
    assert_eq!(first.color, next.color);
    assert_eq!(first.glyph, next.glyph);
    assert_ne!(first.source_index, next.source_index);
}
#[test]
fn concave_and_stroked_symbols_keep_actual_geometry_and_full_projection_mask() {
    for shape in [
        GlyphShape::Filled(FilledGlyph::Star),
        GlyphShape::Filled(FilledGlyph::Wye),
        GlyphShape::Stroked(StrokedGlyph::Plus),
    ] {
        let mut m = map(GeoMapOptions {
            projection: GeoProjection::Orthographic,
            ..Default::default()
        });
        let disk = m.projection.projection_disk().unwrap();
        let mut o = options(shape);
        o.rotation = 0.;
        o.symbols.radius_range = [0., 32.];
        o.symbols.clip = false;
        o.symbols.clip_to_projection = true;
        o.symbols.clip_labels_to_projection = true;
        o.symbols.labels = true;
        let layer = m
            .add_glyphs(
                &[
                    GeoSymbol::new("edge", [80., 0.], Some(100.)),
                    GeoSymbol::new("back", [170., 0.], Some(100.)),
                ],
                &o,
            )
            .unwrap();
        assert_eq!(layer.sites.len(), 1);
        let site = &layer.sites[0];
        let mark = m.scene.marks.iter().find(|m| m.id == site.mark_id).unwrap();
        assert_eq!(mark.geometry, site.glyph.geometry);
        assert!(mark.clip_path.is_some());
        assert!(!m.scene.label_clips.is_empty());
        let outside = [disk.center[0] + disk.radius + 6., disk.center[1]];
        assert!(
            !mark
                .clip_path
                .as_ref()
                .unwrap()
                .contains(outside[0], outside[1])
        );
        assert!(m.scene.hit_test(outside[0], outside[1]).is_none());
        layer.interaction_binding(&m.scene).unwrap();
    }
}
#[test]
fn bad_glyph_source_style_collision_and_metadata_are_atomic() {
    let mut m = map(Default::default());
    let before = m.scene.clone();
    let symbol = GeoSymbol::new("x", [0., 0.], Some(1.));
    let mut o = options(GlyphShape::Stroked(StrokedGlyph::Asterisk));
    o.symbols.stroke = None;
    assert!(m.add_glyphs(std::slice::from_ref(&symbol), &o).is_err());
    assert_eq!(m.scene, before);
    o = options(GlyphShape::Filled(FilledGlyph::Star));
    o.rotation = f64::NAN;
    assert!(m.add_glyphs(std::slice::from_ref(&symbol), &o).is_err());
    assert_eq!(m.scene, before);
    o.rotation = 0.;
    let mut tiny = o.clone();
    tiny.symbols.radius_range = [0., 1e-200];
    assert!(m.add_glyphs(std::slice::from_ref(&symbol), &tiny).is_err());
    assert_eq!(m.scene, before);
    tiny.symbols.radius_range = [0., 12.];
    tiny.symbols.domain = Some([0., f64::MAX]);
    assert!(
        m.add_glyphs(
            &[GeoSymbol::new("tiny", [0., 0.], Some(f64::from_bits(1)))],
            &tiny
        )
        .is_err()
    );
    assert_eq!(m.scene, before);
    let layer = m.add_glyphs(std::slice::from_ref(&symbol), &o).unwrap();
    let accepted = m.scene.clone();
    assert!(m.add_glyphs(&[symbol], &o).is_err());
    assert_eq!(m.scene, accepted);
    let mut stale = layer;
    stale.sites[0].glyph.anchor[0] += 1.;
    assert!(stale.interaction_binding(&m.scene).is_err());
}
#[cfg(feature = "raster")]
#[test]
fn cpu_projection_mask_clips_real_filled_and_stroked_glyph_ink() {
    use kartis::render::raster::*;
    for shape in [
        GlyphShape::Filled(FilledGlyph::Star),
        GlyphShape::Stroked(StrokedGlyph::Plus),
    ] {
        let mut m = map(GeoMapOptions {
            projection: GeoProjection::Orthographic,
            ..Default::default()
        });
        let disk = m.projection.projection_disk().unwrap();
        let mut o = options(shape);
        o.rotation = 0.;
        o.symbols.radius_range = [0., 32.];
        o.symbols.clip = false;
        o.symbols.clip_to_projection = true;
        let layer = m
            .add_glyphs(&[GeoSymbol::new("edge", [80., 0.], Some(100.))], &o)
            .unwrap();
        let raster = render_raster(&m.scene, &FontRegistry::new(), &Default::default()).unwrap();
        let x = (disk.center[0] + disk.radius + 6.) as usize;
        let y = disk.center[1] as usize;
        let index = (y * 640 + x) * 4;
        assert_eq!(&raster.rgba[index..index + 4], &[255, 255, 255, 255]);
        let x = (layer.sites[0].glyph.anchor[0] - 10.) as usize;
        let index = (y * 640 + x) * 4;
        assert!(raster.rgba[index..index + 3].iter().any(|v| *v < 220));
    }
}
#[test]
fn new_glyph_layers_preserve_custom_order_and_reject_cumulative_command_budget() {
    let mut m = map(Default::default());
    m.scene.draw_order = m.scene.draw_items();
    let original = m.scene.draw_order.clone();
    let mut o = options(GlyphShape::Filled(FilledGlyph::Star));
    o.symbols.labels = true;
    o.symbols.size_legend = Some(GeoSizeLegend {
        layout: GeoLegendOptions {
            position: [20., 20.],
            ..Default::default()
        },
        values: vec![0., 25.],
    });
    let layer = m
        .add_glyphs(&[GeoSymbol::new("x", [0., 0.], Some(25.))], &o)
        .unwrap();
    assert_eq!(&m.scene.draw_order[..original.len()], &original);
    assert_eq!(
        m.scene.draw_order.len(),
        m.scene.marks.len() + m.scene.labels.len()
    );
    layer.interaction_binding(&m.scene).unwrap();
    m.scene.validate().unwrap();
    let mut limited = map(Default::default());
    let count = 199_990
        - limited
            .scene
            .marks
            .iter()
            .map(|m| match &m.geometry {
                Geometry::Path(p) => p.len(),
                _ => 0,
            })
            .sum::<usize>();
    let mut path = Vec::with_capacity(count);
    path.push(C::Move(glory::render::GraphicPoint::new(0., 0.)));
    for i in 1..count {
        path.push(C::Line(glory::render::GraphicPoint::new(
            (i % 600) as f64,
            1.,
        )));
    }
    let mut decor = Mark::new("long-decor", Geometry::Path(path), Color(30, 30, 30, 255));
    decor.label.clear();
    limited.scene.marks.push(decor);
    limited.scene.validate().unwrap();
    let before = limited.scene.clone();
    assert!(
        limited
            .add_glyphs(&[GeoSymbol::new("x", [0., 0.], Some(25.))], &o)
            .is_err()
    );
    assert_eq!(limited.scene, before);
}
