use std::collections::BTreeSet;

use glory::render::PathCommand;
use kartis::*;
fn config() -> ChartConfig {
    ChartConfig {
        width: 420.,
        height: 260.,
        margin: Margin {
            left: 10.,
            right: 10.,
            top: 10.,
            bottom: 10.,
        },
        ..Default::default()
    }
}
fn data(case: usize) -> Vec<AreaBumpSeries> {
    match case {
        0 => vec![
            AreaBumpSeries::new(
                "A",
                vec![
                    AreaBumpDatum::new(0, 6.),
                    AreaBumpDatum::new(1, 2.),
                    AreaBumpDatum::new(2, 9.),
                ],
            ),
            AreaBumpSeries::new(
                "B",
                vec![
                    AreaBumpDatum::new(0, 2.),
                    AreaBumpDatum::new(1, 8.),
                    AreaBumpDatum::new(2, 3.),
                ],
            ),
            AreaBumpSeries::new(
                "C",
                vec![
                    AreaBumpDatum::new(0, 0.),
                    AreaBumpDatum::new(1, 2.),
                    AreaBumpDatum::new(2, 0.),
                ],
            ),
        ],
        1 => vec![
            AreaBumpSeries::new(
                "A",
                vec![
                    AreaBumpDatum::new(0, 4.),
                    AreaBumpDatum::new("0", 2.),
                    AreaBumpDatum::new(2, 5.),
                ],
            ),
            AreaBumpSeries::new(
                "B",
                vec![
                    AreaBumpDatum::new(0, 3.),
                    AreaBumpDatum::new("0", 8.),
                    AreaBumpDatum::new(2, 2.),
                ],
            ),
        ],
        _ => vec![
            AreaBumpSeries::new(
                "A",
                vec![
                    AreaBumpDatum::new("left", 6.),
                    AreaBumpDatum::new("right", 2.),
                ],
            ),
            AreaBumpSeries::new(
                "B",
                vec![
                    AreaBumpDatum::new("left", 2.),
                    AreaBumpDatum::new("middle", 8.),
                    AreaBumpDatum::new("right", 4.),
                ],
            ),
            AreaBumpSeries::new(
                "C",
                vec![
                    AreaBumpDatum::new("middle", 4.),
                    AreaBumpDatum::new("right", 0.),
                ],
            ),
        ],
    }
}
fn options(align: &str, spacing: f64, padding: f64) -> AreaBumpOptions {
    AreaBumpOptions {
        align: match align {
            "start" => AreaBumpAlign::Start,
            "end" => AreaBumpAlign::End,
            _ => AreaBumpAlign::Middle,
        },
        spacing,
        x_padding: padding,
        ..Default::default()
    }
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}
#[test]
fn actual_nivo_compute_oracle_matches_396_present_sample_positions_and_thicknesses() {
    let mut count = 0;
    for row in include_str!("fixtures/area-bump-nivo.csv").lines().skip(2) {
        let f = row.split('|').collect::<Vec<_>>();
        let case = f[0].parse().unwrap();
        let num = |i: usize| f[i].parse::<f64>().unwrap();
        let l = layout_area_bump(&config(), &data(case), &options(f[1], num(2), num(3))).unwrap();
        let cell = l
            .cells
            .iter()
            .find(|p| p.series_id == f[4] && p.source_index == f[5].parse::<usize>().unwrap())
            .unwrap();
        near(cell.x_pixel, num(8) + 10.);
        near(cell.y0, num(9) + 10.);
        near(cell.y1, num(10) + 10.);
        count += 1;
    }
    assert_eq!(count, 396);
}
fn command_values(p: &PathCommand) -> (char, Vec<f64>) {
    match p {
        PathCommand::Move(p) => ('M', vec![p.x, p.y]),
        PathCommand::Line(p) => ('L', vec![p.x, p.y]),
        PathCommand::Cubic { first, second, end } => (
            'C',
            vec![first.x, first.y, second.x, second.y, end.x, end.y],
        ),
        PathCommand::Close => ('Z', vec![]),
    }
}
fn parse_path(s: &str) -> Vec<(char, Vec<f64>)> {
    let bytes = s.as_bytes();
    let mut i = 0;
    let mut out = vec![];
    while i < bytes.len() {
        if bytes[i].is_ascii_whitespace() || bytes[i] == b',' {
            i += 1;
            continue;
        }
        let kind = bytes[i] as char;
        i += 1;
        let count = match kind {
            'M' | 'L' => 2,
            'C' => 6,
            'Z' => 0,
            _ => panic!("unexpected command {kind}"),
        };
        let mut values = vec![];
        for _ in 0..count {
            while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b',') {
                i += 1;
            }
            let start = i;
            if i < bytes.len() && (bytes[i] == b'-' || bytes[i] == b'+') {
                i += 1;
            }
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
                i += 1;
                if i < bytes.len() && (bytes[i] == b'-' || bytes[i] == b'+') {
                    i += 1;
                }
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
            values.push(s[start..i].parse::<f64>().unwrap() + 10.);
        }
        out.push((kind, values));
    }
    out
}
#[test]
fn exact_nivo_area_points_and_d3_linear_basis_paths_match_180_whole_closed_bands() {
    let mut count = 0;
    for row in include_str!("fixtures/area-bump-paths.csv").lines().skip(2) {
        let f = row.split('|').collect::<Vec<_>>();
        let mut o = options(f[1], f[2].parse().unwrap(), f[3].parse().unwrap());
        o.interpolation = if f[5] == "linear" {
            AreaBumpInterpolation::Linear
        } else {
            AreaBumpInterpolation::Smooth
        };
        let l = layout_area_bump(&config(), &data(f[0].parse().unwrap()), &o).unwrap();
        let mark = l.scene.marks.iter().find(|m| m.series == f[4]).unwrap();
        let Geometry::Path(path) = &mark.geometry else {
            panic!()
        };
        let actual = path.iter().map(command_values).collect::<Vec<_>>();
        let expected = parse_path(f[6]);
        assert_eq!(actual.len(), expected.len(), "{row}");
        for (a, b) in actual.iter().zip(expected) {
            assert_eq!(a.0, b.0, "{row}");
            for (x, y) in a.1.iter().zip(b.1) {
                near(*x, y);
            }
        }
        count += 1;
    }
    assert_eq!(count, 180);
}
#[test]
fn rank_ties_and_thickness_conservation_use_one_common_scale() {
    let l = layout_area_bump(
        &config(),
        &data(0),
        &AreaBumpOptions {
            spacing: 4.,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(l.max_sum, 12.);
    assert_eq!(l.available_height, 228.);
    let second = &l.slices[1];
    assert_eq!(
        second
            .cell_indices
            .iter()
            .map(|i| l.cells[*i].series_id.as_str())
            .collect::<Vec<_>>(),
        ["B", "A", "C"]
    );
    for slice in &l.slices {
        let height = slice
            .cell_indices
            .iter()
            .map(|i| l.cells[*i].y1 - l.cells[*i].y0)
            .sum::<f64>();
        near(height, slice.total / l.max_sum * l.available_height);
        for pair in slice.cell_indices.windows(2) {
            near(l.cells[pair[1]].y0 - l.cells[pair[0]].y1, 4.);
        }
    }
    let mut reversed = data(0);
    reversed.reverse();
    let o = AreaBumpOptions {
        tie_break: AreaBumpTieBreak::SeriesId,
        ..Default::default()
    };
    let a = layout_area_bump(&config(), &data(0), &o).unwrap();
    let b = layout_area_bump(&config(), &reversed, &o).unwrap();
    for cell in &a.cells {
        let other = b
            .cells
            .iter()
            .find(|c| c.series_id == cell.series_id && c.x_key == cell.x_key)
            .unwrap();
        assert_eq!(cell.rank, other.rank);
        assert_eq!(
            (cell.x_pixel, cell.y0, cell.y1),
            (other.x_pixel, other.y0, other.y1)
        );
    }
}
#[test]
fn numeric_string_and_sparse_columns_remain_distinct_and_gap_policy_is_explicit() {
    let l = layout_area_bump(&config(), &data(1), &Default::default()).unwrap();
    assert_ne!(l.slices[0].key, l.slices[1].key);
    assert_eq!(l.slices.len(), 3);
    let mut o = AreaBumpOptions {
        x_order: Some(
            ["left", "middle", "right", "empty"]
                .map(AreaBumpX::from)
                .to_vec(),
        ),
        ..Default::default()
    };
    let connected = layout_area_bump(&config(), &data(2), &o).unwrap();
    assert_eq!(connected.series[0].mark_ids.len(), 1);
    assert!(connected.slices[3].cell_indices.is_empty());
    assert_eq!(connected.slices[3].total, 0.);
    assert_eq!(
        connected.series[1]
            .cell_indices
            .iter()
            .map(|i| connected.cells[*i].source_index)
            .collect::<Vec<_>>(),
        [0, 1, 2]
    );
    o.missing = AreaBumpMissing::Gap;
    let gap = layout_area_bump(&config(), &data(2), &o).unwrap();
    assert!(gap.series[0].mark_ids.is_empty());
    assert_eq!(gap.series[1].mark_ids.len(), 1);
    assert_eq!(connected.cells, gap.cells);
    assert!(gap.nearest_slice(f64::NAN).is_none());
    assert_eq!(
        gap.nearest_slice(gap.slices[1].x_pixel).unwrap().key,
        gap.slices[1].key
    );
}
#[test]
fn focus_is_transactional_independent_paint_and_keeps_appended_scene_items() {
    let mut o = AreaBumpOptions::default();
    o.styles.insert(
        "A".into(),
        AreaBumpSeriesStyle {
            color: Some(Color(200, 0, 0, 255)),
            fill_opacity: 0.2,
            border_color: Some(Color(0, 0, 0, 255)),
            border_opacity: 0.8,
            border_width: 4.,
            ..Default::default()
        },
    );
    let mut l = layout_area_bump(&config(), &data(0), &o).unwrap();
    let original = l.scene.clone();
    let a = l.scene.marks.iter().find(|m| m.series == "A").unwrap();
    assert_eq!(a.fill, Some(Color(200, 0, 0, 51)));
    assert_eq!(a.stroke.unwrap().color.3, 204);
    assert_eq!(a.opacity, 1.);
    l.scene.marks.push(Mark::new(
        "app",
        Geometry::Circle {
            x: 200.,
            y: 5.,
            radius: 1.,
        },
        Color(1, 2, 3, 255),
    ));
    l.set_active(BTreeSet::from(["A".into()])).unwrap();
    let a = l.scene.marks.iter().find(|m| m.series == "A").unwrap();
    assert_eq!(a.fill, Some(Color(200, 0, 0, 255)));
    assert_eq!(a.stroke.unwrap().width, 1.);
    assert!(l.scene.marks.iter().any(|m| m.id == "app"));
    for (m, n) in original.marks.iter().zip(&l.scene.marks) {
        assert_eq!(m.geometry, n.geometry);
        assert_eq!(m.id, n.id);
    }
    let accepted = l.clone();
    assert!(l.set_active(BTreeSet::from(["unknown".into()])).is_err());
    assert_eq!(l, accepted);
    l.set_active(BTreeSet::new()).unwrap();
    assert_eq!(l.scene.marks[original.marks.len()].id, "app");
    assert_eq!(&l.scene.marks[..original.marks.len()], original.marks);
}
#[test]
fn hidden_relayouts_and_zero_columns_do_not_create_phantom_bands() {
    let old = layout_area_bump(&config(), &data(0), &Default::default()).unwrap();
    let l = layout_area_bump(
        &config(),
        &data(0),
        &AreaBumpOptions {
            hidden: BTreeSet::from(["A".into()]),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(l.slices.len(), old.slices.len());
    assert_eq!(l.max_sum, 10.);
    assert!(l.scene.marks.iter().all(|m| m.series != "A"));
    assert!(l.scene.label_series.values().all(|s| s != "A"));
    assert!(l.scene.legend.iter().any(|v| v.id == "A"));
    assert!(l.cells.iter().all(|p| p.series_id != "A"));
    let zero = [AreaBumpSeries::new(
        "zero",
        vec![AreaBumpDatum::new(0, 0.), AreaBumpDatum::new(1, 0.)],
    )];
    let l = layout_area_bump(&config(), &zero, &Default::default()).unwrap();
    assert_eq!(l.max_sum, 0.);
    assert!(l.cells.iter().all(|p| p.y0 == p.y1));
    assert!(l.series[0].mark_ids.is_empty());
}
#[test]
fn budgets_extremes_and_invalid_inputs_reject_before_any_caller_mutation() {
    let original = data(0);
    for case in 0..8 {
        let mut rows = original.clone();
        let mut o = AreaBumpOptions::default();
        match case {
            0 => rows[0].data[0].value = -1.,
            1 => rows[0].data[0].value = f64::NAN,
            2 => {
                let p = rows[0].data[0].clone();
                rows[0].data.push(p);
            }
            3 => o.spacing = 1000.,
            4 => o.x_order = Some(vec![AreaBumpX::from(0)]),
            5 => o.active_ids.insert("unknown".into()).then_some(()).unwrap(),
            6 => o.style.border_width = f64::INFINITY,
            _ => o.start_labels.rotation = f64::NAN,
        };
        assert!(layout_area_bump(&config(), &rows, &o).is_err(), "{case}");
    }
    assert_eq!(original, data(0));
    let mut oversized = AreaBumpOptions {
        x_order: Some(vec![AreaBumpX::from("reserved"); 4097]),
        ..Default::default()
    };
    assert!(
        layout_area_bump(&config(), &[], &oversized)
            .unwrap_err()
            .0
            .contains("budget")
    );
    oversized.x_order = None;
    oversized.styles = (0..257)
        .map(|i| (i.to_string(), AreaBumpSeriesStyle::default()))
        .collect();
    assert!(
        layout_area_bump(&config(), &[], &oversized)
            .unwrap_err()
            .0
            .contains("style budget")
    );
    let mut grid = AreaBumpOptions {
        grid: false,
        ..Default::default()
    };
    grid.axis_top.as_mut().unwrap().options.grid = true;
    let grid = layout_area_bump(&config(), &original, &grid).unwrap();
    assert_eq!(
        grid.scene
            .marks
            .iter()
            .filter(|m| m.id.starts_with("area-bump-grid:"))
            .count(),
        3
    );
    let rows = [AreaBumpSeries::new(
        "zero",
        vec![AreaBumpDatum::new(-0., 1.), AreaBumpDatum::new(0., 2.)],
    )];
    assert!(layout_area_bump(&config(), &rows, &Default::default()).is_err());
    let huge = [
        AreaBumpSeries::new("A", vec![AreaBumpDatum::new(0, f64::MAX)]),
        AreaBumpSeries::new("B", vec![AreaBumpDatum::new(0, f64::MAX)]),
    ];
    assert!(layout_area_bump(&config(), &huge, &Default::default()).is_err());
    let dense = (0..32)
        .map(|s| {
            AreaBumpSeries::new(
                format!("s{s}"),
                (0..512)
                    .map(|x| AreaBumpDatum::new(x, 1. + ((x + s) % 7) as f64))
                    .collect(),
            )
        })
        .collect::<Vec<_>>();
    let l = layout_area_bump(
        &config(),
        &dense,
        &AreaBumpOptions {
            axis_top: None,
            axis_bottom: None,
            grid: false,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(l.cells.len(), 16384);
    l.scene.validate().unwrap();
    let mut too_many = dense;
    too_many[0].data.push(AreaBumpDatum::new(9999, 1.));
    assert!(layout_area_bump(&config(), &too_many, &Default::default()).is_err());
}
#[cfg(feature = "raster")]
#[test]
fn cpu_pixels_show_rank_crossings_positive_thickness_and_real_spacing_gaps() {
    use render::raster::*;
    let rows = [
        AreaBumpSeries::new(
            "A",
            vec![
                AreaBumpDatum::new(0, 8.),
                AreaBumpDatum::new(1, 2.),
                AreaBumpDatum::new(2, 8.),
            ],
        ),
        AreaBumpSeries::new(
            "B",
            vec![
                AreaBumpDatum::new(0, 2.),
                AreaBumpDatum::new(1, 8.),
                AreaBumpDatum::new(2, 2.),
            ],
        ),
    ];
    let mut o = AreaBumpOptions {
        axis_top: None,
        axis_bottom: None,
        grid: false,
        interpolation: AreaBumpInterpolation::Linear,
        x_padding: 0.,
        spacing: 4.,
        ..Default::default()
    };
    o.start_labels.enabled = false;
    o.end_labels.enabled = false;
    o.style.fill_opacity = 1.;
    o.style.border_width = 0.;
    o.styles.insert(
        "A".into(),
        AreaBumpSeriesStyle {
            color: Some(Color(255, 0, 0, 255)),
            ..o.style.clone()
        },
    );
    o.styles.insert(
        "B".into(),
        AreaBumpSeriesStyle {
            color: Some(Color(0, 0, 255, 255)),
            ..o.style.clone()
        },
    );
    let l = layout_area_bump(&config(), &rows, &o).unwrap();
    let image = render_raster(&l.scene, &FontRegistry::new(), &Default::default()).unwrap();
    let pixel = |x: usize, y: usize| &image.rgba[(y * 420 + x) * 4..(y * 420 + x) * 4 + 4];
    assert_eq!(pixel(30, 50), [255, 0, 0, 255]);
    assert_eq!(pixel(210, 50), [0, 0, 255, 255]);
    assert_eq!(pixel(210, 230), [255, 0, 0, 255]);
    assert_eq!(pixel(210, 199), [255; 4]);
}
