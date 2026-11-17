use std::cell::Cell;

use glory::render::PathCommand;
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 500.,
        height: 340.,
        margin: Margin {
            top: 23.,
            right: 31.,
            bottom: 37.,
            left: 41.,
        },
        ..Default::default()
    }
}

fn source(state: &str) -> Vec<RadialDatum> {
    if state == "empty" {
        return vec![];
    }
    [
        ("zero", "", -0.),
        ("a", "<A>", 4.),
        ("hidden", "Hidden", 3.),
        ("b", "Bee", 4.),
        ("overflow", "Tail", 8.),
    ]
    .map(|(id, label, value)| RadialDatum {
        id: id.into(),
        label: label.into(),
        value,
    })
    .into()
}

fn options(direction: &str, state: &str) -> WaffleLayoutOptions {
    WaffleLayoutOptions {
        rows: 3,
        columns: 4,
        total: Some(if state == "sparse" { 32. } else { 16. }),
        direction: match direction {
            "top" => WaffleFillDirection::Top,
            "right" => WaffleFillDirection::Right,
            "bottom" => WaffleFillDirection::Bottom,
            "left" => WaffleFillDirection::Left,
            _ => unreachable!(),
        },
        padding: 8.,
        corner_radius: 3.,
        border_width: 2.,
        hidden: if state == "all-hidden" {
            source(state).into_iter().map(|d| d.id).collect()
        } else {
            ["hidden".into()].into()
        },
        ..Default::default()
    }
}

fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}

fn palette(index: usize) -> Color {
    [
        Color(17, 34, 51, 255),
        Color(171, 34, 51, 255),
        Color(68, 85, 102, 255),
        Color(34, 119, 170, 255),
        Color(102, 170, 51, 255),
    ][index]
}

fn frame(direction: &str, state: &str) -> WaffleSvgFrame {
    let calls: [Cell<usize>; 3] = std::array::from_fn(|_| Cell::new(0));
    let mut color = |c: &WaffleGroupColorContext<'_>| {
        calls[0].set(calls[0].get() + 1);
        Ok(palette(c.source_index))
    };
    let mut formatter = |value: f64| {
        calls[1].set(calls[1].get() + 1);
        Ok(if value.to_bits() == (-0.0_f64).to_bits() {
            "raw:-0".into()
        } else {
            format!("raw:{value}")
        })
    };
    let mut border = |c: &WaffleBorderColorContext<'_>| {
        calls[2].set(calls[2].get() + 1);
        Ok(match c.target {
            WaffleBorderTarget::Group(_) => Color(68, 85, 102, 255),
            WaffleBorderTarget::Empty(_) => Color(136, 136, 136, 255),
        })
    };
    let source = source(state);
    let result = WaffleSvgFrame::with_factories(
        &config(),
        &source,
        &options(direction, state),
        &mut WaffleFactories {
            color: Some(&mut color),
            formatted_value: Some(&mut formatter),
            border_color: Some(&mut border),
        },
    )
    .unwrap();
    assert_eq!(
        [calls[0].get(), calls[1].get(), calls[2].get()],
        [source.len(), source.len(), source.len() + 1]
    );
    result
}

fn number(fields: &[&str], index: usize) -> f64 {
    fields[index].parse().unwrap()
}

