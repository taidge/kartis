use kartis::*;

fn close(a: [f64; 2], b: [f64; 2], tolerance: f64) {
    assert!(
        (a[0] - b[0]).abs().max((a[1] - b[1]).abs()) <= tolerance,
        "{a:?} != {b:?}"
    );
}

#[test]
fn fixed_cities_match_original_d3_geo_3_1_1_at_multiple_affine_frames() {
    // Original geoAlbersUsa() output, at scale1070/translate[480,250].
    let cities = [
        (
            [-122.3321, 47.6062],
            [159.10285740989343, 37.08535798159676],
            GeoUsaRegion::Lower48,
        ),
        (
            [-122.4194, 37.7749],
            [107.42689983179525, 214.14309852394865],
            GeoUsaRegion::Lower48,
        ),
        (
            [-74.006, 40.7128],
            [794.5954450548626, 176.53258338778596],
            GeoUsaRegion::Lower48,
        ),
        (
            [-149.9, 61.2],
            [171.17122462409534, 447.062592258631],
            GeoUsaRegion::Alaska,
        ),
        (
            [-134.4197, 58.3019],
            [224.38278025854416, 456.6612701495211],
            GeoUsaRegion::Alaska,
        ),
        (
            [-157.8583, 21.3069],
            [298.4785715711044, 450.98746080412104],
            GeoUsaRegion::Hawaii,
        ),
    ];
    for (scale, translate) in [
        (1070., [480., 250.]),
        (80., [200., 120.]),
        (100_000., [0., 0.]),
        (1_000_000., [0., 0.]),
    ] {
        let p = GeoAlbersUsaProjection::new(scale, translate).unwrap();
        assert_eq!(p.scale(), scale);
        assert_eq!(p.translate(), translate);
        for (source, pixel, region) in cities {
            let expected = [
                translate[0] + (pixel[0] - 480.) * scale / 1070.,
                translate[1] + (pixel[1] - 250.) * scale / 1070.,
            ];
            let actual = p.project(source).unwrap().unwrap();
            assert_eq!(actual.region, region);
            close(actual.position, expected, 1e-7);
            close(
                p.project_in_region(region, source).unwrap().unwrap(),
                expected,
                1e-7,
            );
            let inverse = p.invert(actual.position).unwrap().unwrap();
            assert_eq!(inverse.region, region);
            close(inverse.position, source, 1e-8);
        }
    }
}

#[test]
fn clip_epsilon_stays_one_micro_pixel_at_each_scale_and_inset_identity_is_explicit() {
    for scale in [1., 1070., 100_000.] {
        let t = [480., 250.];
        let p = GeoAlbersUsaProjection::new(scale, t).unwrap();
        let ak = p.region_clip(GeoUsaRegion::Alaska);
        let hi = p.region_clip(GeoUsaRegion::Hawaii);
        close(
            [ak.x, ak.y],
            [t[0] - 0.425 * scale + 1e-6, t[1] + 0.120 * scale + 1e-6],
            1e-12,
        );
        close(
            [ak.x + ak.width, ak.y + ak.height],
            [t[0] - 0.214 * scale - 1e-6, t[1] + 0.234 * scale - 1e-6],
            1e-10,
        );
        close(
            [hi.x, hi.y],
            [t[0] - 0.214 * scale + 1e-6, t[1] + 0.166 * scale + 1e-6],
            1e-12,
        );
        assert!(
            p.project_in_region(GeoUsaRegion::Hawaii, [-74.006, 40.7128])
                .unwrap()
                .is_none()
        );
        for region in GeoUsaRegion::ALL {
            let r = p.region_clip(region);
            let center = [r.x + r.width / 2., r.y + r.height / 2.];
            let source = p.invert_in_region(region, center).unwrap().unwrap();
            close(
                p.project_in_region(region, source).unwrap().unwrap(),
                center,
                1e-7,
            );
            assert!(
                p.invert_in_region(region, [r.x - 1e-5, r.y])
                    .unwrap()
                    .is_none()
            );
        }
    }
}

