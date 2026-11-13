use kartis::*;
use std::f64::consts::PI;
fn close(a: [f64; 2], b: [f64; 2]) {
    for (a, b) in a.into_iter().zip(b) {
        assert!((a - b).abs() <= 1e-12, "{a} != {b}");
    }
}
#[test]
fn centroid_dimensions_and_directionless_geometry() -> Result<()> {
    assert_eq!(geo_spherical_centroid(GeoSphericalGeometry::Empty)?, None);
    assert_eq!(geo_spherical_centroid(GeoSphericalGeometry::Sphere)?, None);
    close(
        geo_spherical_centroid(GeoSphericalGeometry::Point([30., 10.]))?.unwrap(),
        [30., 10.],
    );
    close(
        geo_spherical_centroid(GeoSphericalGeometry::LineString(&[[0., 0.], [90., 0.]]))?.unwrap(),
        [45., 0.],
    );
    assert_eq!(
        geo_spherical_centroid(GeoSphericalGeometry::MultiPoint(&[[0., 0.], [180., 0.]]))?,
        None
    );
    let ring = vec![[0., 0.], [0., 90.], [90., 0.], [0., 0.]];
    let expected = [45., (1_f64 / 3_f64.sqrt()).asin() * 180. / PI];
    close(
        geo_spherical_centroid(GeoSphericalGeometry::Polygon(std::slice::from_ref(&ring)))?
            .unwrap(),
        expected,
    );
    let pieces = [
        GeoSphericalGeometry::Sphere,
        GeoSphericalGeometry::Point([120., -30.]),
    ];
    close(
        geo_spherical_centroid(GeoSphericalGeometry::GeometryCollection(&pieces))?.unwrap(),
        [120., -30.],
    );
    let point_ring = vec![[30., 10.]; 4];
    close(
        geo_spherical_centroid(GeoSphericalGeometry::Polygon(&[point_ring]))?.unwrap(),
        [30., 10.],
    );
    Ok(())
}
#[test]
fn actual_spherical_contains_and_oriented_holes() -> Result<()> {
    let outer = vec![
        [-20., -20.],
        [-20., 20.],
        [20., 20.],
        [20., -20.],
        [-20., -20.],
    ];
    let hole = vec![[-5., -5.], [5., -5.], [5., 5.], [-5., 5.], [-5., -5.]];
    assert!(geo_spherical_contains(
        GeoSphericalGeometry::Polygon(std::slice::from_ref(&outer)),
        [0., 0.]
    )?);
    assert!(!geo_spherical_contains(
        GeoSphericalGeometry::Polygon(&[outer.clone(), hole]),
        [0., 0.]
    )?);
    assert!(geo_spherical_contains(
        GeoSphericalGeometry::Polygon(std::slice::from_ref(&outer)),
        [10., 0.]
    )?);
    assert!(!geo_spherical_contains(
        GeoSphericalGeometry::Polygon(std::slice::from_ref(&outer)),
        [90., 0.]
    )?);
    let reverse = outer.into_iter().rev().collect();
    assert!(!geo_spherical_contains(
        GeoSphericalGeometry::Polygon(&[reverse]),
        [0., 0.]
    )?);
    assert!(geo_spherical_contains(
        GeoSphericalGeometry::LineString(&[[0., 0.], [90., 0.]]),
        [45., 0.]
    )?);
    assert!(!geo_spherical_contains(
        GeoSphericalGeometry::LineString(&[[0., 0.], [90., 0.]]),
        [45., 1.]
    )?);
    assert!(!geo_spherical_contains(
        GeoSphericalGeometry::Empty,
        [0., 0.]
    )?);
    assert!(geo_spherical_contains(
        GeoSphericalGeometry::Sphere,
        [180., 90.]
    )?);
    // Original Point comparison is exactly zero angular distance, with no EPS.
    assert!(!geo_spherical_contains(
        GeoSphericalGeometry::Point([0., 0.]),
        [1e-12, 0.]
    )?);
    Ok(())
}
#[test]
fn checked_input_cannot_hide_behind_collection_short_circuit() {
    let invalid = GeoSphericalGeometry::Point([f64::NAN, 0.]);
    let collection = [GeoSphericalGeometry::Sphere, invalid];
    assert!(geo_spherical_centroid(GeoSphericalGeometry::GeometryCollection(&collection)).is_err());
    assert!(
        geo_spherical_contains(
            GeoSphericalGeometry::GeometryCollection(&collection),
            [0., 0.]
        )
        .is_err()
    );
    assert!(geo_spherical_contains(GeoSphericalGeometry::Sphere, [181., 0.]).is_err());
    for line in [
        &[[0., 0.], [180., 0.]][..],
        &[[0., 0.], [180. - 1e-9, 0.]][..],
    ] {
        assert!(geo_spherical_centroid(GeoSphericalGeometry::LineString(line)).is_err());
        assert!(geo_spherical_contains(GeoSphericalGeometry::LineString(line), [0., 0.]).is_err());
    }
    let safe = [[0., 0.], [179.999, 0.]];
    assert!(geo_spherical_centroid(GeoSphericalGeometry::LineString(&safe)).is_ok());
    let open = vec![[0., 0.], [0., 10.], [10., 0.]];
    let feature = GeoFeature {
        polygons: vec![vec![open]],
        id: "open".into(),
        label: "open".into(),
        value: None,
    };
    assert!(geo_spherical_centroid(GeoSphericalGeometry::from(&feature)).is_err());
    assert!(geo_spherical_contains(GeoSphericalGeometry::from(&feature), [1., 1.]).is_err());
    assert!(
        geo_spherical_centroid(GeoSphericalGeometry::Polygon(&[]))
            .unwrap()
            .is_none()
    );
    assert!(!geo_spherical_contains(GeoSphericalGeometry::Polygon(&[]), [0., 0.]).unwrap());
    assert!(geo_spherical_centroid(GeoSphericalGeometry::Polygon(&[vec![]])).is_err());
}
#[test]
fn aggregate_source_parts_and_depth_are_bounded_before_evaluation() -> Result<()> {
    let points = vec![[0., 0.]; 100_000];
    assert!(geo_spherical_contains(
        GeoSphericalGeometry::MultiPoint(&points),
        [0., 0.]
    )?);
    let too_many = vec![[0., 0.]; 100_001];
    assert!(geo_spherical_centroid(GeoSphericalGeometry::MultiPoint(&too_many)).is_err());
    let pieces = [
        GeoSphericalGeometry::Sphere,
        GeoSphericalGeometry::MultiPoint(&too_many),
    ];
    assert!(
        geo_spherical_contains(GeoSphericalGeometry::GeometryCollection(&pieces), [0., 0.])
            .is_err()
    );
    let parts = vec![GeoSphericalGeometry::Sphere; 4095];
    assert!(geo_spherical_centroid(GeoSphericalGeometry::GeometryCollection(&parts)).is_ok());
    let parts = vec![GeoSphericalGeometry::Sphere; 4096];
    assert!(
        geo_spherical_contains(GeoSphericalGeometry::GeometryCollection(&parts), [0., 0.]).is_err()
    );
    fn nested(level: usize) -> Result<()> {
        fn go(level: usize, g: GeoSphericalGeometry<'_>) -> Result<()> {
            if level == 0 {
                geo_spherical_centroid(g)?;
                geo_spherical_contains(g, [0., 0.])?;
                Ok(())
            } else {
                let parts = [g];
                go(level - 1, GeoSphericalGeometry::GeometryCollection(&parts))
            }
        }
        go(level, GeoSphericalGeometry::Point([0., 0.]))
    }
    assert!(nested(32).is_ok());
    assert!(nested(33).is_err());
    Ok(())
}

#[test]
fn generated_circle_feature_uses_both_relations() -> Result<()> {
    let center = [15., 25.];
    let ring = geo_circle(&GeoCircleOptions {
        center,
        radius_degrees: 25.,
        precision_degrees: 3.,
        ..Default::default()
    })?;
    let feature = GeoFeature {
        id: "circle".into(),
        label: "circle".into(),
        value: None,
        polygons: vec![vec![ring]],
    };
    let g = GeoSphericalGeometry::from(&feature);
    close(geo_spherical_centroid(g)?.unwrap(), center);
    assert!(geo_spherical_contains(g, center)?);
    assert!(!geo_spherical_contains(g, [150., -25.])?);
    Ok(())
}
