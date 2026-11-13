//! Golden commands recorded using d3-shape 3.2.0 line().curve(...).context(recorder).
//! Reference implementation: https://github.com/d3/d3-shape/tree/v3.2.0/src/curve
//! Tests have no Node or network dependency; 0/1/2/3/4/5-point goldens include all boundary cases.
use glory::render::{GraphicPoint as P, PathCommand as Cmd};
use kartis::*;
fn config() -> ChartConfig {
    ChartConfig {
        width: 100.,
        height: 100.,
        margin: Margin {
            left: 0.,
            right: 0.,
            top: 0.,
            bottom: 0.,
        },
        ..Default::default()
    }
}
fn options(curve: Curve) -> LineOptions {
    LineOptions {
        curve,
        points: false,
        x_domain: Some([0., 100.]),
        y_domain: Some([0., 100.]),
        ..Default::default()
    }
}
fn input(curve: Curve) -> Vec<Point> {
    let p = if curve == Curve::MonotoneY {
        [(10., 0.), (30., 9.), (15., 25.), (70., 47.), (20., 80.)]
    } else {
        [(0., 10.), (9., 30.), (25., 15.), (47., 70.), (80., 20.)]
    };
    p.into_iter()
        .map(|(x, y)| Point::new(x, 100. - y))
        .collect()
}
fn paths(scene: &Scene, prefix: &str) -> Vec<Vec<Cmd>> {
    scene
        .marks
        .iter()
        .filter(|m| m.id.starts_with(prefix))
        .map(|m| m.geometry.commands())
        .collect()
}
fn numbers(commands: &[Cmd]) -> (String, Vec<f64>) {
    let mut topology = String::new();
    let mut values = vec![];
    for c in commands {
        match c {
            Cmd::Move(p) => {
                topology.push('M');
                values.extend([p.x, p.y]);
            }
            Cmd::Line(p) => {
                topology.push('L');
                values.extend([p.x, p.y]);
            }
            Cmd::Cubic { first, second, end } => {
                topology.push('C');
                values.extend([first.x, first.y, second.x, second.y, end.x, end.y]);
            }
            Cmd::Close => topology.push('Z'),
        }
    }
    (topology, values)
}
fn assert_golden(commands: &[Cmd], golden: &str, name: &str) {
    let (topology, values) = numbers(commands);
    let mut expected_topology = String::new();
    let mut expected = vec![];
    for t in golden.split_whitespace() {
        if matches!(t, "M" | "L" | "C" | "Z") {
            expected_topology.push_str(t);
        } else {
            expected.push(t.parse::<f64>().unwrap());
        }
    }
    assert_eq!(topology, expected_topology, "{name}: command topology");
    assert_eq!(values.len(), expected.len(), "{name}: coordinate count");
    for (actual, expected) in values.into_iter().zip(expected) {
        assert!(
            (actual - expected).abs() < 1e-10,
            "{name}: {actual} != {expected}"
        );
    }
}
#[test]
fn d3_golden_controls_endpoints_closure_and_boundary_point_counts() {
    let fixtures: &[(Curve, usize, &str)] = &[
        (Curve::Linear, 0, ""),
        (Curve::Linear, 1, "M 0 10 Z"),
        (Curve::Linear, 2, "M 0 10 L 9 30"),
        (Curve::Linear, 3, "M 0 10 L 9 30 L 25 15"),
        (Curve::Linear, 4, "M 0 10 L 9 30 L 25 15 L 47 70"),
        (Curve::Linear, 5, "M 0 10 L 9 30 L 25 15 L 47 70 L 80 20"),
        (Curve::LinearClosed, 0, ""),
        (Curve::LinearClosed, 1, "M 0 10 Z"),
        (Curve::LinearClosed, 2, "M 0 10 L 9 30 Z"),
        (Curve::LinearClosed, 3, "M 0 10 L 9 30 L 25 15 Z"),
        (Curve::LinearClosed, 4, "M 0 10 L 9 30 L 25 15 L 47 70 Z"),
        (
            Curve::LinearClosed,
            5,
            "M 0 10 L 9 30 L 25 15 L 47 70 L 80 20 Z",
        ),
        (Curve::Basis, 0, ""),
        (Curve::Basis, 1, "M 0 10 Z"),
        (Curve::Basis, 2, "M 0 10 L 9 30"),
        (
            Curve::Basis,
            3,
            "M 0 10 L 1.5 13.333333333333334 C 3 16.666666666666668 6 23.333333333333332 10.166666666666666 24.166666666666668 C 14.333333333333334 25 19.666666666666668 20 22.333333333333332 17.5 L 25 15",
        ),
        (
            Curve::Basis,
            4,
            "M 0 10 L 1.5 13.333333333333334 C 3 16.666666666666668 6 23.333333333333332 10.166666666666666 24.166666666666668 C 14.333333333333334 25 19.666666666666668 20 26 26.666666666666668 C 32.333333333333336 33.333333333333336 39.666666666666664 51.666666666666664 43.333333333333336 60.833333333333336 L 47 70",
        ),
        (
            Curve::Basis,
            5,
            "M 0 10 L 1.5 13.333333333333334 C 3 16.666666666666668 6 23.333333333333332 10.166666666666666 24.166666666666668 C 14.333333333333334 25 19.666666666666668 20 26 26.666666666666668 C 32.333333333333336 33.333333333333336 39.666666666666664 51.666666666666664 48.833333333333336 52.5 C 58 53.333333333333336 69 36.666666666666664 74.5 28.333333333333332 L 80 20",
        ),
        (Curve::BasisOpen, 0, ""),
        (Curve::BasisOpen, 1, ""),
        (Curve::BasisOpen, 2, ""),
        (
            Curve::BasisOpen,
            3,
            "M 10.166666666666666 24.166666666666668 Z",
        ),
        (
            Curve::BasisOpen,
            4,
            "M 10.166666666666666 24.166666666666668 C 14.333333333333334 25 19.666666666666668 20 26 26.666666666666668",
        ),
        (
            Curve::BasisOpen,
            5,
            "M 10.166666666666666 24.166666666666668 C 14.333333333333334 25 19.666666666666668 20 26 26.666666666666668 C 32.333333333333336 33.333333333333336 39.666666666666664 51.666666666666664 48.833333333333336 52.5",
        ),
        (Curve::BasisClosed, 0, ""),
        (Curve::BasisClosed, 1, "M 0 10 Z"),
        (
            Curve::BasisClosed,
            2,
            "M 6 23.333333333333332 L 3 16.666666666666668 Z",
        ),
        (
            Curve::BasisClosed,
            3,
            "M 10.166666666666666 24.166666666666668 C 14.333333333333334 25 19.666666666666668 20 18.166666666666668 16.666666666666668 C 16.666666666666668 13.333333333333334 8.333333333333334 11.666666666666666 5.666666666666667 14.166666666666666 C 3 16.666666666666668 6 23.333333333333332 10.166666666666666 24.166666666666668",
        ),
        (
            Curve::BasisClosed,
            4,
            "M 10.166666666666666 24.166666666666668 C 14.333333333333334 25 19.666666666666668 20 26 26.666666666666668 C 32.333333333333336 33.333333333333336 39.666666666666664 51.666666666666664 35.5 50.833333333333336 C 31.333333333333332 50 15.666666666666666 30 9.333333333333334 23.333333333333332 C 3 16.666666666666668 6 23.333333333333332 10.166666666666666 24.166666666666668",
        ),
        (
            Curve::BasisClosed,
            5,
            "M 10.166666666666666 24.166666666666668 C 14.333333333333334 25 19.666666666666668 20 26 26.666666666666668 C 32.333333333333336 33.333333333333336 39.666666666666664 51.666666666666664 48.833333333333336 52.5 C 58 53.333333333333336 69 36.666666666666664 61.166666666666664 26.666666666666668 C 53.333333333333336 16.666666666666668 26.666666666666668 13.333333333333334 14.833333333333334 15 C 3 16.666666666666668 6 23.333333333333332 10.166666666666666 24.166666666666668",
        ),
        (Curve::Cardinal, 0, ""),
        (Curve::Cardinal, 1, "M 0 10 Z"),
        (Curve::Cardinal, 2, "M 0 10 L 9 30"),
        (
            Curve::Cardinal,
            3,
            "M 0 10 C 0 10 4.833333333333334 29.166666666666668 9 30 C 13.166666666666666 30.833333333333332 25 15 25 15",
        ),
        (
            Curve::Cardinal,
            4,
            "M 0 10 C 0 10 4.833333333333334 29.166666666666668 9 30 C 13.166666666666666 30.833333333333332 18.666666666666668 8.333333333333334 25 15 C 31.333333333333332 21.666666666666664 47 70 47 70",
        ),
        (
            Curve::Cardinal,
            5,
            "M 0 10 C 0 10 4.833333333333334 29.166666666666668 9 30 C 13.166666666666666 30.833333333333332 18.666666666666668 8.333333333333334 25 15 C 31.333333333333332 21.666666666666664 37.833333333333336 69.16666666666667 47 70 C 56.166666666666664 70.83333333333333 80 20 80 20",
        ),
        (Curve::CardinalOpen, 0, ""),
        (Curve::CardinalOpen, 1, ""),
        (Curve::CardinalOpen, 2, ""),
        (Curve::CardinalOpen, 3, "M 9 30 Z"),
        (
            Curve::CardinalOpen,
            4,
            "M 9 30 C 13.166666666666666 30.833333333333332 18.666666666666668 8.333333333333334 25 15",
        ),
        (
            Curve::CardinalOpen,
            5,
            "M 9 30 C 13.166666666666666 30.833333333333332 18.666666666666668 8.333333333333334 25 15 C 31.333333333333332 21.666666666666664 37.833333333333336 69.16666666666667 47 70",
        ),
        (Curve::CardinalClosed, 0, ""),
        (Curve::CardinalClosed, 1, "M 0 10 Z"),
        (Curve::CardinalClosed, 2, "M 9 30 L 0 10 Z"),
        (
            Curve::CardinalClosed,
            3,
            "M 9 30 C 13.166666666666666 30.833333333333332 26.5 18.333333333333332 25 15 C 23.5 11.666666666666668 2.6666666666666665 7.5 0 10 C -2.6666666666666665 12.5 4.833333333333334 29.166666666666668 9 30",
        ),
        (
            Curve::CardinalClosed,
            4,
            "M 9 30 C 13.166666666666666 30.833333333333332 18.666666666666668 8.333333333333334 25 15 C 31.333333333333332 21.666666666666664 51.166666666666664 70.83333333333333 47 70 C 42.833333333333336 69.16666666666667 6.333333333333333 16.666666666666664 0 10 C -6.333333333333333 3.333333333333334 4.833333333333334 29.166666666666668 9 30",
        ),
        (
            Curve::CardinalClosed,
            5,
            "M 9 30 C 13.166666666666666 30.833333333333332 18.666666666666668 8.333333333333334 25 15 C 31.333333333333332 21.666666666666664 37.833333333333336 69.16666666666667 47 70 C 56.166666666666664 70.83333333333333 87.83333333333333 30 80 20 C 72.16666666666667 10 11.833333333333332 8.333333333333334 0 10 C -11.833333333333332 11.666666666666666 4.833333333333334 29.166666666666668 9 30",
        ),
        (Curve::CardinalTension { tension: 0.4 }, 0, ""),
        (Curve::CardinalTension { tension: 0.4 }, 1, "M 0 10 Z"),
        (Curve::CardinalTension { tension: 0.4 }, 2, "M 0 10 L 9 30"),
        (
            Curve::CardinalTension { tension: 0.4 },
            3,
            "M 0 10 C 0 10 6.5 29.5 9 30 C 11.5 30.5 25 15 25 15",
        ),
        (
            Curve::CardinalTension { tension: 0.4 },
            4,
            "M 0 10 C 0 10 6.5 29.5 9 30 C 11.5 30.5 21.2 11 25 15 C 28.8 19 47 70 47 70",
        ),
        (
            Curve::CardinalTension { tension: 0.4 },
            5,
            "M 0 10 C 0 10 6.5 29.5 9 30 C 11.5 30.5 21.2 11 25 15 C 28.8 19 41.5 69.5 47 70 C 52.5 70.5 80 20 80 20",
        ),
        (Curve::CardinalOpenTension { tension: 0.4 }, 0, ""),
        (Curve::CardinalOpenTension { tension: 0.4 }, 1, ""),
        (Curve::CardinalOpenTension { tension: 0.4 }, 2, ""),
        (Curve::CardinalOpenTension { tension: 0.4 }, 3, "M 9 30 Z"),
        (
            Curve::CardinalOpenTension { tension: 0.4 },
            4,
            "M 9 30 C 11.5 30.5 21.2 11 25 15",
        ),
        (
            Curve::CardinalOpenTension { tension: 0.4 },
            5,
            "M 9 30 C 11.5 30.5 21.2 11 25 15 C 28.8 19 41.5 69.5 47 70",
        ),
        (Curve::CardinalClosedTension { tension: 0.4 }, 0, ""),
        (Curve::CardinalClosedTension { tension: 0.4 }, 1, "M 0 10 Z"),
        (
            Curve::CardinalClosedTension { tension: 0.4 },
            2,
            "M 9 30 L 0 10 Z",
        ),
        (
            Curve::CardinalClosedTension { tension: 0.4 },
            3,
            "M 9 30 C 11.5 30.5 25.9 17 25 15 C 24.1 13 1.5999999999999999 8.5 0 10 C -1.5999999999999999 11.5 6.5 29.5 9 30",
        ),
        (
            Curve::CardinalClosedTension { tension: 0.4 },
            4,
            "M 9 30 C 11.5 30.5 21.2 11 25 15 C 28.8 19 49.5 70.5 47 70 C 44.5 69.5 3.8 14 0 10 C -3.8 6 6.5 29.5 9 30",
        ),
        (
            Curve::CardinalClosedTension { tension: 0.4 },
            5,
            "M 9 30 C 11.5 30.5 21.2 11 25 15 C 28.8 19 41.5 69.5 47 70 C 52.5 70.5 84.7 26 80 20 C 75.3 14 7.1 9 0 10 C -7.1 11 6.5 29.5 9 30",
        ),
        (Curve::CatmullRom { alpha: 0.5 }, 0, ""),
        (Curve::CatmullRom { alpha: 0.5 }, 1, "M 0 10 Z"),
        (Curve::CatmullRom { alpha: 0.5 }, 2, "M 0 10 L 9 30"),
        (
            Curve::CatmullRom { alpha: 0.5 },
            3,
            "M 0 10 C 0 10 4.833333333333333 29.166666666666664 9 30 C 13.166666666666666 30.833333333333336 25 15 25 15",
        ),
        (
            Curve::CatmullRom { alpha: 0.5 },
            4,
            "M 0 10 C 0 10 4.833333333333333 29.166666666666664 9 30 C 13.166666666666666 30.833333333333336 19.99623721025864 13.88857574814574 25 15 C 33.22349222487197 16.82658312907957 47 70 47 70",
        ),
        (
            Curve::CatmullRom { alpha: 0.5 },
            5,
            "M 0 10 C 0 10 4.833333333333333 29.166666666666664 9 30 C 13.166666666666666 30.833333333333336 19.99623721025864 13.88857574814574 25 15 C 33.22349222487197 16.82658312907957 37.86932099971214 69.07065551877405 47 70 C 56.18228138803526 70.93459670773282 80 20 80 20",
        ),
        (Curve::CatmullRomOpen { alpha: 0.5 }, 0, ""),
        (Curve::CatmullRomOpen { alpha: 0.5 }, 1, ""),
        (Curve::CatmullRomOpen { alpha: 0.5 }, 2, ""),
        (Curve::CatmullRomOpen { alpha: 0.5 }, 3, "M 9 30 Z"),
        (
            Curve::CatmullRomOpen { alpha: 0.5 },
            4,
            "M 9 30 C 13.166666666666666 30.833333333333336 19.99623721025864 13.88857574814574 25 15",
        ),
        (
            Curve::CatmullRomOpen { alpha: 0.5 },
            5,
            "M 9 30 C 13.166666666666666 30.833333333333336 19.99623721025864 13.88857574814574 25 15 C 33.22349222487197 16.82658312907957 37.86932099971214 69.07065551877405 47 70",
        ),
        (Curve::CatmullRomClosed { alpha: 0.5 }, 0, ""),
        (Curve::CatmullRomClosed { alpha: 0.5 }, 1, "M 0 10 Z"),
        (Curve::CatmullRomClosed { alpha: 0.5 }, 2, "M 9 30 L 0 10 Z"),
        (
            Curve::CatmullRomClosed { alpha: 0.5 },
            3,
            "M 9 30 C 13.166666666666666 30.833333333333336 25.95215731929398 18.337879930346393 25 15 C 23.973401133809396 11.401157999273892 2.3317997229167484 7.072837717095363 0 10 C -2.1627144217892145 12.714905582129756 4.833333333333333 29.166666666666664 9 30",
        ),
        (
            Curve::CatmullRomClosed { alpha: 0.5 },
            4,
            "M 9 30 C 13.166666666666666 30.833333333333336 19.99623721025864 13.88857574814574 25 15 C 33.22349222487197 16.82658312907957 49.573892258832004 68.51772801841412 47 70 C 44.0804234564999 71.68134718680473 1.8298768561547474 8.893957101888278 0 10 C -0.9815960210769212 10.593311677928035 4.833333333333333 29.166666666666664 9 30",
        ),
        (
            Curve::CatmullRomClosed { alpha: 0.5 },
            5,
            "M 9 30 C 13.166666666666666 30.833333333333336 19.99623721025864 13.88857574814574 25 15 C 33.22349222487197 16.82658312907957 37.86932099971214 69.07065551877405 47 70 C 56.18228138803526 70.93459670773282 84.7342329868032 30.28110105435672 80 20 C 74.50794849143647 8.073181714404525 5.3605731519198585 2.7420022889664355 0 10 C -2.795884051943765 13.785513129702455 4.833333333333333 29.166666666666664 9 30",
        ),
        (Curve::CatmullRomOpen { alpha: 0.0 }, 0, ""),
        (Curve::CatmullRomOpen { alpha: 0.0 }, 1, ""),
        (Curve::CatmullRomOpen { alpha: 0.0 }, 2, ""),
        (Curve::CatmullRomOpen { alpha: 0.0 }, 3, "M 9 30 Z"),
        (
            Curve::CatmullRomOpen { alpha: 0.0 },
            4,
            "M 9 30 C 13.166666666666666 30.833333333333332 18.666666666666668 8.333333333333334 25 15",
        ),
        (
            Curve::CatmullRomOpen { alpha: 0.0 },
            5,
            "M 9 30 C 13.166666666666666 30.833333333333332 18.666666666666668 8.333333333333334 25 15 C 31.333333333333332 21.666666666666664 37.833333333333336 69.16666666666667 47 70",
        ),
        (Curve::CatmullRomClosed { alpha: 1.0 }, 0, ""),
        (Curve::CatmullRomClosed { alpha: 1.0 }, 1, "M 0 10 Z"),
        (Curve::CatmullRomClosed { alpha: 1.0 }, 2, "M 9 30 L 0 10 Z"),
        (
            Curve::CatmullRomClosed { alpha: 1.0 },
            3,
            "M 9 30 C 13.16666666666667 30.83333333333334 25.447972670819667 18.350835577731136 25 15 C 24.479242347954507 11.104732762699737 1.9788790777728613 6.604660108663407 0 10 C -1.7022961491147492 12.920781813744254 4.833333333333334 29.16666666666667 9 30",
        ),
        (
            Curve::CatmullRomClosed { alpha: 1.0 },
            4,
            "M 9 30 C 13.16666666666667 30.83333333333334 20.374118558475054 16.814968320281793 25 15 C 37.49434931744066 10.097831952603231 48.19868187585231 66.48209067249863 47 70 C 45.45772100792991 74.52630323450495 -4.595138399186159 -3.521853643490441 0 10 C 1.322270918114097 13.890971778989112 4.833333333333334 29.16666666666667 9 30",
        ),
        (
            Curve::CatmullRomClosed { alpha: 1.0 },
            5,
            "M 9 30 C 13.16666666666667 30.83333333333334 20.374118558475054 16.814968320281793 25 15 C 37.49434931744066 10.097831952603231 37.90496167525105 68.97517077334797 47 70 C 56.19813074545721 71.0364456841111 82.13651854684225 30.617573373311792 80 20 C 77.12474335129785 5.711217681766506 -2.9669925054876316 -8.55333625184798 0 10 C 0.8071092212605819 15.047053117363248 4.833333333333334 29.16666666666667 9 30",
        ),
        (Curve::Monotone, 0, ""),
        (Curve::Monotone, 1, "M 0 10 Z"),
        (Curve::Monotone, 2, "M 0 10 L 9 30"),
        (
            Curve::Monotone,
            3,
            "M 0 10 C 3 20 6 30 9 30 C 14.333333333333332 30 19.666666666666668 22.5 25 15",
        ),
        (
            Curve::Monotone,
            4,
            "M 0 10 C 3 20 6 30 9 30 C 14.333333333333332 30 19.666666666666668 15 25 15 C 32.333333333333336 15 39.666666666666664 42.5 47 70",
        ),
        (
            Curve::Monotone,
            5,
            "M 0 10 C 3 20 6 30 9 30 C 14.333333333333332 30 19.666666666666668 15 25 15 C 32.333333333333336 15 39.666666666666664 70 47 70 C 58 70 69 45 80 20",
        ),
        (Curve::MonotoneX, 0, ""),
        (Curve::MonotoneX, 1, "M 0 10 Z"),
        (Curve::MonotoneX, 2, "M 0 10 L 9 30"),
        (
            Curve::MonotoneX,
            3,
            "M 0 10 C 3 20 6 30 9 30 C 14.333333333333332 30 19.666666666666668 22.5 25 15",
        ),
        (
            Curve::MonotoneX,
            4,
            "M 0 10 C 3 20 6 30 9 30 C 14.333333333333332 30 19.666666666666668 15 25 15 C 32.333333333333336 15 39.666666666666664 42.5 47 70",
        ),
        (
            Curve::MonotoneX,
            5,
            "M 0 10 C 3 20 6 30 9 30 C 14.333333333333332 30 19.666666666666668 15 25 15 C 32.333333333333336 15 39.666666666666664 70 47 70 C 58 70 69 45 80 20",
        ),
        (Curve::MonotoneY, 0, ""),
        (Curve::MonotoneY, 1, "M 10 0 Z"),
        (Curve::MonotoneY, 2, "M 10 0 L 30 9"),
        (
            Curve::MonotoneY,
            3,
            "M 10 0 C 20 3 30 6 30 9 C 30 14.333333333333332 22.5 19.666666666666668 15 25",
        ),
        (
            Curve::MonotoneY,
            4,
            "M 10 0 C 20 3 30 6 30 9 C 30 14.333333333333332 15 19.666666666666668 15 25 C 15 32.333333333333336 42.5 39.666666666666664 70 47",
        ),
        (
            Curve::MonotoneY,
            5,
            "M 10 0 C 20 3 30 6 30 9 C 30 14.333333333333332 15 19.666666666666668 15 25 C 15 32.333333333333336 70 39.666666666666664 70 47 C 70 58 45 69 20 80",
        ),
        (Curve::BumpX, 0, ""),
        (Curve::BumpX, 1, "M 0 10 Z"),
        (Curve::BumpX, 2, "M 0 10 C 4.5 10 4.5 30 9 30"),
        (
            Curve::BumpX,
            3,
            "M 0 10 C 4.5 10 4.5 30 9 30 C 17 30 17 15 25 15",
        ),
        (
            Curve::BumpX,
            4,
            "M 0 10 C 4.5 10 4.5 30 9 30 C 17 30 17 15 25 15 C 36 15 36 70 47 70",
        ),
        (
            Curve::BumpX,
            5,
            "M 0 10 C 4.5 10 4.5 30 9 30 C 17 30 17 15 25 15 C 36 15 36 70 47 70 C 63.5 70 63.5 20 80 20",
        ),
        (Curve::BumpY, 0, ""),
        (Curve::BumpY, 1, "M 0 10 Z"),
        (Curve::BumpY, 2, "M 0 10 C 0 20 9 20 9 30"),
        (
            Curve::BumpY,
            3,
            "M 0 10 C 0 20 9 20 9 30 C 9 22.5 25 22.5 25 15",
        ),
        (
            Curve::BumpY,
            4,
            "M 0 10 C 0 20 9 20 9 30 C 9 22.5 25 22.5 25 15 C 25 42.5 47 42.5 47 70",
        ),
        (
            Curve::BumpY,
            5,
            "M 0 10 C 0 20 9 20 9 30 C 9 22.5 25 22.5 25 15 C 25 42.5 47 42.5 47 70 C 47 45 80 45 80 20",
        ),
    ];
    for (curve, n, golden) in fixtures {
        let data = input(*curve).into_iter().take(*n).collect();
        let scene = line(&config(), &[Series::new("s", data)], &options(*curve)).unwrap();
        let commands = paths(&scene, "line-")
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        assert_golden(&commands, golden, &format!("{curve:?}, {n} points"));
    }
}

