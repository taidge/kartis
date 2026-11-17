use glory::render::PathCommand;
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 960.,
        height: 500.,
        margin: Margin {
            left: 0.,
            right: 0.,
            top: 0.,
            bottom: 0.,
        },
        ..Default::default()
    }
}
fn ring(a: f64, b: f64, c: f64, d: f64) -> Vec<[f64; 2]> {
    vec![[a, b], [c, b], [c, d], [a, d], [a, b]]
}
fn source() -> Vec<GeoFeature> {
    vec![GeoFeature {
        id: "all-regions".into(),
        label: "Three regions".into(),
        value: Some(10.),
        polygons: vec![
            vec![ring(-125., 34., -118., 42.), ring(-123., 36., -121., 39.)],
            vec![ring(-152., 59., -146., 65.)],
            vec![ring(-160., 19., -155., 23.)],
        ],
    }]
}
fn manual() -> GeoUsaMapOptions {
    GeoUsaMapOptions {
        scale: Some(1070.),
        translate: Some([480., 250.]),
        border: None,
        ..Default::default()
    }
}
fn vertices(scene: &Scene) -> Vec<[f64; 2]> {
    scene
        .marks
        .iter()
        .flat_map(|m| match &m.geometry {
            Geometry::Path(p) => p
                .iter()
                .filter_map(|c| match c {
                    PathCommand::Move(p) | PathCommand::Line(p) => Some([p.x, p.y]),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            _ => vec![],
        })
        .collect()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() <= 1e-8, "{a} != {b}");
}

#[test]
fn canonical_feature_keeps_three_regions_and_holes_under_one_source_identity() {
    let source = source();
    let before = source.clone();
    let layout = geographic_usa_map_layout(&config(), &source, &manual()).unwrap();
    assert_eq!(source, before);
    assert_eq!(layout.scene.marks.len(), 1);
    let m = &layout.scene.marks[0];
    assert_eq!(m.id, "map:all-regions");
    assert_eq!(m.series, "all-regions");
    assert_eq!(m.label, "Three regions");
    assert_eq!(m.value, 10.);
    assert!(layout.projection.projection_disk().is_none());
    for (geo, region) in [
        ([-124., 35.], GeoUsaRegion::Lower48),
        ([-149.9, 61.2], GeoUsaRegion::Alaska),
        ([-157.8583, 21.3069], GeoUsaRegion::Hawaii),
    ] {
        let p = layout.projection.project_usa(geo).unwrap().unwrap();
        assert_eq!(p.region, region);
        assert_eq!(layout.projection.project(geo).unwrap(), Some(p.position));
        assert_eq!(
            layout
                .scene
                .hit_test(p.position[0], p.position[1])
                .unwrap()
                .id,
            m.id
        );
        let inverse = layout
            .projection
            .invert_usa_in_region(region, p.position)
            .unwrap()
            .unwrap();
        close(inverse[0], geo[0]);
        close(inverse[1], geo[1]);
    }
    let hole = layout
        .projection
        .project([-122.4194, 37.7749])
        .unwrap()
        .unwrap();
    assert!(layout.scene.hit_test(hole[0], hole[1]).is_none());
    assert_eq!(layout.projection.usa_map_stats().unwrap().passes, 1);
}

#[test]
fn world_fit_uses_the_composite_rectangle_and_old_context_rejects_usa_specific_methods() {
    let c = config();
    let options = GeoUsaMapOptions {
        padding: 10.,
        ..Default::default()
    };
    let empty = geographic_usa_map_layout(&c, &[], &options).unwrap();
    close(
        empty.projection.scale(),
        ((c.inner_width() - 20.) / 0.91).min((c.inner_height() - 20.) / 0.476),
    );
    assert_eq!(empty.projection.fitted_raw_center(), [0., 0.]);
    assert_eq!(empty.projection.translate(), [480., 250.]);
    let core = empty.projection.usa_projection().unwrap();
    assert_eq!(core.translate(), empty.projection.translate());
    assert_eq!(geographic_usa_map(&c, &[], &options).unwrap(), empty.scene);
    let old = geographic_map_layout(&c, &[], &GeoMapOptions::default()).unwrap();
    assert!(old.projection.usa_projection().is_none());
    assert!(old.projection.usa_map_stats().is_none());
    assert!(old.projection.project_usa([-100., 40.]).is_err());
    assert!(old.projection.invert_usa([480., 250.]).is_err());
}

#[test]
fn data_fit_uses_complete_scale_150_postclip_bounds_and_rebuilds_final_epsilon() {
    let c = config();
    let source = source();
    let fit_stage = geographic_usa_map_layout(
        &c,
        &source,
        &GeoUsaMapOptions {
            scale: Some(150.),
            translate: Some([0., 0.]),
            border: None,
            ..Default::default()
        },
    )
    .unwrap();
    let points = vertices(&fit_stage.scene);
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for p in points {
        b[0] = b[0].min(p[0] / 150.);
        b[1] = b[1].min(-p[1] / 150.);
        b[2] = b[2].max(p[0] / 150.);
        b[3] = b[3].max(-p[1] / 150.);
    }
    let opts = GeoUsaMapOptions {
        fit: GeoFit::Data,
        padding: 12.,
        border: None,
        ..Default::default()
    };
    let layout = geographic_usa_map_layout(&c, &source, &opts).unwrap();
    let scale =
        ((c.inner_width() - 24.) / (b[2] - b[0])).min((c.inner_height() - 24.) / (b[3] - b[1]));
    close(layout.projection.scale(), scale);
    let center = [(b[0] + b[2]) / 2., (b[1] + b[3]) / 2.];
    let actual = layout.projection.fitted_raw_center();
    close(actual[0], center[0]);
    close(actual[1], center[1]);
    let core = layout.projection.usa_projection().unwrap();
    close(core.translate()[0], 480. - scale * center[0]);
    close(core.translate()[1], 250. + scale * center[1]);
    let ak = core.region_clip(GeoUsaRegion::Alaska);
    close(ak.x, core.translate()[0] - 0.425 * scale + 1e-6);
    let stats = layout.projection.usa_map_stats().unwrap();
    assert_eq!(stats.passes, 2);
    assert!(stats.work > fit_stage.projection.usa_map_stats().unwrap().work);
}

#[test]
fn shared_admission_includes_all_regions_and_both_data_fit_passes() {
    let c = config();
    let source = source();
    let opts = GeoUsaMapOptions {
        fit: GeoFit::Data,
        border: None,
        ..Default::default()
    };
    let accepted = geographic_usa_map_layout(&c, &source, &opts).unwrap();
    let stats = accepted.projection.usa_map_stats().unwrap();
    assert!(stats.points > 2 && stats.work > 2);
    let exact = GeoUsaMapOptions {
        path: GeoPathOptions {
            max_points: stats.points,
            max_work: stats.work,
            ..opts.path.clone()
        },
        ..opts.clone()
    };
    assert_eq!(
        geographic_usa_map_layout(&c, &source, &exact)
            .unwrap()
            .scene,
        accepted.scene
    );
    let fewer_points = GeoUsaMapOptions {
        path: GeoPathOptions {
            max_points: stats.points - 1,
            ..opts.path.clone()
        },
        ..opts.clone()
    };
    let fewer_work = GeoUsaMapOptions {
        path: GeoPathOptions {
            max_work: stats.work - 1,
            ..opts.path.clone()
        },
        ..opts
    };
    assert!(geographic_usa_map_layout(&c, &source, &fewer_points).is_err());
    assert!(geographic_usa_map_layout(&c, &source, &fewer_work).is_err());
}

#[test]
fn source_colors_empty_fallback_invalid_frames_and_serializers_preserve_existing_map_contract() {
    let c = config();
    let mut source = source();
    source.push(GeoFeature {
        id: "missing".into(),
        label: "No value".into(),
        value: None,
        polygons: vec![vec![ring(-110., 35., -105., 40.)]],
    });
    let fallback = Color(250, 10, 90, 255);
    let opts = GeoUsaMapOptions {
        fill: Some(fallback),
        ..manual()
    };
    let painted = choropleth_usa_layout(&c, &source, &opts).unwrap();
    assert_eq!(painted.scene.marks[0].fill, Some(Color(135, 180, 220, 255)));
    assert_eq!(painted.scene.marks[1].fill, Some(fallback));
    assert_eq!(
        choropleth_usa_map(&c, &source, &opts).unwrap(),
        painted.scene
    );
    let svg = render::to_svg(&painted.scene).unwrap();
    assert!(svg.contains("<path"));
    render::to_canvas_script(&painted.scene, "usa").unwrap();
    for bad in [
        GeoUsaMapOptions {
            padding: f64::NAN,
            ..manual()
        },
        GeoUsaMapOptions {
            scale: Some(1e-6),
            ..manual()
        },
        GeoUsaMapOptions {
            path: GeoPathOptions {
                precision: 0.,
                ..Default::default()
            },
            ..manual()
        },
    ] {
        assert!(geographic_usa_map_layout(&c, &source, &bad).is_err());
    }
    let empty = geographic_usa_map_layout(
        &c,
        &[],
        &GeoUsaMapOptions {
            fit: GeoFit::Data,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(empty.scene.marks.is_empty());
    assert!(empty.projection.usa_projection().is_some());
    let mut duplicate = source.clone();
    duplicate.push(source[0].clone());
    assert!(geographic_usa_map_layout(&c, &duplicate, &opts).is_err());
}

#[cfg(feature = "raster")]
#[test]
fn real_cpu_raster_contains_all_three_parts_and_keeps_the_mainland_hole_clear() {
    use render::raster::{FontRegistry, RasterOptions, render_raster};
    let color = Color(220, 25, 80, 255);
    let layout = geographic_usa_map_layout(
        &config(),
        &source(),
        &GeoUsaMapOptions {
            fill: Some(color),
            ..manual()
        },
    )
    .unwrap();
    let image = render_raster(
        &layout.scene,
        &FontRegistry::new(),
        &RasterOptions::default(),
    )
    .unwrap();
    let rgba = |point: [f64; 2]| {
        let i = (point[1].floor() as usize * image.width as usize + point[0].floor() as usize) * 4;
        &image.rgba[i..i + 4]
    };
    for geo in [[-124., 35.], [-149.9, 61.2], [-157.8583, 21.3069]] {
        let p = layout.projection.project(geo).unwrap().unwrap();
        assert_eq!(rgba(p), &[220, 25, 80, 255]);
    }
    let hole = layout
        .projection
        .project([-122.4194, 37.7749])
        .unwrap()
        .unwrap();
    assert_ne!(rgba(hole), &[220, 25, 80, 255]);
}
