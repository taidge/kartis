use glory::render::PathCommand as Cmd;
use kartis::*;
const RED: Color = Color(230, 30, 40, 255);
fn config() -> ChartConfig {
    ChartConfig {
        width: 420.,
        height: 420.,
        margin: Margin {
            top: 10.,
            bottom: 10.,
            left: 10.,
            right: 10.,
        },
        ..Default::default()
    }
}
fn shapes() -> [GeoSymbolShape; 5] {
    [
        GeoSymbolShape::Circle,
        GeoSymbolShape::Square,
        GeoSymbolShape::Diamond,
        GeoSymbolShape::Triangle,
        GeoSymbolShape::Cross,
    ]
}
fn projections() -> [GeoProjection; 5] {
    [
        GeoProjection::Orthographic,
        GeoProjection::AzimuthalEqualArea,
        GeoProjection::AzimuthalEquidistant,
        GeoProjection::Stereographic,
        GeoProjection::Gnomonic,
    ]
}
fn options(shape: GeoSymbolShape, r: f64) -> GeoSymbolOptions {
    GeoSymbolOptions {
        shape,
        clip: false,
        clip_to_projection: true,
        stroke: None,
        radius_range: [r, r],
        color: GeoSymbolColor::Fixed(RED),
        ..Default::default()
    }
}
fn projected_position(center: [f64; 2], roll: f64, distance: f64, bearing: f64) -> [f64; 2] {
    let (distance, bearing, roll, latitude) = (
        distance.to_radians(),
        bearing.to_radians(),
        roll.to_radians(),
        center[1].to_radians(),
    );
    let x = distance.cos();
    let y = distance.sin() * bearing.cos();
    let z = distance.sin() * bearing.sin();
    let east = y * roll.cos() - z * roll.sin();
    let north = y * roll.sin() + z * roll.cos();
    [
        center[0]
            + east
                .atan2(x * latitude.cos() - north * latitude.sin())
                .to_degrees(),
        (x * latitude.sin() + north * latitude.cos())
            .asin()
            .to_degrees(),
    ]
}
fn just_symbols(scene: &Scene) -> Scene {
    let mut scene = scene.clone();
    scene.marks.retain(|m| m.id.starts_with("geo-symbol:"));
    scene.draw_order.clear();
    scene
}
#[test]
fn disk_intersection_hit_matches_original_shape_and_projection_for_every_shape_and_projection() {
    let c = config();
    for projection in projections() {
        let map = GeoMapOptions {
            projection,
            clip_angle: Some(30.),
            ..Default::default()
        };
        for shape in shapes() {
            let source = vec![GeoSymbol::new("edge", [29., 0.], Some(1.))];
            let mut clipped = geographic_map_layout(&c, &[], &map).unwrap();
            let mut original = clipped.clone();
            original
                .add_symbols(
                    &source,
                    &GeoSymbolOptions {
                        clip_to_projection: false,
                        ..options(shape, 55.)
                    },
                )
                .unwrap();
            clipped.add_symbols(&source, &options(shape, 55.)).unwrap();
            let disk = clipped.projection.projection_disk().unwrap();
            let p = clipped.projection.project([29., 0.]).unwrap().unwrap();
            let (original, clipped) = (just_symbols(&original.scene), just_symbols(&clipped.scene));
            assert_eq!(original.marks[0].id, clipped.marks[0].id);
            assert_eq!(original.marks[0].label, clipped.marks[0].label);
            assert!(
                matches!(clipped.marks[0].geometry, Geometry::Path(_)),
                "{projection:?}/{shape:?}: genuine clipped outline"
            );
            let mut kept = 0;
            let mut removed = 0;
            for dx in -65..=65 {
                for dy in (-70..=70).step_by(2) {
                    let (x, y) = (p[0] + f64::from(dx) + 0.123, p[1] + f64::from(dy) + 0.371);
                    let distance = (x - disk.center[0]).hypot(y - disk.center[1]);
                    if (distance - disk.radius).abs() < 0.02 {
                        continue;
                    }
                    let src = original.hit_test(x, y).is_some();
                    let expected = src && distance < disk.radius;
                    assert_eq!(
                        clipped.hit_test(x, y).is_some(),
                        expected,
                        "{projection:?}/{shape:?} dx={dx},dy={dy}"
                    );
                    kept += usize::from(expected);
                    removed += usize::from(src && !expected);
                }
            }
            assert!(kept > 100 && removed > 50);
            render::to_svg(&clipped).unwrap();
            render::to_canvas_script(&clipped, "symbols").unwrap();
        }
    }
}
#[test]
fn fully_inside_and_disk_enclosing_symbols_preserve_exact_geometry() {
    let c = config();
    for shape in shapes() {
        let mut map = geographic_map_layout(
            &c,
            &[],
            &GeoMapOptions {
                projection: GeoProjection::Orthographic,
                ..Default::default()
            },
        )
        .unwrap();
        let source = [GeoSymbol::new("center", [0., 0.], Some(1.))];
        let mut original = map.clone();
        original
            .add_symbols(
                &source,
                &GeoSymbolOptions {
                    clip_to_projection: false,
                    ..options(shape, 20.)
                },
            )
            .unwrap();
        map.add_symbols(&source, &options(shape, 20.)).unwrap();
        assert_eq!(map.scene, original.scene, "fully inside {shape:?}");
        let mut tiny = geographic_map_layout(
            &c,
            &[],
            &GeoMapOptions {
                projection: GeoProjection::Orthographic,
                scale: Some(15.),
                ..Default::default()
            },
        )
        .unwrap();
        let disk = tiny.projection.projection_disk().unwrap();
        tiny.add_symbols(&source, &options(shape, 100.)).unwrap();
        assert_eq!(
            tiny.scene.marks[0].geometry,
            Geometry::Circle {
                x: disk.center[0],
                y: disk.center[1],
                radius: disk.radius
            },
            "{shape:?}: entire cap contained"
        );
    }
}
#[test]
fn rotated_data_fit_clip_uses_actual_projected_center_not_scene_center() {
    let c = config();
    let center = [10., -6.];
    let roll = 31.;
    let position = |d, b| projected_position(center, roll, d, b);
    let feature = GeoFeature {
        id: "fit".into(),
        label: "fit".into(),
        value: None,
        polygons: vec![vec![vec![
            position(20., -20.),
            position(28., -20.),
            position(28., 25.),
            position(20., 25.),
        ]]],
    };
    for projection in projections() {
        let mut map = geographic_map_layout(
            &c,
            std::slice::from_ref(&feature),
            &GeoMapOptions {
                projection,
                clip_angle: Some(30.),
                fit: GeoFit::Data,
                center,
                rotation: roll,
                border: None,
                fill: Some(Color(0, 0, 0, 0)),
                ..Default::default()
            },
        )
        .unwrap();
        let disk = map.projection.projection_disk().unwrap();
        assert!((disk.center[0] - c.width / 2.).abs() > 100.);
        let source = [GeoSymbol::new("edge", position(29., 10.), Some(1.))];
        let p = map.projection.project(source[0].position).unwrap().unwrap();
        let mut original = map.clone();
        let o = options(GeoSymbolShape::Cross, 200.);
        original
            .add_symbols(
                &source,
                &GeoSymbolOptions {
                    clip_to_projection: false,
                    ..o.clone()
                },
            )
            .unwrap();
        map.add_symbols(&source, &o).unwrap();
        let (clipped, original) = (just_symbols(&map.scene), just_symbols(&original.scene));
        for dx in (-220..=220).step_by(8) {
            for dy in (-220..=220).step_by(8) {
                let (x, y) = (p[0] + f64::from(dx) + 0.123, p[1] + f64::from(dy) + 0.371);
                let d = (x - disk.center[0]).hypot(y - disk.center[1]);
                if (d - disk.radius).abs() < 0.02 {
                    continue;
                }
                assert_eq!(
                    clipped.hit_test(x, y).is_some(),
                    original.hit_test(x, y).is_some() && d < disk.radius,
                    "{projection:?}, {dx},{dy}"
                );
            }
        }
    }
}
#[test]
fn projection_clip_is_explicit_bounded_and_atomic_and_legends_labels_are_not_clipped() {
    let c = config();
    let mut map = geographic_map_layout(
        &c,
        &[],
        &GeoMapOptions {
            projection: GeoProjection::Orthographic,
            ..Default::default()
        },
    )
    .unwrap();
    let source = [GeoSymbol::new("edge", [89., 0.], Some(1.))];
    let mut outlined = map.clone();
    outlined
        .add_symbols(
            &source,
            &GeoSymbolOptions {
                clip_to_projection: true,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(
        outlined
            .scene
            .marks
            .iter()
            .find(|m| m.id.starts_with("geo-symbol:"))
            .unwrap()
            .clip_path
            .is_some()
    );
    let mut flat = geographic_map_layout(&c, &[], &GeoMapOptions::default()).unwrap();
    let before = flat.scene.clone();
    assert!(
        flat.add_symbols(&source, &options(GeoSymbolShape::Circle, 50.))
            .is_err()
    );
    assert_eq!(flat.scene, before);
    let mut tiny = geographic_map_layout(
        &c,
        &[],
        &GeoMapOptions {
            projection: GeoProjection::Orthographic,
            scale: Some(0.001),
            ..Default::default()
        },
    )
    .unwrap();
    let before = tiny.scene.clone();
    assert!(
        tiny.add_symbols(&source, &options(GeoSymbolShape::Circle, 50.))
            .is_err()
    );
    assert_eq!(tiny.scene, before);
    let mut huge = geographic_map_layout(
        &c,
        &[],
        &GeoMapOptions {
            projection: GeoProjection::Gnomonic,
            scale: Some(1_000_000.),
            ..Default::default()
        },
    )
    .unwrap();
    let before = huge.scene.clone();
    assert!(
        huge.add_symbols(&source, &options(GeoSymbolShape::Circle, 50.))
            .is_err()
    );
    assert_eq!(huge.scene, before);
    map.add_symbols(
        &source,
        &GeoSymbolOptions {
            labels: true,
            size_legend: Some(GeoSizeLegend {
                layout: GeoLegendOptions {
                    position: [405., 30.],
                    ..Default::default()
                },
                values: vec![1.],
            }),
            ..options(GeoSymbolShape::Circle, 55.)
        },
    )
    .unwrap();
    assert_eq!(
        map.scene.labels.len(),
        2,
        "symbol label plus full size legend label"
    );
    let legend = map
        .scene
        .marks
        .iter()
        .find(|m| m.id == "geo-size-legend:0")
        .unwrap();
    assert!(matches!(
        legend.geometry,
        Geometry::Circle { radius: 55., .. }
    ));
    assert_eq!(legend.clip, None);
    assert!(!map.scene.labels[0].text.is_empty());
}
#[test]
fn clip_command_budget_rejects_whole_layer_before_publishing() {
    let c = config();
    let mut map = geographic_map_layout(
        &c,
        &[],
        &GeoMapOptions {
            projection: GeoProjection::Orthographic,
            ..Default::default()
        },
    )
    .unwrap();
    let before = map.scene.clone();
    let source = (0..10_000)
        .map(|i| GeoSymbol::new(format!("p{i}"), [89., 0.], Some(1.)))
        .collect::<Vec<_>>();
    let error = map
        .add_symbols(&source, &options(GeoSymbolShape::Circle, 55.))
        .unwrap_err();
    assert!(error.0.contains("path command budget"));
    assert_eq!(map.scene, before);
}
#[test]
fn tangent_vertices_and_edges_and_minimum_spherical_cap_have_valid_outlines() {
    let c = config();
    for angle in [0.01, 30., 90.] {
        let mut map = geographic_map_layout(
            &c,
            &[],
            &GeoMapOptions {
                projection: GeoProjection::Orthographic,
                clip_angle: Some(angle),
                ..Default::default()
            },
        )
        .unwrap();
        let disk = map.projection.projection_disk().unwrap();
        let r = 2. * disk.radius / std::f64::consts::PI.sqrt();
        map.add_symbols(
            &[GeoSymbol::new("square", [0., 0.], Some(1.))],
            &options(GeoSymbolShape::Square, r),
        )
        .unwrap();
        assert!(map.scene.hit_test(disk.center[0], disk.center[1]).is_some());
        map.scene.validate().unwrap();
        let mut edge = geographic_map_layout(
            &c,
            &[],
            &GeoMapOptions {
                projection: GeoProjection::Orthographic,
                clip_angle: Some(angle),
                ..Default::default()
            },
        )
        .unwrap();
        edge.add_symbols(
            &[GeoSymbol::new("edge", [angle, 0.], Some(1.))],
            &options(GeoSymbolShape::Circle, 55.),
        )
        .unwrap();
        assert!(
            edge.scene.marks[0]
                .geometry
                .commands()
                .iter()
                .any(|c| matches!(c, Cmd::Cubic { .. }))
        );
    }
}
#[cfg(feature = "raster")]
#[test]
fn cpu_pixels_have_no_ink_beyond_projection_disk_and_keep_interior_shape() {
    use kartis::render::raster::{FontRegistry, RasterOptions, render_raster};
    let c = config();
    for projection in projections() {
        for shape in shapes() {
            let mut map = geographic_map_layout(
                &c,
                &[],
                &GeoMapOptions {
                    projection,
                    clip_angle: Some(30.),
                    ..Default::default()
                },
            )
            .unwrap();
            let mut original = map.clone();
            let source = [GeoSymbol::new("edge", [29., 0.], Some(1.))];
            original
                .add_symbols(
                    &source,
                    &GeoSymbolOptions {
                        clip_to_projection: false,
                        ..options(shape, 55.)
                    },
                )
                .unwrap();
            map.add_symbols(&source, &options(shape, 55.)).unwrap();
            let disk = map.projection.projection_disk().unwrap();
            let original = just_symbols(&original.scene);
            let image =
                render_raster(&map.scene, &FontRegistry::new(), &RasterOptions::default()).unwrap();
            let mut ink = 0;
            let mut cropped = 0;
            for y in 80..340 {
                for x in 290..420 {
                    let d = (f64::from(x) + 0.5 - disk.center[0])
                        .hypot(f64::from(y) + 0.5 - disk.center[1]);
                    let offset = ((y * image.width + x) * 4) as usize;
                    let pixel = &image.rgba[offset..offset + 4];
                    if d > disk.radius + 1.5 {
                        assert_eq!(
                            pixel,
                            &[255, 255, 255, 255],
                            "{projection:?}/{shape:?} outside disk at {x},{y}"
                        );
                        cropped += usize::from(
                            original
                                .hit_test(f64::from(x) + 0.5, f64::from(y) + 0.5)
                                .is_some(),
                        );
                    }
                    if pixel == [230, 30, 40, 255] {
                        ink += 1;
                        assert!(d <= disk.radius + 0.01);
                    }
                }
            }
            assert!(ink > 100 && cropped > 100, "{projection:?}/{shape:?}");
        }
    }
}
