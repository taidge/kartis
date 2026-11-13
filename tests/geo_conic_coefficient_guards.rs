use kartis::*;
// Frozen original d3-geo 3.1.1 diagnostics are retained separately at
// target/conic-source-independent-review/original-native-counterexample.json.
const BAD_EQ: [f64; 2] = [-0.9585319170638371, -0.9585319170638373];
const BAD_CC: [f64; 2] = [-53.13008026016052, -53.130080260160526];
#[test]
fn adjacent_standard_quotients_are_checked_per_selected_family() {
    for pair in [BAD_EQ, [BAD_EQ[1], BAD_EQ[0]]] {
        let p = GeoConicParallels::new(pair)
            .expect("finite author standards are still valid parameters");
        assert_eq!(p.degrees(), pair);
        assert!(
            GeoProjection::conic_equidistant(pair)
                .unwrap_err()
                .to_string()
                .contains("coefficient n")
        );
        assert!(GeoProjection::conic_conformal(pair).is_err());
        let ea = GeoProjection::conic_equal_area(pair).unwrap();
        let map = geographic_map_layout(
            &ChartConfig::default(),
            &[],
            &GeoMapOptions {
                projection: ea,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(map.projection.project([0., 0.]).unwrap().is_some());
        assert!(
            map.projection
                .projection_clip_path(0.25, 4096)
                .unwrap()
                .is_some()
        );
    }
    for pair in [BAD_CC, [BAD_CC[1], BAD_CC[0]]] {
        assert!(
            GeoProjection::conic_conformal(pair)
                .unwrap_err()
                .to_string()
                .contains("coefficient n")
        );
        assert!(GeoProjection::conic_equal_area(pair).is_ok());
        // Adjacent cosine differences depend on libm rounding. The same
        // unsafe standards can violate the coefficient bound/interval before
        // the signed radial-origin guard is reached; either must reject.
        let error = GeoProjection::conic_equidistant(pair).unwrap_err();
        assert!(
            error.0.contains("coefficient n") || error.0.contains("radial origin"),
            "{pair:?}: {error}"
        );
    }
    for a in [BAD_EQ[0], BAD_CC[0]] {
        for pair in [[a, a], [a, a + 1e-5], [a + 1e-5, a]] {
            assert!(GeoProjection::conic_equal_area(pair).is_ok());
            assert!(GeoProjection::conic_equidistant(pair).is_ok());
            assert!(GeoProjection::conic_conformal(pair).is_ok());
        }
    }
}
#[test]
fn direct_variants_cannot_bypass_coefficient_checks_at_world_data_or_stream_entry() {
    let p = GeoConicParallels::new(BAD_EQ).unwrap();
    let feature = GeoFeature {
        id: "land".into(),
        label: "land".into(),
        value: None,
        polygons: vec![vec![vec![
            [-10., -10.],
            [10., -10.],
            [10., 10.],
            [-10., 10.],
            [-10., -10.],
        ]]],
    };
    for projection in [
        GeoProjection::ConicEquidistant(p),
        GeoProjection::ConicConformal(p),
        GeoProjection::ConicConformal(GeoConicParallels::new(BAD_CC).unwrap()),
        GeoProjection::ConicEquidistant(GeoConicParallels::new(BAD_CC).unwrap()),
    ] {
        let expected = match projection {
            GeoProjection::ConicEquidistant(p) => {
                GeoProjection::conic_equidistant(p.degrees()).unwrap_err()
            }
            GeoProjection::ConicConformal(p) => {
                GeoProjection::conic_conformal(p.degrees()).unwrap_err()
            }
            _ => unreachable!(),
        };
        for fit in [GeoFit::World, GeoFit::Data] {
            let view = GeoMapOptions {
                projection,
                fit,
                ..Default::default()
            };
            assert_eq!(project_geo_point([0., 0.], &view).unwrap_err(), expected);
            assert_eq!(
                geographic_map_layout(
                    &ChartConfig::default(),
                    std::slice::from_ref(&feature),
                    &view
                )
                .unwrap_err(),
                expected
            );
            assert_eq!(
                geographic_map_layout_with_path_options(
                    &ChartConfig::default(),
                    std::slice::from_ref(&feature),
                    &view,
                    &GeoPathOptions::default()
                )
                .unwrap_err(),
                expected
            );
            assert_eq!(
                geographic_lines(
                    &ChartConfig::default(),
                    &[GeoLineFeature::new("line", vec![[-10., 0.], [10., 0.]])],
                    &view,
                    &GeoStreamOptions::default()
                )
                .unwrap_err(),
                expected
            );
        }
    }
}
#[test]
fn valid_family_preserves_existing_low_work_failure_and_scene_transaction() {
    let p = GeoProjection::conic_equal_area(BAD_EQ).unwrap();
    let view = GeoMapOptions {
        projection: p,
        fit: GeoFit::Data,
        ..Default::default()
    };
    let low = GeoStreamOptions {
        path: Some(GeoPathOptions {
            max_work: 3,
            ..Default::default()
        }),
        ..Default::default()
    };
    assert!(
        geographic_lines(
            &ChartConfig::default(),
            &[GeoLineFeature::new("arc", vec![[-80., 25.], [20., 55.]])],
            &view,
            &low
        )
        .unwrap_err()
        .to_string()
        .contains("work budget")
    );
    let mut map = geographic_map_layout(
        &ChartConfig::default(),
        &[],
        &GeoMapOptions {
            projection: p,
            ..Default::default()
        },
    )
    .unwrap();
    let old = map.scene.clone();
    assert!(
        map.add_lines(
            &[GeoLineFeature::new("bad", vec![[-80., 25.], [20., 55.]])],
            &low
        )
        .is_err()
    );
    assert_eq!(old, map.scene);
}
