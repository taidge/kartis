use glory::render::{GraphicPoint, GraphicStroke, PathCommand, StrokeCap};
use kartis::surface::{SurfaceModel, SurfaceOptions};
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
fn source() -> Vec<GeoLineFeature> {
    let mut all = GeoLineFeature::new("three", vec![]);
    all.label = "three regions".into();
    all.series = "routes".into();
    all.lines = vec![
        vec![[-124., 38.], [-119., 38.]],
        vec![[-152., 61.], [-146., 61.]],
        vec![[-159., 21.], [-156., 21.]],
    ];
    vec![all]
}
fn options() -> GeoStreamOptions {
    GeoStreamOptions {
        clip_plot: false,
        style: GeoLineStyle {
            stroke: GraphicStroke {
                color: Color(230, 20, 80, 255),
                width: 8.,
                cap: StrokeCap::Round,
            },
            ..Default::default()
        },
        ..Default::default()
    }
}
fn frame(scene: Scene, metadata: GeoUsaStreamMetadata) -> SurfaceModel {
    let frame = metadata.into_scene_interaction(scene).unwrap();
    SurfaceModel::new_with_interaction(
        frame.scene,
        SurfaceOptions {
            plot: frame.plot,
            ..Default::default()
        },
        frame.interaction,
    )
    .unwrap()
}
fn first(geometry: &Geometry) -> [f64; 2] {
    let Geometry::Path(path) = geometry else {
        panic!()
    };
    let Some(PathCommand::Move(p)) = path.first() else {
        panic!()
    };
    [p.x, p.y]
}

#[test]
fn regions_have_real_ink_and_one_nullable_canonical_source_control() {
    let mut map = map();
    map.scene.marks.push(Mark::new(
        "unrelated",
        Geometry::Circle {
            x: 800.,
            y: 100.,
            radius: 20.,
        },
        Color(0, 0, 0, 255),
    ));
    let metadata = map.add_usa_lines(&source(), &options()).unwrap();
    assert_eq!(metadata.lines.len(), 1);
    let row = &metadata.lines[0];
    assert_eq!(row.source_index, 0);
    assert_eq!(row.mark_id, "geo-line:three");
    assert_eq!(row.value, None);
    assert_eq!(row.formatted_value, None);
    assert_eq!(
        row.parts.iter().map(|p| p.region).collect::<Vec<_>>(),
        GeoUsaRegion::ALL
    );
    assert_eq!(row.parts[0].mark_id, row.mark_id);
    assert_eq!(row.parts[1].mark_id, "geo-line:three:usa:alaska");
    assert_eq!(row.parts[2].mark_id, "geo-line:three:usa:hawaii");
    let binding = metadata.interaction_binding(&map.scene).unwrap();
    assert_eq!(binding.picking_profile, PickingProfile::PaintedGeometry);
    assert_eq!(binding.sites.len(), 1);
    assert_eq!(binding.ink_owners.len(), 2);
    assert_eq!(binding.sites[0].position, first(&row.parts[0].geometry));
    assert_eq!(binding.sites[0].color, options().style.stroke.color);
    assert_eq!(binding.sites[0].value, None);
    let model = frame(map.scene.clone(), metadata.clone());
    for part in &row.parts {
        let p = first(&part.geometry);
        let hit = model.physical_hit_at((p[0], p[1])).unwrap().unwrap();
        assert_eq!(hit.physical_mark_id, part.mark_id);
        assert_eq!(hit.logical_site_id, row.mark_id);
    }
    assert!(model.physical_hit_at((800., 100.)).unwrap().is_none());
    let nearest = SurfaceOptions {
        picking: kartis::surface::Picking::Nearest { distance: 20. },
        ..Default::default()
    };
    assert!(SurfaceModel::new_with_interaction(map.scene, nearest, Some(binding)).is_err());
}

