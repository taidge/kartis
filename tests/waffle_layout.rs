use std::collections::{BTreeMap, BTreeSet};

use kartis::surface::{SurfaceInput, SurfaceModel, SurfaceOptions};
use kartis::*;

fn frame() -> ChartConfig {
    ChartConfig {
        width: 284.,
        height: 202.,
        margin: Margin {
            left: 22.,
            right: 14.,
            top: 17.,
            bottom: 11.,
        },
        ..Default::default()
    }
}

fn point_in_cell(cell: &WaffleCell) -> [f64; 2] {
    [
        cell.bounds.x + cell.bounds.width / 2.,
        cell.bounds.y + cell.bounds.height / 2.,
    ]
}

#[test]
fn waffle_matches_nivo_grid_order_origin_and_legend_in_all_directions() {
    // Original Nivo grid.ts, using 248×174 logical inner dimensions and
    // three rows by four columns. Its plot-relative x origin is 8.
    use WaffleFillDirection::*;
    let profiles = [
        (Top, [(2, 3), (2, 2), (2, 1), (2, 0), (1, 3)]),
        (Right, [(0, 0), (1, 0), (2, 0), (0, 1), (1, 1)]),
        (Bottom, [(0, 0), (0, 1), (0, 2), (0, 3), (1, 0)]),
        (Left, [(2, 3), (1, 3), (0, 3), (2, 2), (1, 2)]),
    ];
    let data = [RadialDatum::new("one", 27.), RadialDatum::new("two", 34.)];
    let mut identities = None;
    for (direction, expected) in profiles {
        let layout = waffle_layout(
            &frame(),
            &data,
            &WaffleLayoutOptions {
                rows: 3,
                columns: 4,
                total: Some(100.),
                direction,
                padding: 6.,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(layout.cell_size, 58.);
        assert_eq!(
            layout.grid,
            ClipRect {
                x: 30.,
                y: 17.,
                width: 232.,
                height: 174.
            }
        );
        assert_eq!(layout.cells.len(), 12);
        assert_eq!(layout.groups[0].occupied, 0..3);
        assert_eq!(layout.groups[1].occupied, 3..7);
        assert_eq!(
            layout
                .cells
                .iter()
                .map(|c| (c.row, c.column))
                .take(5)
                .collect::<Vec<_>>(),
            expected
        );
        let mut ids = BTreeMap::new();
        for (i, cell) in layout.cells.iter().enumerate() {
            let [x, y] = point_in_cell(cell);
            assert_eq!(layout.cell_at(x, y), Some(cell));
            assert_eq!(cell.index, i);
            assert_eq!(
                layout.scene.marks[i].geometry,
                Geometry::Rect {
                    x: cell.bounds.x + 3.,
                    y: cell.bounds.y + 3.,
                    width: 52.,
                    height: 52.,
                }
            );
            ids.insert((cell.row, cell.column), cell.mark_id.clone());
        }
        if let Some(prior) = &identities {
            assert_eq!(&ids, prior);
        }
        identities = Some(ids);
        let legend = layout
            .scene
            .legend
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            legend,
            if matches!(direction, Top | Left) {
                vec!["two", "one"]
            } else {
                vec!["one", "two"]
            }
        );
        assert!(
            layout
                .cell_at(layout.grid.x + layout.grid.width, layout.grid.y)
                .is_none()
        );
        assert!(
            layout
                .cell_at(layout.grid.x, layout.grid.y + layout.grid.height)
                .is_none()
        );
        assert!(layout.cell_at(f64::NAN, layout.grid.y).is_none());
        assert_eq!(
            layout
                .cell_at(layout.grid.x + 58., layout.grid.y)
                .unwrap()
                .column,
            1
        );
    }
}

#[test]
fn waffle_independent_rounding_exposes_overflow_and_largest_remainder_is_opt_in() {
    let data = [1., 1., 1., 1., 1., 3.]
        .into_iter()
        .enumerate()
        .map(|(i, value)| RadialDatum::new(i.to_string(), value))
        .collect::<Vec<_>>();
    let options = WaffleLayoutOptions {
        rows: 2,
        columns: 2,
        total: Some(8.),
        ..Default::default()
    };
    let rounded = waffle_layout(&frame(), &data, &options).unwrap();
    assert_eq!(rounded.unit, 2.);
    assert_eq!(
        rounded
            .groups
            .iter()
            .map(|g| g.requested.clone())
            .collect::<Vec<_>>(),
        [0..1, 1..2, 2..3, 3..4, 4..5, 5..7]
    );
    assert_eq!(
        rounded
            .groups
            .iter()
            .map(|g| g.occupied.clone())
            .collect::<Vec<_>>(),
        [0..1, 1..2, 2..3, 3..4, 4..4, 4..4]
    );
    let apportioned = waffle_layout(
        &frame(),
        &data,
        &WaffleLayoutOptions {
            allocation: WaffleAllocation::LargestRemainder,
            ..options.clone()
        },
    )
    .unwrap();
    assert_eq!(
        apportioned
            .cells
            .iter()
            .map(|c| c.owner)
            .collect::<Vec<_>>(),
        [Some(0), Some(1), Some(2), Some(5)]
    );

    let hidden = waffle_layout(
        &frame(),
        &data,
        &WaffleLayoutOptions {
            hidden: BTreeSet::from(["0".into(), "1".into(), "2".into()]),
            ..options
        },
    )
    .unwrap();
    assert!(hidden.groups[0].hidden);
    assert_eq!(hidden.groups[2].requested, 0..0);
    assert_eq!(
        hidden.cells.iter().map(|c| c.owner).collect::<Vec<_>>(),
        [Some(3), Some(4), Some(5), Some(5)]
    );
    assert_eq!(hidden.cells[0].mark_id, rounded.cells[0].mark_id);
    assert_eq!(hidden.groups[3].color, rounded.groups[3].color);
    assert_eq!(hidden.scene.legend, rounded.scene.legend);
}

fn surface(layout: &WaffleLayout) -> SurfaceModel {
    let candidate = layout.clone().into_scene_interaction();
    candidate.validate().unwrap();
    SurfaceModel::new_with_interaction(
        candidate.scene,
        SurfaceOptions {
            plot: candidate.plot,
            ..Default::default()
        },
        candidate.interaction,
    )
    .unwrap()
}

#[test]
fn waffle_round_cells_keep_full_slot_lookup_and_exact_ink_distinct() {
    let options = WaffleLayoutOptions {
        rows: 2,
        columns: 2,
        total: Some(4.),
        corner_radius: 12.,
        padding: 8.,
        value_format: ValueFormat::Fixed(2),
        border_width: 2.,
        empty_opacity: 0.25,
        ..Default::default()
    };
    let data = [RadialDatum {
        id: "A".into(),
        label: "Alpha".into(),
        value: 2.,
    }];
    let layout = waffle_layout(&frame(), &data, &options).unwrap();
    let cell = &layout.cells[0];
    let mut surface = surface(&layout);
    let [x, y] = point_in_cell(cell);
    surface
        .apply(SurfaceInput::Move {
            id: 1,
            point: (x, y),
        })
        .unwrap();
    assert_eq!(surface.hovered(), [cell.mark_id.as_deref().unwrap()]);
    let active = surface.active_interaction(surface.hovered()).unwrap();
    assert_eq!(active.rows[0].value, Some(2.));
    assert_eq!(active.rows[0].formatted_value.as_deref(), Some("2.00"));
    assert_eq!(active.rows[0].series, "A");
    assert_eq!(active.rows[0].position, [x, y]);
    let corner = (cell.bounds.x + 4.2, cell.bounds.y + 4.2);
    surface
        .apply(SurfaceInput::Move {
            id: 1,
            point: corner,
        })
        .unwrap();
    assert!(surface.hovered().is_empty());
    assert_eq!(layout.area_at(corner.0, corner.1).unwrap().source, data[0]);
    assert_eq!(
        layout.scene.marks[0].stroke.unwrap().color,
        layout.groups[0].border_color
    );
    assert_eq!(layout.scene.marks[2].opacity, 0.25);
    assert!(layout.scene.marks[2].label.is_empty());
}

#[test]
fn waffle_area_union_preserves_diagonal_components_and_picks_real_group_ink() {
    let data = [
        RadialDatum::new("first", 1.),
        RadialDatum::new("target", 3.),
    ];
    let layout = waffle_layout(
        &frame(),
        &data,
        &WaffleLayoutOptions {
            rows: 2,
            columns: 3,
            total: Some(6.),
            direction: WaffleFillDirection::Bottom,
            mode: WaffleMode::Areas,
            padding: 12.,
            ..Default::default()
        },
    )
    .unwrap();
    let group = &layout.groups[1];
    let x = layout.grid.x;
    let y = layout.grid.y;
    let p = layout.cell_size;
    assert_eq!(
        group.polygons,
        [
            vec![
                [x + 3. * p, y],
                [x + 3. * p, y + p],
                [x + p, y + p],
                [x + p, y]
            ],
            vec![
                [x + p, y + p],
                [x + p, y + 2. * p],
                [x, y + 2. * p],
                [x, y + p]
            ],
        ]
    );
    assert_eq!(layout.scene.marks.len(), 4);
    assert!(layout.cells[1].mark_id.is_none());
    assert_eq!(group.area_mark_id.as_deref(), Some("waffle-area:target"));
    assert!(
        render::to_svg(&layout.scene)
            .unwrap()
            .contains("data-id=\"waffle-area:target\"")
    );
    assert!(
        render::to_canvas_script(&layout.scene, "waffle")
            .unwrap()
            .contains("lineTo")
    );
    let mut surface = surface(&layout);
    let seam = (x + 2. * p, y + p / 2.);
    surface
        .apply(SurfaceInput::Move { id: 1, point: seam })
        .unwrap();
    assert_eq!(surface.hovered(), ["waffle-area:target"]);
    assert_eq!(
        surface.active_interaction(surface.hovered()).unwrap().rows[0].value,
        Some(3.)
    );
    surface
        .apply(SurfaceInput::Move {
            id: 1,
            point: (x + 1.5 * p, y + 1.5 * p),
        })
        .unwrap();
    assert!(surface.hovered().is_empty());
}

#[test]
fn waffle_group_areas_match_occupied_slots_across_each_kind_of_wrap() {
    for direction in [
        WaffleFillDirection::Top,
        WaffleFillDirection::Right,
        WaffleFillDirection::Bottom,
        WaffleFillDirection::Left,
    ] {
        for start in 0..12 {
            for count in [1, (12 - start).min(5)] {
                let data = [
                    RadialDatum::new("previous", start as f64),
                    RadialDatum::new("test", count as f64),
                ];
                let layout = waffle_layout(
                    &frame(),
                    &data,
                    &WaffleLayoutOptions {
                        rows: 3,
                        columns: 4,
                        total: Some(12.),
                        direction,
                        mode: WaffleMode::Areas,
                        ..Default::default()
                    },
                )
                .unwrap();
                let group = &layout.groups[1];
                let mut surface = surface(&layout);
                for cell in &layout.cells {
                    let [x, y] = point_in_cell(cell);
                    surface
                        .apply(SurfaceInput::Move {
                            id: 1,
                            point: (x, y),
                        })
                        .unwrap();
                    let in_group = surface.hovered().first() == group.area_mark_id.as_ref();
                    assert_eq!(in_group, group.occupied.contains(&cell.index));
                }
                let polygon_area = group
                    .polygons
                    .iter()
                    .map(|polygon| {
                        polygon
                            .iter()
                            .zip(polygon.iter().cycle().skip(1))
                            .map(|([x0, y0], [x1, y1])| x0 * y1 - y0 * x1)
                            .sum::<f64>()
                            / 2.
                    })
                    .sum::<f64>();
                assert!((polygon_area - count as f64 * layout.cell_size.powi(2)).abs() < 1e-8);
            }
        }
    }
}

#[test]
fn waffle_validates_layout_budget_total_and_style_before_admitting_a_scene() {
    let data = [RadialDatum::new("A", 1.)];
    let initial = WaffleLayoutOptions {
        rows: 2,
        columns: 2,
        ..Default::default()
    };
    for options in [
        WaffleLayoutOptions {
            rows: usize::MAX,
            ..initial.clone()
        },
        WaffleLayoutOptions {
            columns: 0,
            ..initial.clone()
        },
        WaffleLayoutOptions {
            padding: f64::NAN,
            ..initial.clone()
        },
        WaffleLayoutOptions {
            padding: 87.,
            ..initial.clone()
        },
        WaffleLayoutOptions {
            corner_radius: -1.,
            ..initial.clone()
        },
        WaffleLayoutOptions {
            empty_opacity: 1.1,
            ..initial.clone()
        },
        WaffleLayoutOptions {
            border_width: 1e-7,
            ..initial.clone()
        },
        WaffleLayoutOptions {
            total: Some(0.5),
            ..initial.clone()
        },
    ] {
        assert!(waffle_layout(&frame(), &data, &options).is_err());
    }
    assert!(
        waffle_layout(
            &frame(),
            &[RadialDatum::new("A", f64::from_bits(1))],
            &initial
        )
        .is_err()
    );
    assert!(waffle_layout(&frame(), &[data[0].clone(), data[0].clone()], &initial).is_err());
    let empty = waffle_layout(&frame(), &[], &initial).unwrap();
    assert_eq!(empty.total, 1.);
    assert_eq!(empty.scene.marks.len(), 4);
    assert!(empty.interaction.sites.is_empty());
    assert!(empty.area_at(empty.grid.x, empty.grid.y).is_none());
}
