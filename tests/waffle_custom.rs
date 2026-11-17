use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::surface::{Picking, SurfaceInput, SurfaceModel, SurfaceOptions};
use kartis::*;

fn data() -> Vec<RadialDatum> {
    vec![RadialDatum::new("a", 4.), RadialDatum::new("b", 2.)]
}
fn options() -> WaffleLayoutOptions {
    WaffleLayoutOptions {
        rows: 2,
        columns: 4,
        total: Some(10.),
        ..Default::default()
    }
}

#[test]
fn identity_is_full_layout_equivalent_for_every_mode_direction_and_allocation() {
    for direction in [
        WaffleFillDirection::Top,
        WaffleFillDirection::Right,
        WaffleFillDirection::Bottom,
        WaffleFillDirection::Left,
    ] {
        for allocation in [
            WaffleAllocation::IndependentRound,
            WaffleAllocation::LargestRemainder,
        ] {
            for mode in [WaffleMode::Cells, WaffleMode::Areas] {
                let mut o = options();
                o.direction = direction;
                o.allocation = allocation;
                o.mode = mode;
                o.corner_radius = 4.;
                o.border_width = 1.;
                o.empty_opacity = 0.4;
                o.hidden.insert("a".into());
                let base = waffle_layout(&ChartConfig::default(), &data(), &o).unwrap();
                let mut visited = Vec::new();
                let custom = waffle_layout_with_cell_factory(
                    &ChartConfig::default(),
                    &data(),
                    &o,
                    &mut |ctx| {
                        visited.push(ctx.cell.index);
                        if let Some(group) = ctx.group {
                            assert_eq!(ctx.cell.owner, Some(group.source_index));
                            assert!(!group.hidden);
                        }
                        Ok(ctx.inherit(ctx.geometry.clone()))
                    },
                )
                .unwrap();
                assert_eq!(
                    visited,
                    base.cells
                        .iter()
                        .filter(|c| c.mark_id.is_some())
                        .map(|c| c.index)
                        .collect::<Vec<_>>()
                );
                assert_eq!(custom, base);
            }
        }
    }
}

#[test]
fn area_mode_only_customizes_empty_cells_and_keeps_group_union_and_legend() {
    let o = WaffleLayoutOptions {
        mode: WaffleMode::Areas,
        ..options()
    };
    let base = waffle_layout(&ChartConfig::default(), &data(), &o).unwrap();
    let custom =
        waffle_layout_with_cell_factory(&ChartConfig::default(), &data(), &o, &mut |ctx| {
            assert!(ctx.group.is_none());
            let mut value = ctx.inherit(Geometry::Circle {
                x: ctx.position[0],
                y: ctx.position[1],
                radius: 3.,
            });
            value.fill = Some(Color(255, 0, 0, 255));
            Ok(value)
        })
        .unwrap();
    assert_eq!(base.groups, custom.groups);
    assert_eq!(base.interaction, custom.interaction);
    assert_eq!(base.scene.legend, custom.scene.legend);
    for group in &base.groups {
        if let Some(id) = &group.area_mark_id {
            assert_eq!(
                base.scene.marks.iter().find(|m| &m.id == id),
                custom.scene.marks.iter().find(|m| &m.id == id)
            );
        }
    }
}

#[test]
fn offset_ink_keeps_full_slot_queries_raw_group_and_original_nearest_anchor() {
    let c = ChartConfig::default();
    let o = WaffleLayoutOptions {
        rows: 1,
        columns: 1,
        ..Default::default()
    };
    let source = [RadialDatum::new("a", 1.)];
    let base = waffle_layout(&c, &source, &o).unwrap();
    let p = base.interaction.sites[0].position;
    let out = waffle_layout_with_cell_factory(&c, &source, &o, &mut |ctx| {
        let [x, y] = ctx.position;
        let mut value = ctx.inherit(Geometry::Path(vec![
            C::Move(P::new(x + 20., y - 5.)),
            C::Line(P::new(x + 30., y - 5.)),
            C::Line(P::new(x + 30., y + 5.)),
            C::Line(P::new(x + 20., y + 5.)),
            C::Close,
        ]));
        value.fill = Some(Color(250, 10, 30, 255));
        value.stroke = None;
        Ok(value)
    })
    .unwrap();
    assert_eq!(out.groups, base.groups);
    assert_eq!(out.cells, base.cells);
    assert_eq!(out.scene.legend, base.scene.legend);
    assert_eq!(out.interaction.sites[0].position, p);
    assert_eq!(out.interaction.sites[0].value, Some(1.));
    assert_eq!(out.interaction.sites[0].color, Color(250, 10, 30, 255));
    assert_eq!(out.cell_at(p[0], p[1]), base.cell_at(p[0], p[1]));
    for (picking, point, expected) in [
        (Picking::Geometry, p, false),
        (Picking::Geometry, [p[0] + 25., p[1]], true),
        (Picking::Nearest { distance: 1. }, p, true),
        (Picking::Nearest { distance: 1. }, [p[0] + 25., p[1]], false),
    ] {
        let mut surface = SurfaceModel::new_with_interaction(
            out.scene.clone(),
            SurfaceOptions {
                plot: Some(ClipRect::plot(&c)),
                picking,
                ..Default::default()
            },
            Some(out.interaction.clone()),
        )
        .unwrap();
        surface
            .apply(SurfaceInput::Move {
                id: 1,
                point: (point[0], point[1]),
            })
            .unwrap();
        assert_eq!(!surface.hovered().is_empty(), expected);
    }
}