#[test]
fn shared_source_coarse_sampling_and_postclip_budgets_are_exact_and_atomic() {
    let mut original = map();
    let metadata = original.add_usa_lines(&source(), &options()).unwrap();
    let points = metadata.source_points
        + metadata.coarse_points
        + metadata.sampling.points
        + metadata.clip_stats.output_points;
    let work = metadata.sampling.work + metadata.clip_stats.work;
    assert_eq!(metadata.source_points, 6);
    assert!(metadata.coarse_points > metadata.source_points);
    assert!(metadata.clip_stats.input_points > metadata.clip_stats.output_points);
    let mut limited = options();
    limited.path = Some(GeoPathOptions {
        max_points: points,
        max_work: work,
        ..Default::default()
    });
    let mut exact = map();
    assert_eq!(exact.add_usa_lines(&source(), &limited).unwrap(), metadata);
    assert_eq!(exact.scene, original.scene);
    for point_budget in [false, true] {
        let mut bad = limited.clone();
        if point_budget {
            bad.path.as_mut().unwrap().max_points -= 1;
        } else {
            bad.path.as_mut().unwrap().max_work -= 1;
        }
        let mut target = map();
        let old = target.clone();
        assert!(target.add_usa_lines(&source(), &bad).is_err());
        assert_eq!(target, old);
    }
    let empty = (0..20)
        .map(|i| {
            let mut f = GeoLineFeature::new(format!("empty{i}"), vec![]);
            f.lines.clear();
            f
        })
        .collect::<Vec<_>>();
    let mut target = map();
    let old = target.clone();
    assert!(
        target
            .add_usa_lines(
                &empty,
                &GeoStreamOptions {
                    path: Some(GeoPathOptions {
                        max_work: 20,
                        ..Default::default()
                    }),
                    ..options()
                }
            )
            .is_err()
    );
    assert_eq!(target, old);
}

#[test]
fn whole_stroke_caps_use_selected_region_masks_independently_of_plot_clipping() {
    for cap in [StrokeCap::Butt, StrokeCap::Round, StrokeCap::Square] {
        let base = map();
        let core = base.projection.usa_projection().unwrap();
        let rect = core.region_clip(GeoUsaRegion::Alaska);
        let p = [rect.x + 1., rect.y + rect.height / 2.];
        let q = [rect.x + 35., p[1]];
        let line = GeoLineFeature::new(
            "edge",
            vec![
                core.invert_in_region(GeoUsaRegion::Alaska, p)
                    .unwrap()
                    .unwrap(),
                core.invert_in_region(GeoUsaRegion::Alaska, q)
                    .unwrap()
                    .unwrap(),
            ],
        );
        let mut o = options();
        o.style.stroke.width = 30.;
        o.style.stroke.cap = cap;
        let mut masked = base.clone();
        let metadata = masked
            .add_usa_lines(std::slice::from_ref(&line), &o)
            .unwrap();
        let part = metadata.lines[0]
            .parts
            .iter()
            .find(|p| p.region == GeoUsaRegion::Alaska)
            .unwrap();
        assert_eq!(
            masked
                .scene
                .marks
                .iter()
                .find(|m| m.id == part.mark_id)
                .unwrap()
                .clip,
            Some(rect)
        );
        let mut unmasked = base;
        o.clip_projection = false;
        let unmasked_meta = unmasked.add_usa_lines(&[line], &o).unwrap();
        let plain = unmasked_meta.lines[0]
            .parts
            .iter()
            .find(|p| p.region == GeoUsaRegion::Alaska)
            .unwrap();
        assert_eq!(plain.geometry, part.geometry);
        let a = frame(masked.scene, metadata);
        let b = frame(unmasked.scene, unmasked_meta);
        let mut count = 0;
        for x in ((rect.x - 15.).floor() as i32)..(rect.x.ceil() as i32) {
            for y in ((p[1] - 15.).floor() as i32)..((p[1] + 15.).ceil() as i32) {
                let p = (f64::from(x) + 0.5, f64::from(y) + 0.5);
                if !rect.contains(p.0, p.1) && b.physical_hit_at(p).unwrap().is_some() {
                    assert!(a.physical_hit_at(p).unwrap().is_none());
                    count += 1;
                }
            }
        }
        if cap != StrokeCap::Butt {
            assert!(count > 20, "real cap spill required {cap:?}");
        }
    }
}