fn variants() -> Vec<Curve> {
    vec![
        Curve::LinearClosed,
        Curve::BasisOpen,
        Curve::BasisClosed,
        Curve::Cardinal,
        Curve::CardinalOpen,
        Curve::CardinalClosed,
        Curve::cardinal(0.3),
        Curve::cardinal_open(1.),
        Curve::cardinal_closed(0.6),
        Curve::CatmullRomOpen { alpha: 0.5 },
        Curve::CatmullRomClosed { alpha: 0.5 },
        Curve::MonotoneX,
        Curve::MonotoneY,
        Curve::BumpX,
        Curve::BumpY,
    ]
}
fn endpoint(commands: &[Cmd]) -> P {
    match commands.last().unwrap() {
        Cmd::Move(p) | Cmd::Line(p) => *p,
        Cmd::Cubic { end, .. } => *end,
        Cmd::Close => match commands[0] {
            Cmd::Move(p) => p,
            _ => panic!(),
        },
    }
}
#[test]
fn chunks_preserve_actual_open_endpoints_and_global_closure_without_false_chords() {
    for curve in variants() {
        let data = (0..800)
            .map(|i| {
                let t = i as f64 / 799.;
                if curve == Curve::MonotoneY {
                    Point::new(50. + 30. * (t * 20.).sin(), 100. * t)
                } else {
                    Point::new(100. * t, 50. + 30. * (t * 20.).sin())
                }
            })
            .collect();
        let scene = line(&config(), &[Series::new("long", data)], &options(curve)).unwrap();
        let chunks = paths(&scene, "line-");
        assert!(chunks.len() > 1, "{curve:?}");
        assert!(
            scene
                .marks
                .iter()
                .all(|m| m.geometry.commands().len() <= 256)
        );
        for pair in chunks.windows(2) {
            let end = endpoint(&pair[0]);
            assert_eq!(
                pair[1][0],
                Cmd::Move(end),
                "{curve:?}: continuity at budget boundary"
            );
        }
        if curve == Curve::BasisOpen {
            let y = |i: f64| 50. - 30. * (i / 799. * 20.).sin();
            assert_golden(
                &chunks[0][..1],
                &format!("M {} {}", 100. / 799., (y(0.) + 4. * y(1.) + y(2.)) / 6.),
                "basis-open start",
            );
        }
        if curve == Curve::CardinalOpen
            || curve == Curve::cardinal_open(1.)
            || matches!(curve, Curve::CatmullRomOpen { .. })
        {
            assert_golden(
                &chunks[0][..1],
                &format!("M {} {}", 100. / 799., 50. - 30. * (20_f64 / 799.).sin()),
                "interpolating open start",
            );
        }
        if matches!(
            curve,
            Curve::LinearClosed
                | Curve::BasisClosed
                | Curve::CardinalClosed
                | Curve::CardinalClosedTension { .. }
                | Curve::CatmullRomClosed { .. }
        ) {
            let Cmd::Move(start) = chunks[0][0] else {
                panic!()
            };
            assert_eq!(
                endpoint(chunks.last().unwrap()),
                start,
                "{curve:?}: closes to global start"
            );
            assert!(
                chunks.iter().all(|c| !c.contains(&Cmd::Close)),
                "a split run must not close to local chunk start"
            );
        }
    }
    // A 700-edge polygon must retain its signed area after chunking. A local
    // Close at the final chunk would omit the true last-to-first edge.
    let data = (0..700)
        .map(|i| {
            let a = i as f64 * std::f64::consts::TAU / 700.;
            Point::new(50. + 40. * a.cos(), 50. + 40. * a.sin())
        })
        .collect();
    let scene = line(
        &config(),
        &[Series::new("ring", data)],
        &options(Curve::LinearClosed),
    )
    .unwrap();
    let mut twice_area = 0.;
    for chunk in paths(&scene, "line-") {
        let mut current = match chunk[0] {
            Cmd::Move(p) => p,
            _ => panic!(),
        };
        for c in &chunk[1..] {
            let Cmd::Line(next) = c else { panic!() };
            twice_area += current.x * next.y - next.x * current.y;
            current = *next;
        }
    }
    let expected = 700. * 1600. * (std::f64::consts::TAU / 700.).sin();
    assert!((twice_area.abs() - expected).abs() < 1e-8);
}
#[test]
fn defined_runs_restart_periodic_or_open_splines_without_bridging_a_gap() {
    for curve in variants() {
        let mut data = input(curve);
        data.push(Point::gap(80.));
        data.extend(input(curve));
        let scene = line(&config(), &[Series::new("gaps", data)], &options(curve)).unwrap();
        let chunks = paths(&scene, "line-");
        assert_eq!(chunks.len(), 2, "{curve:?}");
        assert_eq!(
            chunks[0], chunks[1],
            "state must restart at each gap: {curve:?}"
        );
    }
}
#[test]
fn areas_use_actual_open_endpoints_and_closed_areas_do_not_connect_to_baseline() {
    for curve in variants() {
        for n in 0..=5 {
            let mut o = options(curve);
            o.area = true;
            let scene = line(
                &config(),
                &[Series::new("s", input(curve).into_iter().take(n).collect())],
                &o,
            )
            .unwrap();
            let lines = paths(&scene, "line-");
            let fills = paths(&scene, "area-");
            assert_eq!(lines.len(), fills.len(), "{curve:?}, {n}");
            for (top, area) in lines.into_iter().zip(fills) {
                assert_eq!(area.last(), Some(&Cmd::Close));
                let closed = matches!(
                    curve,
                    Curve::LinearClosed
                        | Curve::BasisClosed
                        | Curve::CardinalClosed
                        | Curve::CardinalClosedTension { .. }
                        | Curve::CatmullRomClosed { .. }
                );
                let mut expected = top.clone();
                if expected.last() == Some(&Cmd::Close) {
                    expected.pop();
                }
                if !closed {
                    let start = match top[0] {
                        Cmd::Move(p) => p,
                        _ => panic!(),
                    };
                    let end = endpoint(&top);
                    expected.extend([
                        Cmd::Line(P::new(end.x, 100.)),
                        Cmd::Line(P::new(start.x, 100.)),
                    ]);
                }
                expected.push(Cmd::Close);
                assert_eq!(area, expected, "{curve:?}, {n}: constant baseline geometry");
            }
        }
    }
    let long = (0..700)
        .map(|i| Point::new(i as f64 / 7., (i as f64 * 0.1).sin() + 50.))
        .collect::<Vec<_>>();
    for curve in [
        Curve::BasisOpen,
        Curve::CardinalOpen,
        Curve::CatmullRomOpen { alpha: 0.5 },
        Curve::BumpX,
    ] {
        let mut o = options(curve);
        o.area = true;
        let scene = line(&config(), &[Series::new("s", long.clone())], &o).unwrap();
        assert!(paths(&scene, "area-").len() > 1);
        assert!(
            scene
                .marks
                .iter()
                .all(|m| m.geometry.commands().len() <= 256)
        );
    }
}

