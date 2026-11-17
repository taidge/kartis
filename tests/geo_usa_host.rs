#![cfg(feature = "raster")]
use glory::App;
use kartis::render::raster::{FontRegistry, RasterOptions};
use kartis::surface::SurfaceInput;
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 640.,
        height: 400.,
        margin: Margin {
            left: 30.,
            right: 30.,
            top: 30.,
            bottom: 30.,
        },
        ..Default::default()
    }
}
fn points() -> [[f64; 2]; 3] {
    [[-122.4194, 37.7749], [-149.9, 61.2], [-157.8583, 21.3069]]
}
fn features() -> Vec<GeoFeature> {
    points()
        .iter()
        .enumerate()
        .map(|(i, [x, y])| GeoFeature {
            id: format!("region-{i}"),
            label: format!("region {i}"),
            value: Some(i as f64),
            polygons: vec![vec![vec![
                [x - 2., y - 1.],
                [x - 2., y + 1.],
                [x + 2., y + 1.],
                [x + 2., y - 1.],
            ]]],
        })
        .collect()
}
fn bounds(units: f64) -> RasterSurfaceBounds {
    RasterSurfaceBounds {
        x: 127.25 * units,
        y: 93.5 * units,
        width: 960. * units,
        height: 600. * units,
    }
}
fn host(p: (f64, f64), b: RasterSurfaceBounds) -> (f64, f64) {
    (b.x + p.0 / 640. * b.width, b.y + p.1 / 400. * b.height)
}
fn near(a: [f64; 2], b: [f64; 2]) {
    assert!((a[0] - b[0]).hypot(a[1] - b[1]) < 1e-8, "{a:?} != {b:?}");
}

fn data_controls(app: &App) -> usize {
    let tree = app.tree();
    let mut pending = tree.roots().to_vec();
    let mut count = 0;
    while let Some(id) = pending.pop() {
        let node = tree.node(id).unwrap();
        count += usize::from(
            node.descriptor
                .view()
                .classes
                .iter()
                .any(|c| c == "kartis-datum"),
        );
        pending.extend(node.children.iter().copied());
    }
    count
}