#[test]
fn projection_and_plot_flags_use_native_rectangles_and_empty_intersections() {
    let c = ChartConfig {
        margin: Margin {
            left: 400.,
            right: 400.,
            top: 200.,
            bottom: 200.,
        },
        ..config()
    };
    for clip_plot in [false, true] {
        for clip_projection in [false, true] {
            let mut m = geographic_usa_map_layout(
                &c,
                &[],
                &GeoUsaMapOptions {
                    scale: Some(1070.),
                    translate: Some([480., 250.]),
                    ..Default::default()
                },
            )
            .unwrap();
            let core = m.projection.usa_projection().unwrap();
            let plot = m.projection.plot();
            let metadata = m
                .add_usa_lines(
                    &source(),
                    &GeoStreamOptions {
                        clip_plot,
                        clip_projection,
                        ..options()
                    },
                )
                .unwrap();
            for part in &metadata.lines[0].parts {
                let region = core.region_clip(part.region);
                let mark = m
                    .scene
                    .marks
                    .iter()
                    .find(|mark| mark.id == part.mark_id)
                    .unwrap();
                assert!(
                    mark.clip_path.is_none(),
                    "default Glory graphics must mount the native rectangle"
                );
                let expected = match (clip_plot, clip_projection) {
                    (false, false) => None,
                    (true, false) => Some(plot),
                    (false, true) => Some(region),
                    (true, true) => {
                        let x = plot.x.max(region.x);
                        let y = plot.y.max(region.y);
                        Some(ClipRect {
                            x,
                            y,
                            width: ((plot.x + plot.width).min(region.x + region.width) - x).max(0.),
                            height: ((plot.y + plot.height).min(region.y + region.height) - y)
                                .max(0.),
                        })
                    }
                };
                assert_eq!(mark.clip, expected);
                if clip_plot && clip_projection && part.region != GeoUsaRegion::Lower48 {
                    let rect = mark.clip.unwrap();
                    assert!(rect.width == 0. || rect.height == 0.);
                    let p = first(&part.geometry);
                    assert!(!rect.contains(p[0], p[1]));
                }
            }
            metadata.interaction_binding(&m.scene).unwrap();
        }
    }
}

#[test]
fn authored_geometry_style_clip_metadata_and_paint_order_cannot_be_rebound_stale() {
    let mut m = map();
    let metadata = m.add_usa_lines(&source(), &options()).unwrap();
    for change in 0..9 {
        let mut scene = m.scene.clone();
        match change {
            0 => scene.marks[0].stroke.as_mut().unwrap().width += 1.,
            1 => scene.marks[0].stroke.as_mut().unwrap().color = Color(1, 2, 3, 255),
            2 => scene.marks[0].clip = None,
            3 => scene.marks[0].series = "changed".into(),
            4 => scene.marks[0].value = 1.,
            5 => scene.marks[0].opacity = 0.5,
            6 => scene.marks[0].label = "changed".into(),
            7 => {
                let Geometry::Path(path) = &mut scene.marks[0].geometry else {
                    panic!()
                };
                path[0] = PathCommand::Move(GraphicPoint::new(10., 10.));
            }
            _ => scene.draw_order = vec![DrawItem::Mark(2), DrawItem::Mark(1), DrawItem::Mark(0)],
        }
        scene.validate().unwrap();
        assert!(
            metadata.interaction_binding(&scene).is_err(),
            "change {change}"
        );
    }
    let mut edited = metadata.clone();
    edited.lines[0].value = Some(42.);
    assert!(edited.interaction_binding(&m.scene).is_err());
    m.scene.marks.push(Mark::new(
        "later",
        Geometry::Circle {
            x: 800.,
            y: 100.,
            radius: 5.,
        },
        Color(0, 0, 0, 255),
    ));
    metadata.interaction_binding(&m.scene).unwrap();
}