#[derive(Default)]
struct PathRecorder {
    current: Vec<Cmd>,
    fills: Vec<Vec<Cmd>>,
}
impl render::Canvas for PathRecorder {
    fn supports_clipping(&self) -> bool {
        true
    }
    fn push_clip(&mut self, _: ClipRect) -> Result<()> {
        Ok(())
    }
    fn pop_clip(&mut self) -> Result<()> {
        Ok(())
    }
    fn begin(&mut self, _: f64, _: f64, _: Color) -> Result<()> {
        Ok(())
    }
    fn begin_path(&mut self) -> Result<()> {
        self.current.clear();
        Ok(())
    }
    fn move_to(&mut self, x: f64, y: f64) -> Result<()> {
        self.current.push(Cmd::Move(P::new(x, y)));
        Ok(())
    }
    fn line_to(&mut self, x: f64, y: f64) -> Result<()> {
        self.current.push(Cmd::Line(P::new(x, y)));
        Ok(())
    }
    fn cubic_to(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x: f64, y: f64) -> Result<()> {
        self.current.push(Cmd::Cubic {
            first: P::new(x1, y1),
            second: P::new(x2, y2),
            end: P::new(x, y),
        });
        Ok(())
    }
    fn close_path(&mut self) -> Result<()> {
        self.current.push(Cmd::Close);
        Ok(())
    }
    fn fill(&mut self, _: Color, _: f64) -> Result<()> {
        self.fills.push(self.current.clone());
        Ok(())
    }
    fn stroke(&mut self, _: glory::render::GraphicStroke, _: f64) -> Result<()> {
        Ok(())
    }
    fn text(&mut self, _: &Label) -> Result<()> {
        Ok(())
    }
    fn finish(&mut self) -> Result<()> {
        Ok(())
    }
}
#[test]
fn long_periodic_areas_keep_one_complete_contour_for_direct_renderers() {
    let data = (0..700)
        .map(|i| {
            let a = i as f64 * std::f64::consts::TAU / 700.;
            Point::new(50. + 40. * a.cos(), 50. + 40. * a.sin())
        })
        .collect::<Vec<_>>();
    for curve in [
        Curve::LinearClosed,
        Curve::BasisClosed,
        Curve::CardinalClosed,
        Curve::cardinal_closed(0.4),
        Curve::CatmullRomClosed { alpha: 0.5 },
    ] {
        let mut o = options(curve);
        o.area = true;
        let scene = line(&config(), &[Series::new("ring", data.clone())], &o).unwrap();
        let fills = paths(&scene, "area-");
        assert_eq!(fills.len(), 1, "{curve:?}: no local fill chords");
        let fill = &fills[0];
        assert!(fill.len() > glory::render::MAX_GRAPHIC_COMMANDS);
        assert_eq!(fill.iter().filter(|c| matches!(c, Cmd::Move(_))).count(), 1);
        assert_eq!(fill.iter().filter(|c| matches!(c, Cmd::Close)).count(), 1);
        assert_eq!(fill.last(), Some(&Cmd::Close));
        // Native Canvas replays the entire contour as one fill, without host
        // Graphic limits or opacity-altering segmentation.
        let mut recorder = PathRecorder::default();
        render::draw(&scene, &mut recorder).unwrap();
        assert!(recorder.fills.contains(fill));
        let svg = render::to_svg(&scene).unwrap();
        assert_eq!(svg.matches("data-id=\"area-0-0-0\"").count(), 1);
        assert_eq!(svg.matches("data-id=\"area-").count(), 1);
        let script = render::to_canvas_script(&scene, "long-contour").unwrap();
        assert!(
            script.matches("ctx.bezierCurveTo(").count()
                >= if curve == Curve::LinearClosed {
                    0
                } else {
                    1400
                }
        );
        // BrowserChart uses this plot(false) control host together with the
        // direct SVG/Canvas surface, so its control admission remains valid.
        assert!(glory::ssr::render(chart(scene.clone()).plot(false)).is_ok());
        assert!(
            glory::ssr::render(chart(scene)).is_err(),
            "the low-level Graphic Widget must apply its own per-node budget"
        );
    }
}
#[test]
fn monotone_axis_validation_is_per_run_and_supports_both_directions() {
    let data = vec![Point::new(1., 3.), Point::new(2., 1.), Point::new(3., 2.)];
    assert!(
        line(
            &config(),
            &[Series::new("s", data.clone())],
            &options(Curve::MonotoneX)
        )
        .is_ok()
    );
    assert!(
        line(
            &config(),
            &[Series::new("s", data)],
            &options(Curve::MonotoneY)
        )
        .is_err()
    );
    for curve in [Curve::Monotone, Curve::MonotoneX, Curve::MonotoneY] {
        let mut data = input(curve);
        data.reverse();
        assert!(
            line(
                &config(),
                &[Series::new("descending", data.clone())],
                &options(curve)
            )
            .is_ok()
        );
        data.push(Point::gap(0.));
        data.extend(input(curve));
        assert!(
            line(
                &config(),
                &[Series::new("direction resets", data)],
                &options(curve)
            )
            .is_ok()
        );
    }
    assert_eq!(
        paths(
            &line(
                &config(),
                &[Series::new("s", input(Curve::Monotone))],
                &options(Curve::Monotone)
            )
            .unwrap(),
            "line-"
        ),
        paths(
            &line(
                &config(),
                &[Series::new("s", input(Curve::MonotoneX))],
                &options(Curve::MonotoneX)
            )
            .unwrap(),
            "line-"
        )
    );
}
#[test]
fn parameter_domains_and_coincident_periodic_control_points_are_checked() {
    for bad in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
        for curve in [
            Curve::cardinal(bad),
            Curve::cardinal_open(bad),
            Curve::cardinal_closed(bad),
            Curve::CatmullRomOpen { alpha: bad },
            Curve::CatmullRomClosed { alpha: bad },
        ] {
            assert!(line(&config(), &[], &options(curve)).is_err());
        }
    }
    for curve in [
        Curve::CatmullRomOpen { alpha: 0.5 },
        Curve::CatmullRomClosed { alpha: 0.5 },
        Curve::CardinalClosed,
        Curve::BasisClosed,
    ] {
        let scene = line(
            &config(),
            &[Series::new("repeated", vec![Point::new(5., 5.); 5])],
            &options(curve),
        )
        .unwrap();
        scene.validate().unwrap();
        assert!(
            paths(&scene, "line-")
                .iter()
                .all(|p| numbers(p).1.iter().all(|v| v.is_finite()))
        );
    }
}
