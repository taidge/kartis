#![cfg(feature = "raster")]
use glory::App;
use kartis::{render::raster::*, surface::SurfaceInput, *};
use std::{cell::Cell, rc::Rc};

fn config() -> ChartConfig {
    ChartConfig {
        width: 400.,
        height: 280.,
        margin: Margin {
            left: 30.,
            top: 20.,
            right: 20.,
            bottom: 25.,
        },
        ..Default::default()
    }
}
fn kinds() -> [GeoProjection; 10] {
    [
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
    ]
}
fn layout(kind: GeoProjection, fit: GeoFit) -> GeoMapLayout {
    let features = if fit == GeoFit::Data {
        vec![GeoFeature {
            id: "region".into(),
            label: "Region".into(),
            value: None,
            polygons: vec![vec![vec![
                [40., 22.],
                [40., 30.],
                [52., 30.],
                [52., 22.],
                [40., 22.],
            ]]],
        }]
    } else {
        Vec::new()
    };
    geographic_map_layout(
        &config(),
        &features,
        &GeoMapOptions {
            projection: kind,
            fit,
            center: [35., 20.],
            rotation: 23.,
            scale: Some(75.),
            translate: Some([210., 145.]),
            ..Default::default()
        },
    )
    .unwrap()
}
fn bounds(scale: f64) -> RasterSurfaceBounds {
    RasterSurfaceBounds {
        x: 127.25 * scale,
        y: 93.5 * scale,
        width: 640. * scale,
        height: 420. * scale,
    }
}
fn host(point: (f64, f64), b: RasterSurfaceBounds) -> (f64, f64) {
    (
        b.x + point.0 / 400. * b.width,
        b.y + point.1 / 280. * b.height,
    )
}
fn paint(view: ZoomTransform, plot: ClipRect) -> ZoomTransform {
    ZoomTransform {
        scale: view.scale,
        translate_x: plot.x + view.translate_x - plot.x * view.scale,
        translate_y: plot.y + view.translate_y - plot.y * view.scale,
    }
}
fn near(a: (f64, f64), b: (f64, f64)) {
    assert!(
        (a.0 - b.0).hypot(a.1 - b.1) < 1e-8,
        "{a:?} differs from {b:?}"
    );
}
fn colored_at(image: &RenderedImage, point: (f64, f64)) -> bool {
    let x = (point.0 * image.pixel_ratio).floor() as i32;
    let y = (point.1 * image.pixel_ratio).floor() as i32;
    (-2..=2).any(|dy| {
        (-2..=2).any(|dx| {
            let (x, y) = (x + dx, y + dy);
            if x < 0 || y < 0 || x >= image.width as i32 || y >= image.height as i32 {
                return false;
            }
            let i = (y as usize * image.width as usize + x as usize) * 4;
            image.rgba[i] > 190 && image.rgba[i + 1] < 50 && image.rgba[i + 2] > 130
        })
    })
}