#[test]
fn custom_dashes_original_source_indexes_and_zero_opacity_business_rows_survive() {
    let mut hidden = GeoLineFeature::new("offscreen", vec![[2., 48.], [3., 49.]]);
    hidden.value = Some(90.);
    let mut dashed = GeoLineFeature::new("dashed", vec![[-124., 38.], [-116., 38.]]);
    dashed.value = Some(42.);
    dashed.style = Some(GeoLineStyle {
        stroke: GraphicStroke {
            color: Color(10, 140, 70, 255),
            width: 2.,
            cap: StrokeCap::Butt,
        },
        dash: Some(StrokeDash {
            segments: vec![5., 35.],
            offset: 0.,
        }),
        opacity: 0.7,
    });
    let mut transparent = GeoLineFeature::new("transparent", vec![[-152., 61.], [-146., 61.]]);
    transparent.value = Some(0.);
    transparent.style = Some(GeoLineStyle {
        opacity: 0.,
        ..Default::default()
    });
    let mut m = map();
    let metadata = m
        .add_usa_lines(&[hidden, dashed.clone(), transparent], &options())
        .unwrap();
    assert!(metadata.lines[0].parts.is_empty());
    assert_eq!(metadata.lines[1].source_index, 1);
    assert_eq!(metadata.lines[2].source_index, 2);
    let binding = metadata.interaction_binding(&m.scene).unwrap();
    assert_eq!(binding.sites.len(), 2);
    assert_eq!(binding.sites[0].value, Some(42.));
    assert_eq!(binding.sites[0].formatted_value.as_deref(), Some("42"));
    assert_eq!(binding.sites[1].value, Some(0.));
    let mark = m
        .scene
        .marks
        .iter()
        .find(|mark| mark.id == "geo-line:dashed")
        .unwrap();
    assert_eq!(mark.stroke_dash, dashed.style.unwrap().dash);
    assert_eq!(mark.opacity, 0.7);
    assert_eq!(
        mark.label, "dashed",
        "numeric value is formatted once by the typed host"
    );
    let path = mark.geometry.commands();
    let point_at = |distance: f64| {
        let PathCommand::Move(mut previous) = path[0] else {
            panic!()
        };
        let mut remaining = distance;
        for command in path.iter().skip(1) {
            let PathCommand::Line(next) = *command else {
                panic!()
            };
            let length = (next.x - previous.x).hypot(next.y - previous.y);
            if remaining <= length {
                return (
                    previous.x + (next.x - previous.x) * remaining / length,
                    previous.y + (next.y - previous.y) * remaining / length,
                );
            }
            remaining -= length;
            previous = next;
        }
        panic!("test route too short")
    };
    let on = point_at(2.);
    let gap = point_at(15.);
    let transparent_point = first(&metadata.lines[2].parts[0].geometry);
    let model = frame(m.scene, metadata);
    assert_eq!(
        model.physical_hit_at(on).unwrap().unwrap().logical_site_id,
        "geo-line:dashed"
    );
    assert!(model.physical_hit_at(gap).unwrap().is_none());
    assert!(
        model
            .physical_hit_at((transparent_point[0], transparent_point[1]))
            .unwrap()
            .is_none()
    );
    assert_eq!(
        model
            .controls_scene()
            .marks
            .iter()
            .filter(|mark| !mark.label.is_empty())
            .count(),
        2
    );
}

#[test]
fn empty_source_series_and_label_keep_raw_metadata_and_one_effective_control() {
    let mut source = source();
    source[0].series.clear();
    source[0].label.clear();
    let original = source.clone();
    let mut m = map();
    let metadata = m.add_usa_lines(&source, &options()).unwrap();
    assert_eq!(source, original);
    assert_eq!(metadata.lines[0].series, "three");
    assert_eq!(metadata.lines[0].source_series, "");
    assert_eq!(metadata.lines[0].label, "");
    let binding = metadata.interaction_binding(&m.scene).unwrap();
    assert_eq!(binding.sites.len(), 1);
    assert_eq!(binding.sites[0].series, "three");
    assert_eq!(binding.sites[0].label, "three");
    assert_eq!(binding.sites[0].value, None);
    let model = frame(m.scene, metadata);
    assert_eq!(
        model
            .controls_scene()
            .marks
            .iter()
            .filter(|m| !m.label.is_empty())
            .count(),
        1
    );
    let mut noninteractive = map();
    let metadata = noninteractive
        .add_usa_lines(
            &source,
            &GeoStreamOptions {
                interactive: false,
                ..options()
            },
        )
        .unwrap();
    assert_eq!(metadata.lines[0].series, "three");
    assert_eq!(metadata.lines[0].source_series, "");
    assert!(
        metadata
            .interaction_binding(&noninteractive.scene)
            .unwrap()
            .sites
            .is_empty()
    );
}

