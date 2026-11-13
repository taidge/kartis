use std::collections::{BTreeMap, BTreeSet};

use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::*;
fn config() -> ChartConfig {
    ChartConfig {
        width: 200.,
        height: 200.,
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 20.,
            bottom: 20.,
        },
        ..Default::default()
    }
}
fn axes() -> Vec<String> {
    ["north", "east", "south", "west"]
        .map(str::to_string)
        .to_vec()
}
fn data() -> Vec<RadarSeries> {
    vec![RadarSeries::new("alpha", vec![1., 0.5, 0.75, 0.])]
}
fn options() -> RadarLayoutOptions {
    RadarLayoutOptions {
        domain: Some([0., 1.]),
        radius_ratio: 0.8,
        ..Default::default()
    }
}
fn series(scene: &Scene) -> &Mark {
    scene
        .marks
        .iter()
        .find(|m| m.id == "radar-series:5:alpha")
        .unwrap()
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}")
}
#[test]
fn closed_radial_curves_have_real_periodic_controls_not_an_open_spline_chord() {
    let mut o = options();
    o.curve = Curve::BasisClosed;
    let s = radar_with_options(&config(), &axes(), &data(), &o).unwrap();
    let Geometry::Path(path) = &series(&s).geometry else {
        panic!()
    };
    // Independent D3 basisClosed recurrence for radial p0=(100,36),
    // p1=(132,100),p2=(100,148),p3=(100,100).
    let C::Move(start) = path[0] else { panic!() };
    near(start.x, (100. + 4. * 132. + 100.) / 6.);
    near(start.y, (36. + 4. * 100. + 148.) / 6.);
    assert_eq!(
        path.iter().filter(|c| matches!(c, C::Cubic { .. })).count(),
        4
    );
    let C::Cubic { first, second, end } = path[1] else {
        panic!()
    };
    near(first.x, (2. * 132. + 100.) / 3.);
    near(second.y, (100. + 2. * 148.) / 3.);
    near(end.x, (132. + 4. * 100. + 100.) / 6.);
    let C::Cubic { end, .. } = path[4] else {
        panic!()
    };
    near(end.x, start.x);
    near(end.y, start.y);
    assert!(matches!(path[5], C::Close));
    for curve in [
        Curve::LinearClosed,
        Curve::CardinalClosed,
        Curve::cardinal_closed(0.75),
        Curve::CatmullRomClosed { alpha: 0.5 },
    ] {
        o.curve = curve;
        radar_with_options(&config(), &axes(), &data(), &o)
            .unwrap()
            .validate()
            .unwrap();
    }
    o.curve = Curve::Basis;
    assert!(radar_with_options(&config(), &axes(), &data(), &o).is_err());
}
#[test]
fn grid_spokes_format_and_point_labels_have_correct_visibility_ownership() {
    let mut o = options();
    o.grid.shape = RadarGridShape::Polygon;
    o.grid.values = Some(vec![0.25, 0.5, 1.]);
    o.grid.tick_labels = true;
    o.grid.tick_format = ValueFormat::Percent(0);
    o.points.labels = true;
    o.points.value_format = ValueFormat::Fixed(1);
    o.axis_labels_by_id
        .insert("north".into(), "North <safe>".into());
    o.axis_label_style.size = Some(18.);
    o.axis_label_style.rotation = 12.;
    let mut d = data();
    d.push(RadarSeries::new("beta", vec![0.5; 4]));
    let s = radar_with_options(&config(), &axes(), &d, &o).unwrap();
    assert_eq!(
        s.marks
            .iter()
            .filter(|m| m.id.starts_with("radar-grid:"))
            .count(),
        3
    );
    let spokes = s
        .marks
        .iter()
        .filter(|m| m.id.starts_with("radar-spoke:"))
        .collect::<Vec<_>>();
    assert_eq!(spokes.len(), 4);
    assert!(
        spokes
            .iter()
            .all(|m| m.label.is_empty() && m.series.is_empty())
    );
    assert!(s.labels.iter().any(|l| l.text == "25%"));
    assert!(
        s.labels
            .iter()
            .any(|l| l.text == "North <safe>" && l.size == 18. && l.rotation == 12.)
    );
    assert_eq!(s.label_series.len(), 8);
    let filtered = s.without_series(&BTreeSet::from(["alpha".into()]));
    assert!(!filtered.marks.iter().any(|m| m.series == "alpha"));
    assert_eq!(filtered.label_series.len(), 4);
    assert!(filtered.labels.iter().any(|l| l.text == "North <safe>"));
    o.hidden.insert("alpha".into());
    let direct = radar_with_options(&config(), &axes(), &d, &o).unwrap();
    assert_eq!(direct.labels, filtered.labels);
    assert_eq!(direct.marks, filtered.marks);
    assert_eq!(direct.legend.len(), 2);
    let svg = kartis::render::to_svg(&s).unwrap();
    assert!(svg.contains("North &lt;safe&gt;"));
}
#[test]
fn source_reordering_preserves_datum_ids_and_independent_paint_alpha() {
    let mut d = data();
    d.push(RadarSeries::new("a:b", vec![0.6; 4]));
    let mut o = options();
    o.series.fill_color = Some(Color(200, 0, 0, 200));
    o.series.fill_opacity = 0.25;
    o.series.border.color = Some(Color(0, 0, 200, 240));
    o.series.border.opacity = 0.75;
    o.series.border.dash = Some(StrokeDash {
        segments: vec![8., 4.],
        offset: 2.,
    });
    let s = radar_with_options(&config(), &axes(), &d, &o).unwrap();
    let m = series(&s);
    assert_eq!(m.opacity, 1.);
    assert_eq!(m.fill, Some(Color(200, 0, 0, 50)));
    assert_eq!(m.stroke.as_ref().unwrap().color, Color(0, 0, 200, 180));
    assert_eq!(m.stroke_dash.as_ref().unwrap().segments, vec![8., 4.]);
    assert_eq!(
        s.marks
            .iter()
            .find(|m| m.id == "radar-point:5:alpha:5:north")
            .unwrap()
            .value,
        1.
    );
    let ids = s
        .marks
        .iter()
        .filter(|m| !m.series.is_empty())
        .map(|m| m.id.clone())
        .collect::<BTreeSet<_>>();
    d.reverse();
    let next = radar_with_options(&config(), &axes(), &d, &o).unwrap();
    assert_eq!(
        ids,
        next.marks
            .iter()
            .filter(|m| !m.series.is_empty())
            .map(|m| m.id.clone())
            .collect()
    );
}
#[test]
fn symbols_preserve_area_and_series_overrides_are_real_geometry() {
    let mut o = options();
    o.points.size = 20.;
    o.series_styles = BTreeMap::from([(
        "alpha".into(),
        RadarSeriesStyle {
            points: Some(RadarPointOptions {
                shape: RadarPointShape::Cross,
                size: 20.,
                ..Default::default()
            }),
            ..Default::default()
        },
    )]);
    for shape in [
        RadarPointShape::Square,
        RadarPointShape::Diamond,
        RadarPointShape::Triangle,
        RadarPointShape::Cross,
    ] {
        o.series_styles
            .get_mut("alpha")
            .unwrap()
            .points
            .as_mut()
            .unwrap()
            .shape = shape;
        let s = radar_with_options(&config(), &axes(), &data(), &o).unwrap();
        let m = s
            .marks
            .iter()
            .find(|m| m.id == "radar-point:5:alpha:5:north")
            .unwrap();
        let Geometry::Path(path) = &m.geometry else {
            panic!()
        };
        let vertices = path
            .iter()
            .filter_map(|c| match c {
                C::Move(p) | C::Line(p) => Some(*p),
                _ => None,
            })
            .collect::<Vec<P>>();
        let area = vertices
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let b = vertices[(i + 1) % vertices.len()];
                a.x * b.y - b.x * a.y
            })
            .sum::<f64>()
            .abs()
            / 2.;
        near(area, std::f64::consts::PI * 100.);
    }
}
#[test]
fn extreme_domains_and_limits_are_checked_without_mutating_source() {
    let mut o = options();
    let mut d = data();
    o.domain = Some([-f64::MAX, f64::MAX]);
    d[0].values = vec![-f64::MAX, 0., f64::MAX, 0.];
    radar_with_options(&config(), &axes(), &d, &o).unwrap();
    o.domain = Some([0., f64::from_bits(1)]);
    d[0].values = vec![f64::from_bits(1); 4];
    radar_with_options(&config(), &axes(), &d, &o).unwrap();
    let before = d.clone();
    o.domain = Some([0., 0.]);
    assert!(radar_with_options(&config(), &axes(), &d, &o).is_err());
    assert_eq!(d, before);
    let a = (0..128).map(|i| format!("a{i}")).collect::<Vec<_>>();
    let d = (0..128)
        .map(|i| RadarSeries::new(format!("s{i}"), vec![1.; 128]))
        .collect::<Vec<_>>();
    let mut o = RadarLayoutOptions::default();
    o.points.shape = RadarPointShape::Cross;
    assert!(
        radar_with_options(&config(), &a, &d, &o)
            .unwrap_err()
            .to_string()
            .contains("output budget")
    );
    // Admit a real near-budget scene; a disabled polygon grid and an extra
    // hypothetical cubic command must not artificially reject valid paths.
    let a = (0..113).map(|i| format!("a{i}")).collect::<Vec<_>>();
    let d = (0..126)
        .map(|i| RadarSeries::new(format!("s{i}"), vec![1.; 113]))
        .collect::<Vec<_>>();
    o.grid.shape = RadarGridShape::Polygon;
    o.grid.line.width = 0.;
    let s = radar_with_options(&config(), &a, &d, &o).unwrap();
    let count = s
        .marks
        .iter()
        .map(|m| match &m.geometry {
            Geometry::Path(p) => p.len(),
            _ => 0,
        })
        .sum::<usize>();
    assert_eq!(count, 199_684);
}
#[cfg(feature = "raster")]
#[test]
fn cpu_pixels_prove_fill_opacity_does_not_fade_the_independent_border() {
    use kartis::render::raster::*;
    let mut o = options();
    o.grid.line.enabled = false;
    o.grid.spokes.enabled = false;
    o.axis_labels = false;
    o.points.enabled = false;
    o.series.fill_color = Some(Color(255, 0, 0, 255));
    o.series.fill_opacity = 0.2;
    o.series.border.color = Some(Color(0, 0, 255, 255));
    o.series.border.width = 8.;
    let d = [RadarSeries::new("alpha", vec![1.; 4])];
    let s = radar_with_options(&config(), &axes(), &d, &o).unwrap();
    let image = render_raster(&s, &FontRegistry::new(), &Default::default()).unwrap();
    let px = |x: u32, y: u32| {
        &image.rgba[((y * image.width + x) * 4) as usize..((y * image.width + x) * 4 + 4) as usize]
    };
    assert_eq!(px(100, 100), [255, 204, 204, 255]);
    assert_eq!(px(132, 68), [0, 0, 255, 255]);
}