#[test]
fn default_inverse_preserves_original_inset_ambiguity_and_region_inverse_recovers_source() {
    let p = GeoAlbersUsaProjection::default();
    let source = [-122., 22.];
    let projected = p.project(source).unwrap().unwrap();
    assert_eq!(projected.region, GeoUsaRegion::Lower48);
    close(
        projected.position,
        [33.85958154790899, 499.0204851128873],
        1e-8,
    );
    let inverse = p.invert(projected.position).unwrap().unwrap();
    assert_eq!(inverse.region, GeoUsaRegion::Alaska);
    close(
        inverse.position,
        [176.34147044911975, 48.973552328834856],
        1e-8,
    );
    close(
        p.invert_in_region(projected.region, projected.position)
            .unwrap()
            .unwrap(),
        source,
        1e-8,
    );
    close(
        p.project_in_region(inverse.region, inverse.position)
            .unwrap()
            .unwrap(),
        projected.position,
        1e-8,
    );
}

#[test]
fn visible_footprints_reject_foreign_points_and_off_frame_inverse_without_fallback() {
    let p = GeoAlbersUsaProjection::default();
    for source in [[0., 0.], [139.69, 35.68], [-66.1, 18.4]] {
        assert!(p.project(source).unwrap().is_none());
    }
    assert!(p.invert([10_000., 10_000.]).unwrap().is_none());
    for region in GeoUsaRegion::ALL {
        assert!(
            p.invert_in_region(region, [10_000., 10_000.])
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn collapsed_insets_and_nonfinite_or_out_of_budget_frames_and_queries_reject() {
    for scale in [
        0.,
        -1.,
        f64::NAN,
        f64::INFINITY,
        1_000_001.,
        1e-6,
        f64::MIN_POSITIVE,
    ] {
        assert!(GeoAlbersUsaProjection::new(scale, [0., 0.]).is_err());
    }
    for t in [
        [f64::NAN, 0.],
        [0., f64::INFINITY],
        [1_000_001., 0.],
        [1_000_000., 0.],
    ] {
        assert!(GeoAlbersUsaProjection::new(1070., t).is_err());
    }
    GeoAlbersUsaProjection::new(0.001, [0., 0.]).unwrap();
    let p = GeoAlbersUsaProjection::default();
    for source in [[181., 0.], [0., 91.], [f64::NAN, 0.], [0., f64::INFINITY]] {
        assert!(p.project(source).is_err());
    }
    for pixel in [[f64::NAN, 0.], [0., f64::INFINITY], [1_000_001., 0.]] {
        assert!(p.invert(pixel).is_err());
    }
}

#[test]
fn inverse_domain_uses_the_requested_pixel_at_exact_edges_not_secondary_forward_visibility() {
    let p = GeoAlbersUsaProjection::new(150., [0., 0.]).unwrap();
    let pixel = [-68.25_f64, 35.699999999999996];
    let source = p
        .invert_in_region(GeoUsaRegion::Lower48, pixel)
        .unwrap()
        .unwrap();
    close(source, [-124.10598297359105, 21.075639301619752], 1e-8);
    assert_eq!(
        p.invert(pixel).unwrap().unwrap().region,
        GeoUsaRegion::Lower48
    );
    // The recovered finite representative may map one ulp beyond the strict
    // forward rectangle on a host's math library. When visible, it still maps
    // back to the original pixel; absence does not invalidate the input domain.
    if let Some(again) = p.project_in_region(GeoUsaRegion::Lower48, source).unwrap() {
        close(again, pixel, 1e-7);
    }
    assert!(
        p.invert_in_region(GeoUsaRegion::Lower48, [pixel[0].next_down(), pixel[1]])
            .unwrap()
            .is_none()
    );
}

#[cfg(feature = "geojson")]
#[test]
fn complete_original_d3_reference_checks_regions_visibility_and_strict_inverse_profiles() {
    use serde_json::Value;
    fn number(v: &Value) -> f64 {
        v.as_f64().unwrap()
    }
    fn point(v: &Value) -> [f64; 2] {
        [number(&v[0]), number(&v[1])]
    }
    fn region(v: &Value) -> GeoUsaRegion {
        match v.as_str().unwrap() {
            "Lower48" => GeoUsaRegion::Lower48,
            "Alaska" => GeoUsaRegion::Alaska,
            "Hawaii" => GeoUsaRegion::Hawaii,
            other => panic!("unknown reference region {other}"),
        }
    }
    fn optional(actual: Option<[f64; 2]>, expected: &Value, context: &str) {
        match (actual, expected.is_null()) {
            (None, true) => {}
            (Some(actual), false) => close(actual, point(expected), 1e-7),
            (actual, _) => panic!("{context}: {actual:?} vs {expected}"),
        }
    }
    fn optional_inverse(
        actual: Option<[f64; 2]>,
        expected: &Value,
        context: &str,
        errors: &mut Vec<String>,
    ) {
        match (actual, expected.is_null()) {
            (Some(actual), false) => {
                let expected = point(expected);
                let longitude = (actual[0] - expected[0] + 180.).rem_euclid(360.) - 180.;
                if longitude.abs().max((actual[1] - expected[1]).abs()) > 1e-7 {
                    errors.push(format!("{context}: {actual:?} vs {expected:?}"));
                }
            }
            (None, true) => {}
            (actual, _) => errors.push(format!("{context}: {actual:?} vs {expected}")),
        }
    }
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/geo-usa-kernel/values.json")).unwrap();
    let mut counts = [0; 3];
    let mut errors = Vec::new();
    for case in fixture["cases"].as_array().unwrap() {
        let p =
            GeoAlbersUsaProjection::new(number(&case["scale"]), point(&case["translate"])).unwrap();
        for clip in case["clips"].as_array().unwrap() {
            let r = p.region_clip(region(&clip["region"]));
            close([r.x, r.y], point(&clip["extent"][0]), 1e-8);
            close(
                [r.x + r.width, r.y + r.height],
                point(&clip["extent"][1]),
                1e-8,
            );
        }
        for query in case["points"].as_array().unwrap() {
            let input = point(&query["input"]);
            let actual = p.project(input).unwrap();
            optional(
                actual.map(|p| p.position),
                &query["output"],
                &format!("project {input:?} scale {}", p.scale()),
            );
            if let Some(actual) = actual {
                assert_eq!(actual.region, region(&query["region"]));
            }
            for r in query["regions"].as_array().unwrap() {
                optional(
                    p.project_in_region(region(&r["region"]), input).unwrap(),
                    &r["output"],
                    &format!("project region {input:?}"),
                );
            }
            counts[0] += 1;
        }
        for query in case["inverses"].as_array().unwrap() {
            let input = point(&query["input"]);
            let actual = p.invert(input).unwrap();
            optional_inverse(
                actual.map(|p| p.position),
                &query["strict"],
                &format!("inverse {input:?} scale {}", p.scale()),
                &mut errors,
            );
            if let Some(actual) = actual {
                assert_eq!(actual.region, region(&query["region"]));
            }
            counts[1] += 1;
        }
        for query in case["regionInverses"].as_array().unwrap() {
            let input = point(&query["input"]);
            optional_inverse(
                p.invert_in_region(region(&query["region"]), input).unwrap(),
                &query["output"],
                &format!("inverse region {input:?} scale {}", p.scale()),
                &mut errors,
            );
            counts[2] += 1;
        }
    }
    assert_eq!(counts, [5778, 792, 414]);
    assert!(
        errors.is_empty(),
        "{} inverse differences: {:#?}",
        errors.len(),
        &errors[..errors.len().min(40)]
    );
}