#[test]
fn native_usa_regions_share_real_cpu_ink_typed_controls_and_one_host_camera_removal() {
    for fit in [GeoFit::World, GeoFit::Data] {
        for dpr in [1., 2.] {
            let app = App::memory();
            let mut map = geographic_usa_map_layout(
                &config(),
                &features(),
                &GeoUsaMapOptions {
                    fit,
                    padding: 10.,
                    ..Default::default()
                },
            )
            .unwrap();
            let symbols = points()
                .iter()
                .enumerate()
                .map(|(i, p)| GeoSymbol::new(format!("place-{i}"), *p, Some(7.)))
                .collect::<Vec<_>>();
            let layer = map
                .add_glyphs(
                    &symbols,
                    &GeoGlyphOptions {
                        shape: GlyphShape::Filled(FilledGlyph::Diamond),
                        symbols: GeoSymbolOptions {
                            clip_to_projection: true,
                            radius_range: [6., 6.],
                            color: GeoSymbolColor::Fixed(Color(240, 15, 180, 255)),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                )
                .unwrap();
            assert_eq!(layer.sites.len(), 3);
            let ctx = map.projection;
            let binding = layer.interaction_binding(&map.scene).unwrap();
            let surface = RasterSurface::mount_layout(
                app.scope(),
                SceneInteraction {
                    scene: map.scene,
                    interaction: Some(binding),
                    plot: Some(ctx.plot()),
                },
                FontRegistry::new(),
                RasterSurfaceOptions {
                    highlight: false,
                    raster: RasterOptions {
                        pixel_ratio: dpr,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .unwrap();
            app.mount(surface.widget()).unwrap();
            assert_eq!(data_controls(&app), 3);
            let handle = surface.handle();
            for camera in [1_f64, 1.15] {
                if camera > 1. {
                    handle
                        .dispatch_host(
                            SurfaceInput::Wheel {
                                point: host((320., 200.), bounds(1.)),
                                delta: -camera.ln() / 0.002,
                            },
                            bounds(1.),
                        )
                        .unwrap();
                }
                let snapshot = handle.snapshot().unwrap();
                assert!((snapshot.view.scale - camera).abs() < 1e-12);
                let plot = ctx.plot();
                let transform = ZoomTransform {
                    scale: snapshot.view.scale,
                    translate_x: plot.x + snapshot.view.translate_x - plot.x * snapshot.view.scale,
                    translate_y: plot.y + snapshot.view.translate_y - plot.y * snapshot.view.scale,
                };
                let image = handle.frame().unwrap();
                assert_eq!(
                    (image.width, image.height),
                    ((640. * dpr) as u32, (400. * dpr) as u32)
                );
                for p in points() {
                    let projected = ctx.project_usa(p).unwrap().unwrap();
                    let pixel = transform
                        .project((projected.position[0], projected.position[1]))
                        .unwrap();
                    let x = (pixel.0 * dpr).floor() as usize;
                    let y = (pixel.1 * dpr).floor() as usize;
                    let index = (y * image.width as usize + x) * 4;
                    assert!(
                        image.rgba[index] > 200
                            && image.rgba[index + 1] < 50
                            && image.rgba[index + 2] > 130,
                        "actual glyph pixel missing {fit:?}/{dpr}/{camera}/{p:?}"
                    );
                    for units in [1., 2.] {
                        let b = bounds(units);
                        let input = host(pixel, b);
                        let scene = handle.scene_point_at_host(input, b).unwrap();
                        near([scene.0, scene.1], projected.position);
                        near(
                            handle.invert_geo_at_host(&ctx, input, b).unwrap().unwrap(),
                            p,
                        );
                        let found = handle.invert_usa_at_host(&ctx, input, b).unwrap().unwrap();
                        assert_eq!(found.region, projected.region);
                        near(found.position, p);
                        near(
                            handle
                                .invert_usa_in_region_at_host(&ctx, projected.region, input, b)
                                .unwrap()
                                .unwrap(),
                            p,
                        );
                        handle
                            .dispatch_host(
                                SurfaceInput::Move {
                                    id: 1,
                                    point: input,
                                },
                                b,
                            )
                            .unwrap();
                        let active = handle.active_interaction().unwrap().unwrap();
                        near(active.anchor, projected.position);
                        assert_eq!(active.rows[0].value, Some(7.));
                    }
                }
            }
            surface.stop();
            app.flush().unwrap();
            assert_eq!(data_controls(&app), 0);
            assert!(
                handle
                    .invert_usa_at_host(&ctx, (0., 0.), bounds(1.))
                    .is_err()
            );
        }
    }
}

#[test]
fn native_usa_known_region_rejects_noncomposite_context_and_wrong_dimensions() {
    let app = App::memory();
    let plain = geographic_map_layout(&config(), &[], &Default::default())
        .unwrap()
        .projection;
    let surface = RasterSurface::mount(
        app.scope(),
        config().scene().unwrap(),
        FontRegistry::new(),
        Default::default(),
    )
    .unwrap();
    let handle = surface.handle();
    assert!(
        handle
            .invert_usa_at_host(&plain, host((320., 200.), bounds(1.)), bounds(1.))
            .is_err()
    );
    assert!(
        handle
            .invert_usa_in_region_at_host(
                &plain,
                GeoUsaRegion::Alaska,
                host((320., 200.), bounds(1.)),
                bounds(1.)
            )
            .is_err()
    );
    let wrong = geographic_usa_map_layout(
        &ChartConfig {
            width: 800.,
            ..config()
        },
        &[],
        &Default::default(),
    )
    .unwrap()
    .projection;
    assert!(
        handle
            .invert_usa_at_host(&wrong, host((320., 200.), bounds(1.)), bounds(1.))
            .is_err()
    );
}

#[test]
fn location_label_mask_removes_actual_cpu_text_inside_the_broad_mainland_rectangle() {
    use kartis::render::raster::{FaceDescriptor, render_raster};
    let mut fonts = FontRegistry::new();
    let id = fonts
        .register(include_bytes!("fonts/Lato-Regular.ttf").as_slice(), 0)
        .unwrap();
    fonts
        .bind_face(
            id,
            FaceDescriptor {
                family: FontFamily::Named("LabelFixture".into()),
                weight: 400.,
            },
        )
        .unwrap();
    let base = geographic_usa_map_layout(
        &ChartConfig {
            width: 960.,
            height: 560.,
            ..Default::default()
        },
        &[],
        &GeoUsaMapOptions {
            scale: Some(1070.),
            translate: Some([480., 250.]),
            ..Default::default()
        },
    )
    .unwrap();
    let mut original = base.clone();
    let options = GeoSymbolOptions {
        clip: false,
        labels: true,
        label_color: Color(240, 15, 180, 255),
        radius_range: [60., 60.],
        ..Default::default()
    };
    let symbols = [GeoSymbol::new(
        "VISIBLE TEXT",
        [-157.8583, 21.3069],
        Some(1.),
    )];
    original.add_symbols(&symbols, &options).unwrap();
    let mut masked = base;
    masked
        .add_symbols(
            &symbols,
            &GeoSymbolOptions {
                clip_labels_to_projection: true,
                ..options
            },
        )
        .unwrap();
    assert_eq!(original.scene.marks, masked.scene.marks);
    assert_eq!(original.scene.labels, masked.scene.labels);
    for scene in [&mut original.scene, &mut masked.scene] {
        scene.typography.default = FontOptions {
            family: Some(vec![FontFamily::Named("LabelFixture".into())]),
            weight: Some(400.),
        };
    }
    let core = masked.projection.usa_projection().unwrap();
    let label = &masked.scene.labels[0];
    assert!(
        core.region_clip(GeoUsaRegion::Lower48)
            .contains(label.x, label.y)
    );
    assert!(
        !core
            .region_clip(GeoUsaRegion::Hawaii)
            .contains(label.x, label.y)
    );
    for dpr in [1., 2.] {
        let opt = RasterOptions {
            pixel_ratio: dpr,
            ..Default::default()
        };
        let a = render_raster(&original.scene, &fonts, &opt).unwrap();
        let b = render_raster(&masked.scene, &fonts, &opt).unwrap();
        let count = |pixels: &[u8]| {
            pixels
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|p| p[0] > 200 && p[1] < 60 && p[2] > 130)
                .count()
        };
        assert!(count(&a.rgba) > 30);
        assert_eq!(count(&b.rgba), 0);
    }
}
