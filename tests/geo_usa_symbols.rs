use glory::render::{GraphicStroke, StrokeCap};
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 960.,
        height: 560.,
        margin: Margin {
            left: 0.,
            right: 0.,
            top: 0.,
            bottom: 0.,
        },
        ..Default::default()
    }
}
fn map() -> GeoMapLayout {
    geographic_usa_map_layout(
        &config(),
        &[],
        &GeoUsaMapOptions {
            scale: Some(1070.),
            translate: Some([480., 250.]),
            ..Default::default()
        },
    )
    .unwrap()
}
fn source() -> Vec<GeoSymbol> {
    vec![
        GeoSymbol::new("continental", [-122.4194, 37.7749], Some(7.)),
        GeoSymbol::new("alaska", [-149.9, 61.2], Some(7.)),
        GeoSymbol::new("hawaii", [-157.8583, 21.3069], Some(7.)),
    ]
}
fn rectangle(rect: ClipRect) -> ClipPath {
    ClipPath {
        geometry: Geometry::Rect {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
        },
        rule: ClipRule::NonZero,
    }
}
fn intersection(a: ClipRect, b: ClipRect) -> ClipRect {
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    ClipRect {
        x,
        y,
        width: ((a.x + a.width).min(b.x + b.width) - x).max(0.),
        height: ((a.y + a.height).min(b.y + b.height) - y).max(0.),
    }
}
fn shapes() -> Vec<GlyphShape> {
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

#[test]
fn every_real_glyph_retains_geometry_and_gets_its_selected_region_mask() {
    let base = map();
    let core = base.projection.usa_projection().unwrap();
    for shape in shapes() {
        for clip in [false, true] {
            let options = GeoGlyphOptions {
                shape,
                rotation: 0.37,
                symbols: GeoSymbolOptions {
                    clip,
                    labels: true,
                    radius_range: [70., 70.],
                    stroke: Some(GraphicStroke {
                        color: Color(20, 30, 40, 255),
                        width: 4.,
                        cap: StrokeCap::Round,
                    }),
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut original = base.clone();
            let unmasked = original.add_glyphs(&source(), &options).unwrap();
            let mut masked = base.clone();
            let typed = masked
                .add_glyphs(
                    &source(),
                    &GeoGlyphOptions {
                        symbols: GeoSymbolOptions {
                            clip_to_projection: true,
                            clip_labels_to_projection: true,
                            ..options.symbols.clone()
                        },
                        ..options
                    },
                )
                .unwrap();
            assert_eq!(typed, unmasked);
            assert_eq!(typed.sites.len(), 3);
            assert_eq!(masked.projection, base.projection);
            for site in &typed.sites {
                let symbol = &source()[site.source_index];
                let p = core.project(symbol.position).unwrap().unwrap();
                assert_eq!(site.glyph.anchor, p.position);
                let expected = rectangle(core.region_clip(p.region));
                let mark = masked
                    .scene
                    .marks
                    .iter()
                    .find(|m| m.id == site.mark_id)
                    .unwrap();
                let old = original
                    .scene
                    .marks
                    .iter()
                    .find(|m| m.id == site.mark_id)
                    .unwrap();
                assert_eq!(mark.geometry, old.geometry);
                assert_eq!(mark.stroke, old.stroke);
                assert!(mark.clip_path.is_none());
                let region = core.region_clip(p.region);
                assert_eq!(
                    mark.clip,
                    Some(if clip {
                        intersection(region, base.projection.plot())
                    } else {
                        region
                    })
                );
                let label_index = masked
                    .scene
                    .labels
                    .iter()
                    .position(|l| l.text == symbol.label)
                    .unwrap();
                assert_eq!(masked.scene.label_clips.get(&label_index), Some(&expected));
            }
            typed
                .interaction_binding(&masked.scene)
                .unwrap()
                .validate(&masked.scene)
                .unwrap();
            masked.scene.validate().unwrap();
        }
    }
}

#[test]
fn inset_full_ink_masks_remove_spill_that_a_union_footprint_would_keep() {
    let base = map();
    let core = base.projection.usa_projection().unwrap();
    for (point, region) in [
        ([-149.9, 61.2], GeoUsaRegion::Alaska),
        ([-157.8583, 21.3069], GeoUsaRegion::Hawaii),
    ] {
        let symbol = [GeoSymbol::new("large", point, Some(1.))];
        for stroke in [
            None,
            Some(GraphicStroke {
                color: Color(20, 30, 40, 255),
                width: 12.,
                cap: StrokeCap::Round,
            }),
        ] {
            let options = GeoSymbolOptions {
                clip: false,
                radius_range: [100., 100.],
                stroke,
                ..Default::default()
            };
            let mut original = base.clone();
            original.add_symbols(&symbol, &options).unwrap();
            let mut masked = base.clone();
            masked
                .add_symbols(
                    &symbol,
                    &GeoSymbolOptions {
                        clip_to_projection: true,
                        ..options
                    },
                )
                .unwrap();
            assert_eq!(
                original.scene.marks[0].geometry,
                masked.scene.marks[0].geometry
            );
            let rect = core.region_clip(region);
            let lower = core.region_clip(GeoUsaRegion::Lower48);
            let center = core.project(point).unwrap().unwrap().position;
            let mut spill = 0;
            for dx in (-100..=100).step_by(5) {
                for dy in (-100..=100).step_by(5) {
                    let p = [center[0] + f64::from(dx), center[1] + f64::from(dy)];
                    if lower.contains(p[0], p[1])
                        && !rect.contains(p[0], p[1])
                        && original.scene.hit_test(p[0], p[1]).is_some()
                    {
                        assert!(masked.scene.hit_test(p[0], p[1]).is_none());
                        spill += 1;
                    }
                }
            }
            assert!(
                spill > 20,
                "must test real spill inside the Lower48 footprint"
            );
        }
    }
}

#[test]
fn hidden_missing_zero_and_clipped_rows_keep_palette_and_nullable_metadata() {
    let mut symbols = source();
    symbols[0].value = None;
    symbols[0].series = "A".into();
    symbols[1].series = "B".into();
    symbols[2].series = "C".into();
    symbols.push(GeoSymbol::new("outside", [2.35, 48.86], Some(1.)));
    symbols.push(GeoSymbol::new("zero", [-74., 40.], Some(0.)));
    let options = GeoGlyphOptions {
        shape: GlyphShape::Filled(FilledGlyph::Wye),
        hidden: ["B".into()].into(),
        symbols: GeoSymbolOptions {
            clip_to_projection: true,
            clip_labels_to_projection: true,
            labels: true,
            radius_range: [4., 10.],
            missing: GeoMissingSymbol::Draw {
                radius: 5.,
                color: Color(130, 140, 150, 255),
                label: "missing".into(),
            },
            color: GeoSymbolColor::Series(vec![
                Color(10, 0, 0, 255),
                Color(20, 0, 0, 255),
                Color(30, 0, 0, 255),
                Color(40, 0, 0, 255),
            ]),
            ..Default::default()
        },
        ..Default::default()
    };
    let mut m = map();
    let layer = m.add_glyphs(&symbols, &options).unwrap();
    assert_eq!(layer.sites.len(), 2);
    assert_eq!(
        layer
            .sites
            .iter()
            .find(|s| s.id == "continental")
            .unwrap()
            .value,
        None
    );
    assert_eq!(
        layer.sites.iter().find(|s| s.id == "hawaii").unwrap().color,
        Color(30, 0, 0, 255)
    );
    let binding = layer.interaction_binding(&m.scene).unwrap();
    assert_eq!(binding.sites.len(), 2);
    assert!(binding.sites.iter().any(|s| s.value.is_none()));
    let mut reversed = symbols;
    reversed.reverse();
    let mut second = map();
    let other = second.add_glyphs(&reversed, &options).unwrap();
    assert_eq!(second.scene, m.scene);
    for site in other.sites {
        assert_eq!(reversed[site.source_index].id, site.id);
    }
}

#[test]
fn usa_overlays_reject_atomically_and_single_conic_footprint_is_unavailable() {
    let mut m = map();
    let old = m.clone();
    assert!(m.projection.projection_clip_path(0.25, 4096).is_err());
    assert!(m.projection.projection_disk().is_none());
    let duplicate = [source()[0].clone(), source()[0].clone()];
    assert!(
        m.add_symbols(
            &duplicate,
            &GeoSymbolOptions {
                clip_to_projection: true,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(m, old);
    let wrong = ChartConfig {
        width: 800.,
        ..config()
    }
    .scene()
    .unwrap();
    let mut wrong = wrong;
    let before = wrong.clone();
    assert!(
        m.projection
            .add_symbols(&mut wrong, &source(), &Default::default())
            .is_err()
    );
    assert_eq!(wrong, before);
    m.add_symbols(&source(), &Default::default()).unwrap();
    let before = m.clone();
    assert!(
        m.add_symbols(
            &source(),
            &GeoSymbolOptions {
                clip_to_projection: true,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(m, before);
}

#[test]
fn unimplemented_line_and_graticule_consumers_reject_without_using_a_single_albers_frame() {
    let mut m = map();
    let old = m.clone();
    assert!(
        m.add_lines(
            &[GeoLineFeature::new(
                "route",
                vec![[-149.9, 61.2], [-157.8583, 21.3069]]
            )],
            &Default::default()
        )
        .is_err()
    );
    assert_eq!(m, old);
    assert!(
        m.add_graticule(&Default::default(), &Default::default())
            .is_err()
    );
    assert_eq!(m, old);
}

#[test]
fn shared_choropleth_colors_include_invisible_values_and_preserve_one_feature_identity() {
    let mut features = source()
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let [x, y] = s.position;
            GeoFeature {
                id: s.id.clone(),
                label: s.label.clone(),
                value: if i == 2 { None } else { Some(10. * i as f64) },
                polygons: vec![vec![vec![
                    [x - 1., y - 1.],
                    [x - 1., y + 1.],
                    [x + 1., y + 1.],
                    [x + 1., y - 1.],
                ]]],
            }
        })
        .collect::<Vec<_>>();
    features.push(GeoFeature {
        id: "outside".into(),
        label: "Outside".into(),
        value: Some(100.),
        polygons: vec![vec![vec![[1., 47.], [1., 49.], [3., 49.], [3., 47.]]]],
    });
    let colors = ColorScaleOptions {
        unknown: Color(190, 30, 150, 255),
        ..Default::default()
    };
    let layout =
        choropleth_usa_with_color_scale(&config(), &features, &Default::default(), &colors)
            .unwrap();
    assert_eq!(layout.color_scale.domain(), [0., 100.]);
    assert_eq!(layout.scene.marks.len(), 3);
    for mark in &layout.scene.marks {
        let feature = features.iter().find(|f| f.id == mark.series).unwrap();
        assert_eq!(mark.id, format!("map:{}", feature.id));
        assert_eq!(mark.fill, Some(layout.color_scale.map(feature.value)));
        assert_eq!(mark.label, feature.label);
    }
    assert_eq!(
        layout
            .scene
            .marks
            .iter()
            .find(|m| m.series == "hawaii")
            .unwrap()
            .fill,
        Some(colors.unknown)
    );
    let profiled = choropleth_usa_with_color_profile(
        &config(),
        &features,
        &Default::default(),
        &colors,
        ColorInterpolation::Lab,
    )
    .unwrap();
    assert_eq!(profiled.projection, layout.projection);
    assert_ne!(
        profiled
            .scene
            .marks
            .iter()
            .find(|m| m.series == "alaska")
            .unwrap()
            .fill,
        layout
            .scene
            .marks
            .iter()
            .find(|m| m.series == "alaska")
            .unwrap()
            .fill
    );
}

#[test]
fn default_region_masks_mount_real_glory_glyph_controls_and_editable_labels() {
    for shape in shapes() {
        let app = glory::App::memory();
        let mut m = map();
        let layer = m
            .add_glyphs(
                &source(),
                &GeoGlyphOptions {
                    shape,
                    symbols: GeoSymbolOptions {
                        clip_to_projection: true,
                        clip_labels_to_projection: true,
                        labels: true,
                        radius_range: [18., 18.],
                        stroke: Some(GraphicStroke {
                            color: Color(20, 30, 40, 255),
                            width: 2.,
                            cap: StrokeCap::Round,
                        }),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .unwrap();
        let mut binding = layer.interaction_binding(&m.scene).unwrap();
        binding.picking_profile = PickingProfile::PaintedGeometry;
        let frame = SceneInteraction {
            scene: m.scene,
            interaction: Some(binding),
            plot: Some(m.projection.plot()),
        };
        assert!(
            frame
                .scene
                .marks
                .iter()
                .all(|mark| mark.clip_path.is_none() && mark.clip.is_some())
        );
        app.mount(chart_layout(frame).legend(false)).unwrap();
        let tree = app.tree();
        let mut pending = tree.roots().to_vec();
        let mut controls = 0;
        let mut labels = 0;
        while let Some(id) = pending.pop() {
            let node = tree.node(id).unwrap();
            let classes = &node.descriptor.view().classes;
            controls += usize::from(classes.iter().any(|c| c == "kartis-datum"));
            labels += usize::from(classes.iter().any(|c| c == "kartis-label"));
            pending.extend(node.children.iter().copied());
        }
        assert_eq!(controls, 3);
        assert_eq!(labels, 3);
    }
}