fn polygon_matches(actual: &[[f64; 2]], expected: &[[f64; 2]]) -> bool {
    // The original helper keeps redundant row endpoints. The existing Rust
    // outline emits only corners. Removing zero-length edges and intermediate
    // points on a forward axis-aligned segment preserves the complete winding
    // boundary, including every concavity. A reversal is deliberately retained.
    let canonical = |points: &[[f64; 2]]| {
        let mut p = points.to_vec();
        loop {
            let remove = (0..p.len()).find(|&i| {
                if p.len() < 3 {
                    return false;
                }
                let a = p[(i + p.len() - 1) % p.len()];
                let b = p[i];
                let c = p[(i + 1) % p.len()];
                let equal = |x: f64, y: f64| (x - y).abs() < 1e-8;
                let duplicate = equal(a[0], b[0]) && equal(a[1], b[1]);
                let vertical =
                    equal(a[0], b[0]) && equal(b[0], c[0]) && (b[1] - a[1]) * (c[1] - b[1]) >= 0.;
                let horizontal =
                    equal(a[1], b[1]) && equal(b[1], c[1]) && (b[0] - a[0]) * (c[0] - b[0]) >= 0.;
                duplicate || vertical || horizontal
            });
            match remove {
                Some(i) => {
                    p.remove(i);
                }
                None => break,
            }
        }
        p
    };
    let actual = canonical(actual);
    let expected = canonical(expected);
    if actual.len() < 3 || expected.len() < 3 {
        return false;
    }
    if actual.len() != expected.len() {
        return false;
    }
    (0..actual.len()).any(|start| {
        [false, true].into_iter().any(|reverse| {
            expected.iter().enumerate().all(|(i, expected)| {
                let index = if reverse {
                    (start + actual.len() - i) % actual.len()
                } else {
                    (start + i) % actual.len()
                };
                actual[index]
                    .iter()
                    .zip(expected)
                    .all(|(a, b)| (a - b).abs() < 1e-8)
            })
        })
    })
}

#[test]
fn all_sixteen_original_function_cases_match_captured_cells_and_full_slot_polygons() {
    let frames: Vec<_> = ["top", "right", "bottom", "left"]
        .into_iter()
        .flat_map(|direction| {
            ["full", "sparse", "all-hidden", "empty"]
                .into_iter()
                .map(move |state| frame(direction, state))
        })
        .collect();
    let mut cases = 0;
    let mut cells = 0;
    let mut polygons = 0;
    for line in include_str!("fixtures/waffle-svg/cases.tsv").lines() {
        let f: Vec<_> = line.split('\t').collect();
        let case: usize = f[1].parse().unwrap();
        let capture = &frames[case];
        match f[0] {
            "CASE" => {
                assert_eq!(capture.source(), source(f[3]));
                assert_eq!(capture.options(), &options(f[2], f[3]));
                assert_eq!(capture.cells().len(), 12);
                assert_eq!(capture.config().width, 500.);
                assert_eq!(capture.config().height, 340.);
                assert_eq!(capture, &capture.clone());
                assert_ne!(capture, &frame(f[2], f[3]));
                if let Some(first) = capture.source().first() {
                    assert_eq!(first.value.to_bits(), (-0.0_f64).to_bits());
                }
                cases += 1;
            }
            "CELL" => {
                let index: usize = f[2].parse().unwrap();
                let cell = &capture.cells()[index];
                assert_eq!(cell.index, index);
                assert_eq!(cell.row, f[3].parse::<usize>().unwrap());
                assert_eq!(cell.column, f[4].parse::<usize>().unwrap());
                let expected_owner: i32 = f[5].parse().unwrap();
                assert_eq!(cell.owner.map(|i| i as i32).unwrap_or(-1), expected_owner);
                near(cell.bounds.x, number(&f, 6) + 41.);
                near(cell.bounds.y, number(&f, 7) + 23.);
                near(cell.bounds.width, number(&f, 8));
                near(cell.bounds.height, number(&f, 9));
                let mark = capture.cell_mark(index).unwrap().unwrap();
                assert_eq!(mark.id, cell.mark_id.as_ref().unwrap().as_str());
                let mark_slot = capture
                    .scene()
                    .marks
                    .iter()
                    .position(|m| m.id == mark.id)
                    .unwrap();
                assert_eq!(capture.cell_for_mark(mark_slot).unwrap(), Some(index));
                assert_eq!(
                    mark.fill,
                    Some(cell.owner.map_or(Color(204, 204, 204, 255), palette))
                );
                assert_eq!(mark.stroke.unwrap().width, 2.);
                cells += 1;
            }
            "GROUP" => {
                let index: usize = f[2].parse().unwrap();
                let group = &capture.groups()[index];
                assert_eq!(group.source_index, index);
                assert_eq!(group.hidden, f[3] == "1");
                assert_eq!(group.requested.start, f[4].parse::<usize>().unwrap());
                assert_eq!(group.requested.end, f[5].parse::<usize>().unwrap());
                assert_eq!(group.polygons.len(), f[6].parse::<usize>().unwrap());
                assert_eq!(capture.group_by_id(&group.source.id), Some(index));
            }
            "POLYGON" => {
                let source: usize = f[2].parse().unwrap();
                let polygon: usize = f[3].parse().unwrap();
                let expected: Vec<_> = f[4..]
                    .chunks_exact(2)
                    .map(|p| {
                        [
                            p[0].parse::<f64>().unwrap() + 41.,
                            p[1].parse::<f64>().unwrap() + 23.,
                        ]
                    })
                    .collect();
                let actual = &capture.groups()[source].polygons[polygon];
                assert!(
                    polygon_matches(actual, &expected),
                    "case={case} source={source} polygon={polygon} actual={actual:?} original={expected:?}"
                );
                let area = capture.area(source, polygon).unwrap();
                assert_eq!(area.source_index(), source);
                assert_eq!(area.polygon_index(), polygon);
                assert!(!capture.scene().marks.iter().any(|m| m.id == area.id()));
                let commands = area.geometry().commands();
                assert!(matches!(commands.first(), Some(PathCommand::Move(_))));
                assert!(matches!(commands.last(), Some(PathCommand::Close)));
                assert_eq!(commands.len(), actual.len() + 1);
                let area_points: Vec<_> = commands[..commands.len() - 1]
                    .iter()
                    .enumerate()
                    .map(|(i, command)| match command {
                        PathCommand::Move(point) if i == 0 => [point.x, point.y],
                        PathCommand::Line(point) if i != 0 => [point.x, point.y],
                        _ => panic!("transparent area must retain the original polygon"),
                    })
                    .collect();
                assert!(polygon_matches(&area_points, &expected));
                polygons += 1;
            }
            _ => panic!("unknown original reference row"),
        }
    }
    assert_eq!(cases, 16);
    assert_eq!(cells, 192);
    assert!(polygons > 0);
}

