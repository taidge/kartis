use kartis::*;

#[cfg(feature = "geojson")]
#[test]
fn near_right_angle_edges_preserve_fold_without_asin_singularity() {
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/geo-spherical-stability/witnesses.json"
    ))
    .unwrap();
    let mut count = 0;
    for c in data["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["id"].is_string())
    {
        let rings: Vec<Vec<[f64; 2]>> = c["geometry"]["coordinates"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| {
                r.as_array()
                    .unwrap()
                    .iter()
                    .map(|p| [p[0].as_f64().unwrap(), p[1].as_f64().unwrap()])
                    .collect()
            })
            .collect();
        let actual = geo_spherical_centroid(GeoSphericalGeometry::Polygon(&rings))
            .unwrap()
            .unwrap();
        let expected: Vec<f64> = c["highPrecision"]["oracleCentroid"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap().parse().unwrap())
            .collect();
        for (a, b) in actual.into_iter().zip(expected) {
            assert!((a - b).abs() <= 1e-12, "{} {a} != {b}", c["id"]);
        }
        count += 1;
    }
    assert_eq!(count, 2);
}

#[test]
fn small_ring_area_moment_avoids_global_cartesian_cancellation() {
    // Independent80/120-digit vector integral over exact input binary f64s.
    // The old native and original JS answers are both wrong by millidegrees.
    let ring = vec![
        [-39.53941720537841, 10.451751030050218],
        [-39.53941720537841, 10.451796039474646],
        [-39.53937219595399, 10.451796039474646],
        [-39.53937219595399, 10.451751030050218],
        [-39.53941720537841, 10.451751030050218],
    ];
    let g = GeoSphericalGeometry::Polygon(std::slice::from_ref(&ring));
    let actual = geo_spherical_centroid(g).unwrap().unwrap();
    let expected: [f64; 2] = [
        "-39.539394700666200321848009480163455009".parse().unwrap(),
        "10.451773534762676807694451398015163475".parse().unwrap(),
    ];
    for (a, b) in actual.into_iter().zip(expected) {
        assert!((a - b).abs() <= 1e-12, "{a} != {b}");
    }
}

#[test]
fn tiny_ring_center_and_outside_use_stable_meridian_crossing() {
    // All four exact high-precision edge-side products have the same strict
    // sign at the center. This checks interior/outside, not boundary convention.
    let ring = vec![
        [145.5424987245351, -47.36207062844187],
        [145.5424987245351, -47.36207057707666],
        [145.54249877590033, -47.36207057707666],
        [145.54249877590033, -47.36207062844187],
        [145.5424987245351, -47.36207062844187],
    ];
    let g = GeoSphericalGeometry::Polygon(std::slice::from_ref(&ring));
    assert!(geo_spherical_contains(g, [145.5424987502177, -47.36207060275927]).unwrap());
    assert!(!geo_spherical_contains(g, [145.5424986731699, -47.36207067980708]).unwrap());
}

#[test]
fn original_small_area_fallback_and_exact_source_boundary_remain_explicit() {
    let ring = vec![
        [-25.791670032776892, 42.58757248520851],
        [-25.791670032776892, 42.587572669357364],
        [-25.791669848628036, 42.587572669357364],
        [-25.791669848628036, 42.58757248520851],
        [-25.791670032776892, 42.58757248520851],
    ];
    let g = GeoSphericalGeometry::Polygon(std::slice::from_ref(&ring));
    let actual = geo_spherical_centroid(g).unwrap().unwrap();
    for (a, b) in actual
        .into_iter()
        .zip([-25.791669959117346, 42.58757255886806])
    {
        assert!((a - b).abs() <= 1e-12);
    }
    assert!(!geo_spherical_contains(g, ring[0]).unwrap());
    assert!(geo_spherical_contains(g, [-25.791669940702466, 42.58757257728294]).unwrap());
}