#[test]
fn append_order_collisions_invalid_inputs_and_nonusa_context_reject_atomically() {
    let mut m = map();
    m.scene.marks.push(Mark::new(
        "background",
        Geometry::Rect {
            x: 0.,
            y: 0.,
            width: 960.,
            height: 560.,
        },
        Color(255, 255, 255, 255),
    ));
    m.scene.draw_order = vec![DrawItem::Mark(0)];
    let metadata = m
        .add_usa_lines(
            &source(),
            &GeoStreamOptions {
                behind: true,
                ..options()
            },
        )
        .unwrap();
    assert_eq!(
        m.scene.draw_items(),
        vec![
            DrawItem::Mark(1),
            DrawItem::Mark(2),
            DrawItem::Mark(3),
            DrawItem::Mark(0)
        ]
    );
    metadata.interaction_binding(&m.scene).unwrap();
    let before = m.clone();
    assert!(m.add_usa_lines(&source(), &options()).is_err());
    assert_eq!(m, before);
    for bad in [
        vec![source()[0].clone(), source()[0].clone()],
        vec![GeoLineFeature::new("antipodal", vec![[0., 0.], [180., 0.]])],
        vec![GeoLineFeature::new("invalid", vec![[0., 91.], [1., 0.]])],
    ] {
        let mut candidate = map();
        let old = candidate.clone();
        assert!(candidate.add_usa_lines(&bad, &options()).is_err());
        assert_eq!(candidate, old);
    }
    let mut plain = geographic_map_layout(&config(), &[], &Default::default()).unwrap();
    let old = plain.clone();
    assert!(plain.add_usa_lines(&source(), &options()).is_err());
    assert_eq!(plain, old);
    let mut wrong = config().scene().unwrap();
    wrong.width = 900.;
    let before = wrong.clone();
    assert!(
        map()
            .projection
            .add_usa_lines(&mut wrong, &source(), &options())
            .is_err()
    );
    assert_eq!(wrong, before);
}

#[test]
fn graticules_use_three_continuous_components_and_remain_noninteractive() {
    let mut m = map();
    let metadata = m
        .add_usa_graticule(&GeoGraticuleOptions::default(), &options())
        .unwrap();
    assert!(metadata.lines.len() > 30);
    let regions = metadata
        .lines
        .iter()
        .flat_map(|r| r.parts.iter().map(|p| p.region))
        .collect::<Vec<_>>();
    for region in GeoUsaRegion::ALL {
        assert!(regions.contains(&region));
    }
    let binding = metadata.interaction_binding(&m.scene).unwrap();
    assert!(binding.sites.is_empty());
    assert!(binding.ink_owners.is_empty());
    assert!(
        m.scene
            .marks
            .iter()
            .all(|mark| mark.label.is_empty() && mark.clip.is_some() && mark.clip_path.is_none())
    );
    let old = m.clone();
    assert!(m.add_graticule(&Default::default(), &options()).is_err());
    assert_eq!(m, old);
    assert!(m.add_lines(&source(), &options()).is_err());
    assert_eq!(m, old);
}

#[cfg(feature = "raster")]
#[test]
fn cpu_raster_renders_all_region_strokes_with_the_same_physical_masks() {
    use kartis::render::raster::{FontRegistry, RasterOptions, render_raster};
    let mut m = map();
    let metadata = m.add_usa_lines(&source(), &options()).unwrap();
    for dpr in [1., 2.] {
        let image = render_raster(
            &m.scene,
            &FontRegistry::new(),
            &RasterOptions {
                pixel_ratio: dpr,
                ..Default::default()
            },
        )
        .unwrap();
        for part in &metadata.lines[0].parts {
            let p = first(&part.geometry);
            let x = (p[0] * dpr).round() as usize;
            let y = (p[1] * dpr).round() as usize;
            let i = (y * image.width as usize + x) * 4;
            assert!(
                image.rgba[i] > 180 && image.rgba[i + 1] < 80 && image.rgba[i + 2] < 140,
                "real region stroke missing {:?}",
                part.region
            );
        }
    }
}