#[test]
fn empty_ink_never_creates_typed_data_sites() {
    let out =
        waffle_layout_with_cell_factory(&ChartConfig::default(), &[], &options(), &mut |ctx| {
            assert!(ctx.group.is_none());
            Ok(ctx.inherit(Geometry::Path(vec![
                C::Move(P::new(ctx.position[0] + 1., ctx.position[1])),
                C::Line(P::new(ctx.position[0] + 3., ctx.position[1] + 2.)),
                C::Line(P::new(ctx.position[0] + 3., ctx.position[1] - 2.)),
                C::Close,
            ])))
        })
        .unwrap();
    assert!(out.interaction.sites.is_empty() && out.interaction.anchors.is_empty());
    assert!(
        out.scene
            .marks
            .iter()
            .all(|m| m.label.is_empty() && m.series.is_empty())
    );
}

#[test]
fn invalid_geometry_opacity_and_callback_errors_reject_without_source_mutation() {
    let c = ChartConfig::default();
    let d = data();
    let old = waffle_layout(&c, &d, &options()).unwrap();
    for invalid in [f64::NAN, -1., 1.1] {
        assert!(
            waffle_layout_with_cell_factory(&c, &d, &options(), &mut |ctx| {
                let mut value = ctx.inherit(ctx.geometry.clone());
                value.opacity = invalid;
                Ok(value)
            })
            .is_err()
        );
    }
    assert!(
        waffle_layout_with_cell_factory(&c, &d, &options(), &mut |ctx| Ok(ctx.inherit(
            Geometry::Circle {
                x: ctx.position[0] + 1.,
                y: ctx.position[1],
                radius: 2.
            }
        )))
        .is_err()
    );
    assert!(
        waffle_layout_with_cell_factory(&c, &d, &options(), &mut |_| Err(ChartError(
            "caller rejected".into()
        )))
        .unwrap_err()
        .0
        .contains("caller rejected")
    );
    assert_eq!(old, waffle_layout(&c, &d, &options()).unwrap());
}

#[test]
fn command_budget_counts_final_cells_and_rejects_over_limit() {
    let c = ChartConfig::default();
    let o = WaffleLayoutOptions {
        rows: 1,
        columns: 25,
        corner_radius: 3.,
        ..Default::default()
    };
    let source = [RadialDatum::new("a", 1.)];
    for (last, valid) in [(8_000, true), (8_001, false)] {
        let out = waffle_layout_with_cell_factory(&c, &source, &o, &mut |ctx| {
            let count = if ctx.cell.index == 24 { last } else { 8_000 };
            let mut path = vec![C::Move(P::new(ctx.position[0], ctx.position[1]))];
            path.extend(
                (1..count).map(|_| C::Line(P::new(ctx.position[0] + 0.1, ctx.position[1]))),
            );
            Ok(ctx.inherit(Geometry::Path(path)))
        });
        assert_eq!(out.is_ok(), valid, "{out:?}");
    }
}

#[test]
fn final_gradient_budget_accepts_exact_limit_and_rejects_the_next_cell() {
    let c = ChartConfig::default();
    let o = WaffleLayoutOptions {
        rows: 1,
        columns: 33,
        ..Default::default()
    };
    let source = [RadialDatum::new("a", 1.)];
    let stops: Vec<_> = (0..512)
        .map(|i| GradientStop::new(f64::from(i) / 511., Color(30, 60, 100, 255)))
        .collect();
    for (gradient_cells, valid) in [(32, true), (33, false)] {
        let out = waffle_layout_with_cell_factory(&c, &source, &o, &mut |ctx| {
            let mut value = ctx.inherit(ctx.geometry.clone());
            if ctx.cell.index < gradient_cells {
                value.fill_paint = Some(FillPaint::Linear {
                    start: [0., 0.],
                    end: [600., 0.],
                    stops: stops.clone(),
                });
            }
            Ok(value)
        });
        assert_eq!(out.is_ok(), valid);
        if let Err(error) = out {
            assert!(error.0.contains("paint budget"));
        }
    }
}
