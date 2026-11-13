use kartis::*;
use std::f64::consts::{FRAC_PI_2, PI};
#[test]
fn angular_distance_and_oriented_area_have_known_units() {
    assert!((geo_spherical_distance([0., 0.], [90., 0.]).unwrap() - FRAC_PI_2).abs() < 1e-12);
    assert!((geo_spherical_distance([0., 0.], [180., 0.]).unwrap() - PI).abs() < 1e-12);
    assert_eq!(
        geo_spherical_length(GeoSphericalGeometry::Sphere).unwrap(),
        0.
    );
    assert_eq!(
        geo_spherical_area(GeoSphericalGeometry::Sphere).unwrap(),
        4. * PI
    );
    let repeated = [GeoSphericalGeometry::Sphere, GeoSphericalGeometry::Sphere];
    assert_eq!(
        geo_spherical_area(GeoSphericalGeometry::GeometryCollection(&repeated)).unwrap(),
        8. * PI
    );
    let ring = vec![[0., 0.], [0., 90.], [90., 0.], [0., 0.]];
    let rings = vec![ring.clone()];
    let mut reverse = ring.clone();
    reverse.reverse();
    assert!(
        (geo_spherical_area(GeoSphericalGeometry::Polygon(&rings)).unwrap() - FRAC_PI_2).abs()
            < 1e-12
    );
    assert!(
        (geo_spherical_area(GeoSphericalGeometry::Polygon(&[reverse])).unwrap()
            - (4. * PI - FRAC_PI_2))
            .abs()
            < 1e-12
    );
    assert!(
        (geo_spherical_length(GeoSphericalGeometry::Polygon(&rings)).unwrap() - PI).abs() < 1e-12
    );
    assert!(
        (geo_spherical_length(GeoSphericalGeometry::LineString(&ring)).unwrap() - 3. * FRAC_PI_2)
            .abs()
            < 1e-12
    );
}
#[test]
fn invalid_domains_and_shared_part_point_budgets_are_errors() {
    for p in [[181., 0.], [0., 91.], [f64::NAN, 0.], [0., f64::INFINITY]] {
        assert!(geo_spherical_distance(p, [0., 0.]).is_err());
        assert!(geo_spherical_area(GeoSphericalGeometry::Point(p)).is_err());
    }
    let oversized = vec![[0., 0.]; 100001];
    assert!(
        geo_spherical_length(GeoSphericalGeometry::LineString(&oversized))
            .unwrap_err()
            .to_string()
            .contains("point budget")
    );
    let too_many = vec![Vec::new(); 4097];
    assert!(
        geo_spherical_length(GeoSphericalGeometry::MultiLineString(&too_many))
            .unwrap_err()
            .to_string()
            .contains("part budget")
    );
    let open = vec![vec![[0., 0.], [1., 0.], [1., 1.]]];
    assert!(geo_spherical_area(GeoSphericalGeometry::Polygon(&open)).is_err());
    let antipodal = vec![vec![[0., 0.], [180., 0.], [90., 20.], [0., 0.]]];
    assert!(
        geo_spherical_area(GeoSphericalGeometry::Polygon(&antipodal))
            .unwrap_err()
            .to_string()
            .contains("antipodal")
    );
    assert!(geo_spherical_length(GeoSphericalGeometry::Polygon(&antipodal)).is_ok());
}
#[test]
fn closed_circle_is_compatible_with_existing_feature_and_geometry_consumer() {
    let ring = geo_circle(&GeoCircleOptions {
        radius_degrees: 30.,
        ..Default::default()
    })
    .unwrap();
    assert_eq!(
        ring.first().unwrap().map(f64::to_bits),
        ring.last().unwrap().map(f64::to_bits)
    );
    for p in &ring {
        assert!((geo_spherical_distance([0., 0.], *p).unwrap() - 30f64.to_radians()).abs() < 1e-12);
    }
    let feature = GeoFeature {
        id: "circle".into(),
        label: "circle".into(),
        value: None,
        polygons: vec![vec![ring]],
    };
    let area = geo_spherical_area(GeoSphericalGeometry::from(&feature)).unwrap();
    assert!(area > 0. && area < 4. * PI);
    let map = geographic_map_layout(
        &ChartConfig::default(),
        &[feature],
        &GeoMapOptions {
            projection: GeoProjection::Orthographic,
            ..Default::default()
        },
    )
    .unwrap();
    map.scene.validate().unwrap();
}
#[test]
fn circle_precision_budget_and_degenerate_policies_are_explicit() {
    for precision in [0., -1., 91., f64::NAN, f64::INFINITY, 1e-300] {
        assert!(
            geo_circle(&GeoCircleOptions {
                precision_degrees: precision,
                ..Default::default()
            })
            .is_err()
        );
    }
    for radius in [-1., 181., f64::NAN] {
        assert!(
            geo_circle(&GeoCircleOptions {
                radius_degrees: radius,
                ..Default::default()
            })
            .is_err()
        );
    }
    assert!(
        geo_circle(&GeoCircleOptions {
            max_points: 4,
            ..Default::default()
        })
        .is_err()
    );
    for radius in [0., 180.] {
        let ring = geo_circle(&GeoCircleOptions {
            radius_degrees: radius,
            ..Default::default()
        })
        .unwrap();
        assert!(ring.len() >= 4);
        assert_eq!(ring.first(), ring.last());
    }
    let o = GeoCircleOptions {
        precision_degrees: 7.,
        ..Default::default()
    };
    let ring = geo_circle(&o).unwrap();
    assert!(
        geo_circle(&GeoCircleOptions {
            max_points: ring.len() - 1,
            ..o.clone()
        })
        .is_err()
    );
    assert_eq!(
        geo_circle(&GeoCircleOptions {
            max_points: ring.len(),
            ..o
        })
        .unwrap(),
        ring
    );
}

fn nested(depth: usize, f: &dyn Fn(GeoSphericalGeometry<'_>) -> Result<f64>) -> Result<f64> {
    if depth == 0 {
        f(GeoSphericalGeometry::Empty)
    } else {
        nested(depth - 1, &|g| {
            f(GeoSphericalGeometry::GeometryCollection(
                std::slice::from_ref(&g),
            ))
        })
    }
}
#[test]
fn recursive_geometry_and_feature_open_ring_boundaries_are_explicit() {
    assert_eq!(nested(32, &geo_spherical_area).unwrap(), 0.);
    assert!(
        nested(33, &geo_spherical_area)
            .unwrap_err()
            .to_string()
            .contains("nesting budget")
    );
    let feature = GeoFeature {
        id: "open".into(),
        label: "open".into(),
        value: None,
        polygons: vec![vec![vec![[0., 0.], [0., 10.], [10., 0.]]]],
    };
    assert!(
        geo_spherical_area(GeoSphericalGeometry::from(&feature))
            .unwrap_err()
            .to_string()
            .contains("exact closure")
    );
    assert!(
        geographic_map_layout(
            &ChartConfig::default(),
            std::slice::from_ref(&feature),
            &GeoMapOptions::default()
        )
        .is_ok()
    );
}
