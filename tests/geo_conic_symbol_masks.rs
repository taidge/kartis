use kartis::*;
#[test]
fn cumulative_masks_reserve_before_clone_without_charging_skipped_rows() {
    let map = geographic_map_layout(
        &ChartConfig::default(),
        &[],
        &GeoMapOptions {
            projection: GeoProjection::conic_equal_area_default(),
            scale: Some(1500.),
            ..Default::default()
        },
    )
    .unwrap();
    let options = GeoSymbolOptions {
        clip: false,
        clip_to_projection: true,
        radius_range: [5., 5.],
        domain: Some([0., 1.]),
        ..Default::default()
    };
    let many = (0..1000)
        .map(|i| GeoSymbol::new(format!("row-{i}"), [0., 0.], Some(1.)))
        .collect::<Vec<_>>();
    let before = map.scene.clone();
    let mut dense = before.clone();
    let error = map
        .projection
        .add_symbols(&mut dense, &many, &options)
        .unwrap_err();
    assert!(error.0.contains("conic overlay cumulative"));
    assert_eq!(dense, before);
    let sparse = many
        .into_iter()
        .enumerate()
        .map(|(i, mut s)| {
            if i != 0 {
                s.value = None;
            }
            s
        })
        .collect::<Vec<_>>();
    let mut accepted = before;
    map.projection
        .add_symbols(&mut accepted, &sparse, &options)
        .unwrap();
    assert_eq!(
        accepted
            .marks
            .iter()
            .filter(|m| m.id.starts_with("geo-symbol:"))
            .count(),
        1
    );
    accepted.validate().unwrap();
}
#[test]
fn conic_stroked_glyph_masks_preserve_raw_nullable_metadata_and_original_ink() {
    for projection in [
        GeoProjection::conic_equal_area_default(),
        GeoProjection::conic_equidistant_default(),
        GeoProjection::conic_conformal_default(),
        GeoProjection::Albers,
    ] {
        let base = geographic_map_layout(
            &ChartConfig::default(),
            &[],
            &GeoMapOptions {
                projection,
                ..Default::default()
            },
        )
        .unwrap();
        let point = if projection == GeoProjection::Albers {
            [83., 10.]
        } else {
            [179., 10.]
        };
        let source = [
            GeoSymbol::new("raw-seven", point, Some(7.)),
            GeoSymbol::new("raw-missing", [point[0] - 10., 20.], None),
        ];
        let options = GeoGlyphOptions {
            shape: GlyphShape::Filled(FilledGlyph::Star),
            symbols: GeoSymbolOptions {
                clip: false,
                labels: true,
                missing: GeoMissingSymbol::Draw {
                    radius: 6.,
                    color: Color(150, 140, 160, 255),
                    label: "missing".into(),
                },
                radius_range: [35., 35.],
                ..Default::default()
            },
            ..Default::default()
        };
        let mut before = base.clone();
        let original = before.add_glyphs(&source, &options).unwrap();
        let mut masked = base.clone();
        let clipped = masked
            .add_glyphs(
                &source,
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
        assert_eq!(original.sites, clipped.sites);
        assert_eq!(
            clipped
                .sites
                .iter()
                .find(|s| s.id == "raw-seven")
                .unwrap()
                .value,
            Some(7.)
        );
        assert_eq!(
            clipped
                .sites
                .iter()
                .find(|s| s.id == "raw-missing")
                .unwrap()
                .value,
            None
        );
        let mask = base
            .projection
            .projection_clip_path(0.25, 4096)
            .unwrap()
            .unwrap();
        for site in &clipped.sites {
            let a = before
                .scene
                .marks
                .iter()
                .find(|m| m.id == site.mark_id)
                .unwrap();
            let b = masked
                .scene
                .marks
                .iter()
                .find(|m| m.id == site.mark_id)
                .unwrap();
            assert_eq!(a.geometry, b.geometry);
            assert_eq!(a.stroke, b.stroke);
            assert_eq!(b.clip_path.as_deref(), Some(&mask));
        }
        clipped
            .interaction_binding(&masked.scene)
            .unwrap()
            .validate(&masked.scene)
            .unwrap();
    }
}
#[test]
fn conic_full_paint_masks_keep_geometry_and_intersect_plot_and_exact_picking() {
    for projection in [
        GeoProjection::conic_equal_area_default(),
        GeoProjection::conic_equidistant_default(),
        GeoProjection::conic_conformal_default(),
        GeoProjection::Albers,
    ] {
        let map = geographic_map_layout(
            &ChartConfig {
                width: 640.,
                height: 420.,
                ..Default::default()
            },
            &[],
            &GeoMapOptions {
                projection,
                ..Default::default()
            },
        )
        .unwrap();
        let context = map.projection;
        let footprint = context.projection_clip_path(0.25, 4096).unwrap().unwrap();
        assert!(context.projection_disk().is_none());
        for labels in [false, true] {
            for clip in [false, true] {
                let options = GeoSymbolOptions {
                    clip,
                    labels,
                    clip_to_projection: true,
                    clip_labels_to_projection: true,
                    radius_range: [100., 100.],
                    stroke: None,
                    ..Default::default()
                };
                let point = if projection == GeoProjection::Albers {
                    [83., 10.]
                } else {
                    [179., 10.]
                };
                let source = [GeoSymbol::new("seam", point, Some(1.))];
                let mut masked = map.scene.clone();
                let mut unmasked = masked.clone();
                context
                    .add_symbols(
                        &mut unmasked,
                        &source,
                        &GeoSymbolOptions {
                            clip_to_projection: false,
                            clip_labels_to_projection: false,
                            ..options.clone()
                        },
                    )
                    .unwrap();
                context.add_symbols(&mut masked, &source, &options).unwrap();
                let a = unmasked
                    .marks
                    .iter()
                    .find(|m| m.id.starts_with("geo-symbol:"))
                    .unwrap();
                let b = masked.marks.iter().find(|m| m.id == a.id).unwrap();
                assert_eq!(a.geometry, b.geometry);
                assert_eq!(b.clip_path.as_deref(), Some(&footprint));
                assert_eq!(b.clip, clip.then_some(context.plot()));
                if labels {
                    assert!(masked.label_clips.values().any(|c| c == &footprint));
                }
                let mut original_only = unmasked.clone();
                original_only.marks.retain(|m| m.id == a.id);
                original_only.draw_order.clear();
                let mut masked_only = masked.clone();
                masked_only.marks.retain(|m| m.id == a.id);
                masked_only.draw_order.clear();
                let p = context.project(point).unwrap().unwrap();
                let mut outside_witness = false;
                for dx in (-100..=100).step_by(4) {
                    for dy in (-100..=100).step_by(4) {
                        let q = [p[0] + dx as f64, p[1] + dy as f64];
                        if original_only.hit_test(q[0], q[1]).is_some()
                            && masked_only.hit_test(q[0], q[1]).is_none()
                        {
                            outside_witness = true;
                        }
                    }
                }
                assert!(
                    outside_witness,
                    "finite cone mask must actually remove original circle ink"
                );
                masked.validate().unwrap();
            }
        }
    }
}
#[test]
fn requested_invalid_or_budgeted_mask_does_not_change_source() {
    let map = geographic_map_layout(
        &ChartConfig::default(),
        &[],
        &GeoMapOptions {
            projection: GeoProjection::conic_equal_area_default(),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(map.projection.projection_clip_path(0.0001, 4).is_err());
    let old = map.scene.clone();
    assert!(map.projection.projection_clip_path(f64::NAN, 4096).is_err());
    assert_eq!(map.scene, old);
}