#[test]
fn boundary_comparison_preserves_reversals_concavities_and_non_axis_bends() {
    let rect = [[0., 0.], [4., 0.], [4., 3.], [0., 3.]];
    assert!(polygon_matches(
        &rect,
        &[
            [0., 0.],
            [2., 0.],
            [4., 0.],
            [4., 3.],
            [0., 3.],
            [0., 3.],
            [0., 0.]
        ]
    ));
    assert!(!polygon_matches(
        &rect,
        &[[0., 0.], [4., 0.], [3., 0.], [4., 0.], [4., 3.], [0., 3.]]
    ));
    assert!(!polygon_matches(
        &rect,
        &[
            [0., 0.],
            [4., 0.],
            [4., 3.],
            [2., 3.],
            [2., 2.],
            [1., 2.],
            [1., 3.],
            [0., 3.]
        ]
    ));
    assert!(!polygon_matches(
        &rect,
        &[[0., 0.], [4., 0.], [4., 3.], [2., 2.], [0., 3.]]
    ));
}

#[test]
fn sealed_capture_preserves_legacy_area_mode_and_explicit_input_domain_boundaries() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<WaffleSvgFrame>();
    let c = config();
    let data = source("full");
    let o = options("top", "full");
    assert!(waffle_layout(&c, &data, &o).is_err());
    assert!(WaffleSvgFrame::new(&c, &data, &o).is_ok());
    assert!(
        WaffleSvgFrame::new(
            &c,
            &data,
            &WaffleLayoutOptions {
                allocation: WaffleAllocation::LargestRemainder,
                ..o.clone()
            }
        )
        .is_err()
    );
    let union = WaffleLayoutOptions {
        total: Some(32.),
        mode: WaffleMode::Areas,
        ..o
    };
    let legacy = waffle_layout(&c, &data, &union).unwrap();
    let captured = WaffleSvgFrame::new(&c, &data, &union).unwrap();
    assert_eq!(captured.layout(), &legacy);
    assert_eq!(captured.scene(), &legacy.scene);
    for cell in captured.cells() {
        assert_eq!(
            captured.cell_mark(cell.index).unwrap().is_none(),
            cell.owner.is_some()
        );
    }
    assert!(captured.cell_mark(12).is_err());
    assert!(
        captured
            .cell_for_mark(captured.scene().marks.len())
            .is_err()
    );
    assert!(captured.area(0, 0).is_err());
}
