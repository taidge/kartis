use glory::render::{GraphicStroke, StrokeCap};
use kartis::*;
use std::collections::BTreeSet;

fn config() -> ChartConfig {
    ChartConfig {
        width: 420.,
        height: 420.,
        margin: Margin {
            left: 10.,
            right: 10.,
            top: 10.,
            bottom: 10.,
        },
        ..Default::default()
    }
}
fn symbol_scene(scene: &Scene) -> Scene {
    let mut scene = scene.clone();
    scene.marks.retain(|m| m.id.starts_with("geo-symbol:"));
    scene.draw_order.clear();
    scene
}
fn options(shape: GeoSymbolShape) -> GeoSymbolOptions {
    GeoSymbolOptions {
        shape,
        clip: false,
        clip_to_projection: true,
        radius_range: [55., 55.],
        color: GeoSymbolColor::Fixed(Color(230, 30, 40, 255)),
        stroke: Some(GraphicStroke {
            color: Color(20, 30, 40, 255),
            width: 12.,
            cap: StrokeCap::Round,
        }),
        ..Default::default()
    }
}

#[test]
fn stroke_masks_preserve_original_geometry_and_clip_hits_for_all_shapes_and_disks() {
    for projection in [
        GeoProjection::Orthographic,
        GeoProjection::AzimuthalEqualArea,
        GeoProjection::AzimuthalEquidistant,
        GeoProjection::Stereographic,
        GeoProjection::Gnomonic,
    ] {
        let base = geographic_map_layout(
            &config(),
            &[],
            &GeoMapOptions {
                projection,
                clip_angle: Some(30.),
                ..Default::default()
            },
        )
        .unwrap();
        let disk = base.projection.projection_disk().unwrap();
        let p = base.projection.project([29., 0.]).unwrap().unwrap();
        for shape in [
            GeoSymbolShape::Circle,
            GeoSymbolShape::Square,
            GeoSymbolShape::Diamond,
            GeoSymbolShape::Triangle,
            GeoSymbolShape::Cross,
        ] {
            let mut original = base.clone();
            let mut clipped = base.clone();
            let source = [GeoSymbol::new("edge", [29., 0.], Some(1.))];
            original
                .add_symbols(
                    &source,
                    &GeoSymbolOptions {
                        clip_to_projection: false,
                        ..options(shape)
                    },
                )
                .unwrap();
            clipped.add_symbols(&source, &options(shape)).unwrap();
            let original = symbol_scene(&original.scene);
            let clipped = symbol_scene(&clipped.scene);
            clipped.validate().unwrap();
            let a = &original.marks[0];
            let b = &clipped.marks[0];
            assert_eq!(a.geometry, b.geometry);
            assert_eq!(a.stroke, b.stroke);
            assert_eq!(a.label, b.label);
            assert_eq!(
                b.clip_path.as_deref(),
                Some(&ClipPath::circle(
                    disk.center[0],
                    disk.center[1],
                    disk.radius
                ))
            );
            let mut kept = 0;
            let mut removed = 0;
            for dx in (-75..=75).step_by(3) {
                for dy in (-75..=75).step_by(3) {
                    let (x, y) = (p[0] + f64::from(dx) + 0.173, p[1] + f64::from(dy) + 0.371);
                    let inside = (x - disk.center[0]).hypot(y - disk.center[1]) < disk.radius;
                    let hit = original.hit_test(x, y).is_some();
                    assert_eq!(
                        clipped.hit_test(x, y).is_some(),
                        hit && inside,
                        "{projection:?}/{shape:?} {dx},{dy}"
                    );
                    kept += usize::from(hit && inside);
                    removed += usize::from(hit && !inside);
                }
            }
            assert!(
                kept > 0 && removed > 0,
                "{projection:?}/{shape:?} did not cross the horizon"
            );
        }
    }
}

