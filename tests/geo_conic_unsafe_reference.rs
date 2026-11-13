use kartis::*;
const POLICIES: &str = include_str!("fixtures/geo-conic-unsafe/policies.csv");
#[test]
fn selected_family_rejects_unsafe_coefficients_without_rejecting_other_families() {
    let mut rejected = 0;
    let mut accepted = 0;
    let mut invalid = 0;
    for line in POLICIES.lines() {
        let c = line.split(',').collect::<Vec<_>>();
        let p = [c[2].parse::<f64>().unwrap(), c[3].parse::<f64>().unwrap()];
        let result = match c[1] {
            "equalArea" => GeoProjection::conic_equal_area(p),
            "equidistant" => GeoProjection::conic_equidistant(p),
            "conformal" => GeoProjection::conic_conformal(p),
            _ => panic!(),
        };
        if c[4] == "invalid-parallels" {
            assert!(GeoConicParallels::new(p).is_err(), "{line}");
            assert!(result.is_err(), "{line}");
            invalid += 1;
            continue;
        }
        let standards = GeoConicParallels::new(p)
            .expect("finite parallels are legal independently of selected-family coefficients");
        let direct = match c[1] {
            "equalArea" => GeoProjection::ConicEqualArea(standards),
            "equidistant" => GeoProjection::ConicEquidistant(standards),
            "conformal" => GeoProjection::ConicConformal(standards),
            _ => panic!(),
        };
        let options = GeoMapOptions {
            projection: direct,
            ..Default::default()
        };
        if c[4] == "selected-family-error" {
            assert!(result.is_err(), "checked constructor {line}");
            assert!(
                project_geo_point([20., 20.], &options).is_err(),
                "direct variant {line}"
            );
            assert!(
                geographic_map_layout(&ChartConfig::default(), &[], &options).is_err(),
                "World direct variant {line}"
            );
            rejected += 1;
        } else {
            assert!(result.is_ok(), "{line}: {result:?}");
            assert!(
                project_geo_point([20., 20.], &options).unwrap().is_some(),
                "{line}"
            );
            assert!(
                geographic_map_layout(&ChartConfig::default(), &[], &options).is_ok(),
                "{line}"
            );
            accepted += 1;
        }
    }
    assert_eq!((accepted, rejected, invalid), (34, 8, 9));
}
#[test]
fn same_unsafe_adjacent_pairs_keep_equal_area_available() {
    for p in [
        [-0.9585319170638371, -0.9585319170638373],
        [-53.13008026016052, -53.130080260160526],
    ] {
        let projection = GeoProjection::conic_equal_area(p).unwrap();
        let options = GeoMapOptions {
            projection,
            ..Default::default()
        };
        let map = geographic_map_layout(&ChartConfig::default(), &[], &options).unwrap();
        let source = [20., 20.];
        let pixel = map.projection.project(source).unwrap().unwrap();
        let inverse = map.projection.invert(pixel).unwrap().unwrap();
        let vector = |p: [f64; 2]| {
            let [l, f] = p.map(f64::to_radians);
            [f.cos() * l.cos(), f.cos() * l.sin(), f.sin()]
        };
        let (a, b) = (vector(source), vector(inverse));
        assert!((a[0] - b[0]).hypot(a[1] - b[1]).hypot(a[2] - b[2]) <= 1e-12);
    }
}