#[test]
fn actual_cpu_mark_and_host_input_share_one_inverse_at_both_dprs_and_scene_cameras() {
    for kind in kinds() {
        for fit in [GeoFit::World, GeoFit::Data] {
            for dpr in [1., 2.] {
                for camera in [1_f64, 2.] {
                    let app = App::memory();
                    let mut map = layout(kind, fit);
                    let ctx = map.projection;
                    if fit == GeoFit::Data {
                        assert!(ctx.fitted_raw_center().iter().any(|v| v.abs() > 0.001));
                    }
                    let position = [45., 25.];
                    let scene_point = ctx.project(position).unwrap().unwrap();
                    let mut marker = Mark::new(
                        "pointer-marker",
                        Geometry::Circle {
                            x: scene_point[0],
                            y: scene_point[1],
                            radius: 5.,
                        },
                        Color(240, 15, 180, 255),
                    );
                    marker.series = "locations".into();
                    marker.label = "Pointer marker".into();
                    map.scene.marks.push(marker);
                    map.scene.draw_order.clear();
                    let plot = ctx.plot();
                    let surface = RasterSurface::mount(
                        app.scope(),
                        map.scene,
                        FontRegistry::new(),
                        RasterSurfaceOptions {
                            highlight: false,
                            interaction: surface::SurfaceOptions {
                                plot: Some(plot),
                                ..Default::default()
                            },
                            raster: RasterOptions {
                                pixel_ratio: dpr,
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                    )
                    .unwrap();
                    let handle = surface.handle();
                    if camera != 1. {
                        handle
                            .dispatch_host(
                                SurfaceInput::Wheel {
                                    point: host((210., 145.), bounds(1.)),
                                    delta: -camera.ln() / 0.002,
                                },
                                bounds(1.),
                            )
                            .unwrap();
                    }
                    let snapshot = handle.snapshot().unwrap();
                    assert!((snapshot.view.scale - camera).abs() < 1e-12);
                    let transform = paint(snapshot.view, plot);
                    if camera != 1. {
                        assert!(transform.translate_x != 0. && transform.translate_y != 0.);
                    }
                    let viewport = transform.project((scene_point[0], scene_point[1])).unwrap();
                    let image = handle.frame().unwrap();
                    assert_eq!(
                        (image.width, image.height),
                        ((400. * dpr) as u32, (280. * dpr) as u32)
                    );
                    assert!(
                        colored_at(&image, viewport),
                        "missing actual CPU pixel {kind:?}/{fit:?}/DPR{dpr}/camera{camera}"
                    );
                    // Apace logical host units and a physical-pixel host use distinct,
                    // offset/scaled bounds and must reach the same accepted Scene point.
                    for units in [1., 2.] {
                        let b = bounds(units);
                        let input = host(viewport, b);
                        near(
                            handle.scene_point_at_host(input, b).unwrap(),
                            (scene_point[0], scene_point[1]),
                        );
                        let inverse = handle.invert_geo_at_host(&ctx, input, b).unwrap().unwrap();
                        near((inverse[0], inverse[1]), (position[0], position[1]));
                        assert_eq!(
                            handle.invert_geo_in_plot_at_host(&ctx, input, b).unwrap(),
                            Some(inverse)
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
                        assert!(
                            handle
                                .snapshot()
                                .unwrap()
                                .hovered
                                .iter()
                                .any(|id| id == "pointer-marker")
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn intrinsic_and_both_plot_clips_remain_distinct_under_native_camera() {
    let app = App::memory();
    let ctx = geographic_map_layout(
        &config(),
        &[],
        &GeoMapOptions {
            scale: Some(10000.),
            translate: Some([210., 145.]),
            ..Default::default()
        },
    )
    .unwrap()
    .projection;
    let plot = ctx.plot();
    let surface = RasterSurface::mount(
        app.scope(),
        config().scene().unwrap(),
        FontRegistry::new(),
        RasterSurfaceOptions {
            interaction: surface::SurfaceOptions {
                plot: Some(plot),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    let handle = surface.handle();
    handle
        .dispatch(SurfaceInput::Wheel {
            point: (210., 145.),
            delta: -2_f64.ln() / 0.002,
        })
        .unwrap();
    let point = (plot.x - 0.01, 145.);
    let input = host(point, bounds(1.));
    let scene = handle.scene_point_at_host(input, bounds(1.)).unwrap();
    assert!(plot.contains(scene.0, scene.1));
    assert!(
        handle
            .invert_geo_at_host(&ctx, input, bounds(1.))
            .unwrap()
            .is_some()
    );
    assert_eq!(
        handle
            .invert_geo_in_plot_at_host(&ctx, input, bounds(1.))
            .unwrap(),
        None
    );
    let surface2 = RasterSurface::mount(
        app.scope(),
        config().scene().unwrap(),
        FontRegistry::new(),
        Default::default(),
    )
    .unwrap();
    let handle2 = surface2.handle();
    // Here the viewport is the whole image, but the authored projection plot is smaller.
    assert!(
        handle2
            .invert_geo_at_host(&ctx, input, bounds(1.))
            .unwrap()
            .is_some()
    );
    assert_eq!(
        handle2
            .invert_geo_in_plot_at_host(&ctx, input, bounds(1.))
            .unwrap(),
        None
    );
    for point in [
        (plot.x, plot.y),
        (plot.x + plot.width, plot.y + plot.height),
    ] {
        assert!(
            handle2
                .invert_geo_in_plot_at_host(&ctx, host(point, bounds(1.)), bounds(1.))
                .unwrap()
                .is_some()
        );
    }
    let globe = geographic_map_layout(
        &config(),
        &[],
        &GeoMapOptions {
            projection: GeoProjection::Orthographic,
            scale: Some(50.),
            translate: Some([210., 145.]),
            ..Default::default()
        },
    )
    .unwrap()
    .projection;
    assert_eq!(
        handle2
            .invert_geo_at_host(&globe, host((300., 145.), bounds(1.)), bounds(1.))
            .unwrap(),
        None
    );
}

#[test]
fn rejected_queries_do_not_repaint_publish_or_mutate_and_dimension_change_is_checked() {
    let app = App::memory();
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let map = layout(GeoProjection::TransverseMercator, GeoFit::Data);
    let ctx = map.projection;
    let surface = RasterSurface::mount_published(
        app.scope(),
        map.scene,
        FontRegistry::new(),
        Default::default(),
        move |_| {
            count.set(count.get() + 1);
            Ok("application-owned-frame".into())
        },
    )
    .unwrap();
    let handle = surface.handle();
    let before = handle.snapshot().unwrap();
    let image = handle.frame().unwrap();
    let projected = ctx.project([45., 25.]).unwrap().unwrap();
    let valid_host = host((projected[0], projected[1]), bounds(1.));
    assert!(handle.scene_point_at_host(valid_host, bounds(1.)).is_ok());
    assert!(
        handle
            .invert_geo_at_host(&ctx, valid_host, bounds(1.))
            .unwrap()
            .is_some()
    );
    assert!(
        handle
            .invert_geo_in_plot_at_host(&ctx, valid_host, bounds(1.))
            .unwrap()
            .is_some()
    );
    for b in [
        RasterSurfaceBounds {
            width: 0.,
            ..bounds(1.)
        },
        RasterSurfaceBounds {
            height: -1.,
            ..bounds(1.)
        },
        RasterSurfaceBounds {
            x: f64::NAN,
            ..bounds(1.)
        },
        RasterSurfaceBounds {
            height: f64::INFINITY,
            ..bounds(1.)
        },
        RasterSurfaceBounds {
            width: f64::MIN_POSITIVE,
            ..bounds(1.)
        },
    ] {
        for result in [
            handle.scene_point_at_host((400., 200.), b).map(|_| ()),
            handle.invert_geo_at_host(&ctx, (400., 200.), b).map(|_| ()),
            handle
                .invert_geo_in_plot_at_host(&ctx, (400., 200.), b)
                .map(|_| ()),
        ] {
            assert!(result.is_err());
        }
    }
    for p in [(f64::NAN, 0.), (0., f64::INFINITY), (1e300, 0.)] {
        assert!(handle.scene_point_at_host(p, bounds(1.)).is_err());
        assert!(handle.invert_geo_at_host(&ctx, p, bounds(1.)).is_err());
        assert!(
            handle
                .invert_geo_in_plot_at_host(&ctx, p, bounds(1.))
                .is_err()
        );
    }
    assert_eq!(calls.get(), 1);
    assert_eq!(handle.snapshot().unwrap(), before);
    assert!(Rc::ptr_eq(&image, &handle.frame().unwrap()));
    let mut replacement = config().scene().unwrap();
    replacement.width = 401.;
    handle.set_scene(replacement).unwrap();
    let accepted = handle.snapshot().unwrap();
    let frame = handle.frame().unwrap();
    for p in [(200., 150.), (-1000., -1000.)] {
        assert!(handle.invert_geo_at_host(&ctx, p, bounds(1.)).is_err());
        assert!(
            handle
                .invert_geo_in_plot_at_host(&ctx, p, bounds(1.))
                .is_err()
        );
    }
    assert_eq!(handle.snapshot().unwrap(), accepted);
    assert!(Rc::ptr_eq(&frame, &handle.frame().unwrap()));
    assert_eq!(calls.get(), 2);
}

#[test]
fn stopped_dropped_and_owner_disposed_surfaces_retire_coordinate_access() {
    let app = App::memory();
    let map = layout(GeoProjection::Mercator, GeoFit::World);
    let ctx = map.projection;
    let assert_retired = |handle: &RasterSurfaceHandle| {
        assert!(handle.scene_point_at_host((0., 0.), bounds(1.)).is_err());
        assert!(
            handle
                .invert_geo_at_host(&ctx, (0., 0.), bounds(1.))
                .is_err()
        );
        assert!(
            handle
                .invert_geo_in_plot_at_host(&ctx, (0., 0.), bounds(1.))
                .is_err()
        );
    };
    let surface = RasterSurface::mount(
        app.scope(),
        map.scene.clone(),
        FontRegistry::new(),
        Default::default(),
    )
    .unwrap();
    let handle = surface.handle();
    surface.stop();
    assert_retired(&handle);
    let surface = RasterSurface::mount(
        app.scope(),
        map.scene.clone(),
        FontRegistry::new(),
        Default::default(),
    )
    .unwrap();
    let handle = surface.handle();
    drop(surface);
    assert_retired(&handle);
    let owner = app.scope().child().unwrap();
    let surface =
        RasterSurface::mount(&owner, map.scene, FontRegistry::new(), Default::default()).unwrap();
    let handle = surface.handle();
    owner.owner().dispose();
    assert_retired(&handle);
}