#[test]
fn only_new_location_labels_are_masked_and_owned_while_legends_remain_complete() {
    let mut layout = geographic_map_layout(
        &config(),
        &[],
        &GeoMapOptions {
            projection: GeoProjection::Orthographic,
            ..Default::default()
        },
    )
    .unwrap();
    layout.scene.labels.push(Label {
        x: 20.,
        y: 20.,
        text: "prior".into(),
        color: Color(0, 0, 0, 255),
        size: 12.,
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
    layout
        .scene
        .label_clips
        .insert(0, ClipPath::circle(20., 20., 10.));
    layout.scene.draw_order = layout.scene.draw_items();
    let previous = layout.scene.label_clips.clone();
    let mut edge = GeoSymbol::new("edge", [89., 0.], Some(1.));
    edge.label = "location label".into();
    let source = [edge, GeoSymbol::new("back", [180., 0.], Some(1.))];
    layout
        .add_symbols(
            &source,
            &GeoSymbolOptions {
                labels: true,
                clip_labels_to_projection: true,
                size_legend: Some(GeoSizeLegend {
                    layout: GeoLegendOptions {
                        title: "size legend".into(),
                        ..Default::default()
                    },
                    values: vec![1.],
                }),
                color_legend: Some(GeoLegendOptions {
                    title: "color legend".into(),
                    ..Default::default()
                }),
                ..options(GeoSymbolShape::Circle)
            },
        )
        .unwrap();
    layout.scene.validate().unwrap();
    let disk = layout.projection.projection_disk().unwrap();
    assert_eq!(layout.scene.label_clips.get(&0), previous.get(&0));
    let location = layout
        .scene
        .labels
        .iter()
        .position(|l| l.text == "location label")
        .unwrap();
    assert_eq!(
        layout.scene.label_clips.get(&location),
        Some(&ClipPath::circle(
            disk.center[0],
            disk.center[1],
            disk.radius
        ))
    );
    assert_eq!(
        layout.scene.label_series.get(&location).map(String::as_str),
        Some("symbols")
    );
    assert_eq!(layout.scene.label_clips.len(), 2);
    assert!(!layout.scene.labels.iter().any(|l| l.text == "back"));
    for (index, label) in layout.scene.labels.iter().enumerate() {
        if label.text.contains("legend") {
            assert!(!layout.scene.label_clips.contains_key(&index));
        }
    }
    for mark in layout
        .scene
        .marks
        .iter()
        .filter(|m| m.id.contains("-legend:"))
    {
        assert!(mark.clip_path.is_none());
    }
    assert!(layout.scene.draw_order.contains(&DrawItem::Label(location)));
    let hidden = layout
        .scene
        .without_series(&BTreeSet::from(["symbols".into()]));
    hidden.validate().unwrap();
    assert_eq!(hidden.label_clips.len(), 1);
    assert_eq!(hidden.labels[0].text, "prior");
    assert!(!hidden.labels.iter().any(|l| l.text == "location label"));
}

#[test]
fn label_masks_are_independent_of_fill_clipping_and_bad_profiles_are_atomic() {
    let c = config();
    let source = [GeoSymbol::new("edge", [89., 0.], Some(1.))];
    let mut map = geographic_map_layout(
        &c,
        &[],
        &GeoMapOptions {
            projection: GeoProjection::Orthographic,
            ..Default::default()
        },
    )
    .unwrap();
    map.add_symbols(
        &source,
        &GeoSymbolOptions {
            clip_to_projection: false,
            clip_labels_to_projection: true,
            labels: true,
            ..options(GeoSymbolShape::Circle)
        },
    )
    .unwrap();
    assert!(
        map.scene
            .marks
            .iter()
            .filter(|m| m.id.starts_with("geo-symbol:"))
            .all(|m| m.clip_path.is_none())
    );
    assert_eq!(map.scene.label_clips.len(), 1);
    let before = map.scene.clone();
    let mut invalid = options(GeoSymbolShape::Circle);
    invalid.clip_labels_to_projection = true;
    invalid.stroke.as_mut().unwrap().width = f64::NAN;
    assert!(
        map.add_symbols(&[GeoSymbol::new("second", [0., 0.], Some(1.))], &invalid)
            .is_err()
    );
    assert_eq!(map.scene, before);
    for projection in [
        GeoProjection::Mercator,
        GeoProjection::Equirectangular,
        GeoProjection::EqualEarth,
        GeoProjection::NaturalEarth,
    ] {
        let mut map = geographic_map_layout(
            &c,
            &[],
            &GeoMapOptions {
                projection,
                ..Default::default()
            },
        )
        .unwrap();
        let before = map.scene.clone();
        assert!(
            map.add_symbols(
                &[],
                &GeoSymbolOptions {
                    clip_labels_to_projection: true,
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert_eq!(map.scene, before);
    }
}

#[test]
fn fill_only_geometry_profile_is_retained_and_new_masks_obey_scene_bounds() {
    let mut map = geographic_map_layout(
        &config(),
        &[],
        &GeoMapOptions {
            projection: GeoProjection::Orthographic,
            ..Default::default()
        },
    )
    .unwrap();
    map.add_symbols(
        &[GeoSymbol::new("edge", [89., 0.], Some(1.))],
        &GeoSymbolOptions {
            stroke: None,
            ..options(GeoSymbolShape::Circle)
        },
    )
    .unwrap();
    let mark = map
        .scene
        .marks
        .iter()
        .find(|m| m.id.starts_with("geo-symbol:"))
        .unwrap();
    assert!(matches!(mark.geometry, Geometry::Path(_)));
    assert!(mark.clip_path.is_none());
    let mut far = geographic_map_layout(
        &config(),
        &[],
        &GeoMapOptions {
            projection: GeoProjection::Orthographic,
            scale: Some(100.),
            translate: Some([999999., 200.]),
            ..Default::default()
        },
    )
    .unwrap();
    let before = far.scene.clone();
    assert!(
        far.add_symbols(
            &[GeoSymbol::new("center", [0., 0.], Some(1.))],
            &GeoSymbolOptions {
                clip: false,
                clip_labels_to_projection: true,
                labels: true,
                ..options(GeoSymbolShape::Circle)
            }
        )
        .is_err()
    );
    assert_eq!(far.scene, before);
}

#[cfg(feature = "raster")]
#[test]
fn real_cpu_pixels_mask_strokes_and_shaped_location_labels_without_masking_legend() {
    use kartis::render::raster::{FontRegistry, RasterOptions, render_raster};
    let mut fonts = FontRegistry::new();
    fonts
        .register(include_bytes!("fonts/ABeeZee-Regular.ttf").as_slice(), 0)
        .unwrap();
    let mut original = geographic_map_layout(
        &config(),
        &[],
        &GeoMapOptions {
            projection: GeoProjection::Orthographic,
            scale: Some(100.),
            translate: Some([210., 210.]),
            ..Default::default()
        },
    )
    .unwrap();
    let mut masked = original.clone();
    let mut symbol = GeoSymbol::new("edge", [89., 0.], Some(1.));
    symbol.label = "MASK LABEL".into();
    let options = GeoSymbolOptions {
        labels: true,
        label_color: Color(20, 30, 230, 255),
        label_size: 20.,
        stroke: Some(GraphicStroke {
            color: Color(20, 20, 20, 255),
            width: 12.,
            cap: StrokeCap::Round,
        }),
        clip_to_projection: false,
        clip_labels_to_projection: false,
        size_legend: Some(GeoSizeLegend {
            layout: GeoLegendOptions {
                position: [15., 25.],
                title: "Legend".into(),
                ..Default::default()
            },
            values: vec![1.],
        }),
        ..options(GeoSymbolShape::Circle)
    };
    original
        .add_symbols(std::slice::from_ref(&symbol), &options)
        .unwrap();
    masked
        .add_symbols(
            &[symbol],
            &GeoSymbolOptions {
                clip_to_projection: true,
                clip_labels_to_projection: true,
                ..options
            },
        )
        .unwrap();
    let source = render_raster(&original.scene, &fonts, &RasterOptions::default()).unwrap();
    let clipped = render_raster(&masked.scene, &fonts, &RasterOptions::default()).unwrap();
    let mut source_outside = [0usize; 3];
    let mut clipped_outside = [0usize; 3];
    let mut inside = [0usize; 3];
    let mut legend = 0;
    for y in 0..420usize {
        for x in 0..420usize {
            let classify = |frame: &kartis::render::raster::RenderedImage| {
                let i = (y * 420 + x) * 4;
                let p = &frame.rgba[i..i + 4];
                [
                    p[0] > 160 && p[1] < 90 && p[2] < 100,
                    p[2] > 150 && p[0] < 70 && p[1] < 100,
                    p[0] < 60 && p[1] < 60 && p[2] < 60,
                ]
            };
            let a = classify(&source);
            let b = classify(&clipped);
            let distance = (x as f64 + 0.5 - 210.).hypot(y as f64 + 0.5 - 210.);
            if x > 200 && distance > 102. {
                for i in 0..3 {
                    source_outside[i] += usize::from(a[i]);
                    clipped_outside[i] += usize::from(b[i]);
                }
            }
            if distance < 98. {
                for i in 0..3 {
                    inside[i] += usize::from(b[i]);
                }
            }
            if x < 140 && y < 180 {
                let offset = (y * 420 + x) * 4;
                assert_eq!(
                    &source.rgba[offset..offset + 4],
                    &clipped.rgba[offset..offset + 4],
                    "symbol mask changed legend pixels"
                );
                legend += usize::from(b.iter().any(|v| *v));
            }
        }
    }
    assert!(
        source_outside.iter().all(|n| *n > 30),
        "fixture must exercise actual fill/stroke/font beyond disk: {source_outside:?}"
    );
    assert_eq!(clipped_outside, [0, 0, 0]);
    assert!(
        inside.iter().all(|n| *n > 10),
        "mask removed all actual symbol stroke/font: {inside:?}"
    );
    assert!(legend > 100, "actual legend ink was not drawn");
    let svg = kartis::render::to_svg(&masked.scene).unwrap();
    assert!(svg.contains("clipPath") && svg.contains("MASK LABEL"));
}
