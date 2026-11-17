use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 320.,
        height: 240.,
        margin: Margin {
            left: 23.,
            right: 17.,
            top: 19.,
            bottom: 21.,
        },
        ..Default::default()
    }
}

fn directions() -> [WaffleFillDirection; 4] {
    [
        WaffleFillDirection::Top,
        WaffleFillDirection::Right,
        WaffleFillDirection::Bottom,
        WaffleFillDirection::Left,
    ]
}

fn source() -> Vec<RadialDatum> {
    vec![
        RadialDatum::new("a", 34.),
        RadialDatum::new("b", 16.),
        RadialDatum::new("hidden", 20.),
        RadialDatum::new("zero", -0.),
    ]
}

fn options(mode: WaffleMode, direction: WaffleFillDirection) -> WaffleLayoutOptions {
    WaffleLayoutOptions {
        rows: 2,
        columns: 3,
        total: Some(100.),
        mode,
        direction,
        hidden: ["hidden".into()].into(),
        border_width: 3.,
        padding: 5.,
        ..Default::default()
    }
}

fn fill(index: usize) -> Color {
    Color(40 + index as u8 * 30, 100, 180, 255)
}
fn border(index: usize) -> Color {
    Color(210, 40 + index as u8, 10, 255)
}
fn formatted(value: f64) -> String {
    format!("raw:{:016x}", value.to_bits())
}

#[test]
fn empty_and_identity_factories_preserve_legacy_structure_and_svg_bytes() {
    // Independent rounding deliberately requests seven cells from a six-slot grid.
    let data = [13., 13., -0., 13., 13., 48.].map(|value| RadialDatum::new("", value));
    let data: Vec<_> = data
        .into_iter()
        .enumerate()
        .map(|(i, mut d)| {
            d.id = format!("g{i}");
            d.label = d.id.clone();
            d
        })
        .collect();
    for direction in directions() {
        for mode in [WaffleMode::Cells, WaffleMode::Areas] {
            for allocation in [
                WaffleAllocation::IndependentRound,
                WaffleAllocation::LargestRemainder,
            ] {
                for hidden in [false, true] {
                    let o = WaffleLayoutOptions {
                        rows: 2,
                        columns: 3,
                        total: Some(100.),
                        direction,
                        mode,
                        allocation,
                        hidden: if hidden {
                            ["g1".into()].into()
                        } else {
                            Default::default()
                        },
                        border_width: 2.,
                        corner_radius: 4.,
                        ..Default::default()
                    };
                    let old = waffle_layout(&config(), &data, &o).unwrap();
                    let empty = waffle_layout_with_factories(
                        &config(),
                        &data,
                        &o,
                        &mut WaffleFactories::default(),
                    )
                    .unwrap();
                    assert_eq!(empty, old);
                    assert_eq!(
                        render::to_svg(&empty.scene).unwrap().as_bytes(),
                        render::to_svg(&old.scene).unwrap().as_bytes()
                    );
                    let color_calls = Cell::new(0);
                    let border_calls = Cell::new(0);
                    let mut color = |c: &WaffleGroupColorContext<'_>| {
                        color_calls.set(color_calls.get() + 1);
                        Ok(c.default_color)
                    };
                    let mut border = |c: &WaffleBorderColorContext<'_>| {
                        border_calls.set(border_calls.get() + 1);
                        Ok(c.default_color.unwrap())
                    };
                    let identity = waffle_layout_with_factories(
                        &config(),
                        &data,
                        &o,
                        &mut WaffleFactories {
                            color: Some(&mut color),
                            border_color: Some(&mut border),
                            ..Default::default()
                        },
                    )
                    .unwrap();
                    assert_eq!(
                        color_calls.get(),
                        data.len(),
                        "include hidden, zero and overflow groups"
                    );
                    assert_eq!(border_calls.get(), data.len() + 1);
                    assert_eq!(identity, old);
                    assert_eq!(identity.groups[2].source.value.to_bits(), (-0f64).to_bits());
                    if allocation == WaffleAllocation::IndependentRound && !hidden {
                        assert_eq!(identity.groups.last().unwrap().requested, 4..7);
                        assert_eq!(identity.groups.last().unwrap().occupied, 4..6);
                    }
                }
            }
        }
    }
}

#[test]
fn raw_source_order_and_pre_polygon_border_context_are_observable() {
    let data = source();
    let original = data.clone();
    let o = options(WaffleMode::Areas, WaffleFillDirection::Left);
    let trace = RefCell::new(Vec::new());
    let mut color = |c: &WaffleGroupColorContext<'_>| {
        assert_eq!(c.datum, &data[c.source_index]);
        assert_eq!(
            c.datum.value.to_bits(),
            original[c.source_index].value.to_bits()
        );
        assert_eq!(c.plot, ClipRect::plot(&config()));
        assert_eq!(c.total, 100.);
        assert_eq!(c.unit, 100. / 6.);
        trace.borrow_mut().push(format!("color:{}", c.source_index));
        Ok(fill(c.source_index))
    };
    let mut format = |v: f64| {
        trace
            .borrow_mut()
            .push(format!("format:{:016x}", v.to_bits()));
        Ok(formatted(v))
    };
    let mut border_fn = |c: &WaffleBorderColorContext<'_>| match c.target {
        WaffleBorderTarget::Group(g) => {
            trace
                .borrow_mut()
                .push(format!("border:{}", g.source_index));
            assert_eq!(
                g.source.value.to_bits(),
                data[g.source_index].value.to_bits()
            );
            assert_eq!(g.color, fill(g.source_index));
            assert_eq!(
                g.border_color, g.color,
                "temporary border is the resolved fill"
            );
            assert_eq!(g.formatted_value, formatted(g.source.value));
            assert!(g.polygons.is_empty());
            assert!(g.area_mark_id.is_none());
            let expected = [0..2, 2..3, 3..3, 3..3];
            assert_eq!(g.requested, expected[g.source_index]);
            assert_eq!(g.occupied, expected[g.source_index]);
            assert_eq!(g.hidden, g.source_index == 2);
            Ok(border(g.source_index))
        }
        WaffleBorderTarget::Empty(style) => {
            trace.borrow_mut().push("empty".into());
            assert_eq!(style.color, o.empty_color);
            assert_eq!(style.opacity, o.empty_opacity);
            Ok(Color(12, 34, 56, 255))
        }
    };
    let layout = waffle_layout_with_factories(
        &config(),
        &data,
        &o,
        &mut WaffleFactories {
            color: Some(&mut color),
            formatted_value: Some(&mut format),
            border_color: Some(&mut border_fn),
        },
    )
    .unwrap();
    let expected: Vec<_> = data
        .iter()
        .enumerate()
        .flat_map(|(i, d)| {
            [
                format!("color:{i}"),
                format!("format:{:016x}", d.value.to_bits()),
                format!("border:{i}"),
            ]
        })
        .chain(std::iter::once("empty".into()))
        .collect();
    assert_eq!(*trace.borrow(), expected);
    assert_eq!(data, original);
    assert_eq!(data[3].value.to_bits(), (-0f64).to_bits());
    assert_eq!(layout.groups[0].polygons.len(), 1);
    assert!(layout.groups[2].polygons.is_empty());
}

#[test]
fn resolved_styles_reach_ink_legends_sites_widgets_regions_and_custom_cells_once() {
    for direction in directions() {
        for mode in [WaffleMode::Cells, WaffleMode::Areas] {
            let data = source();
            let o = options(mode, direction);
            let counts = Cell::new([0usize; 3]);
            let mut color = |c: &WaffleGroupColorContext<'_>| {
                let mut n = counts.get();
                n[0] += 1;
                counts.set(n);
                Ok(fill(c.source_index))
            };
            let mut format = |v| {
                let mut n = counts.get();
                n[1] += 1;
                counts.set(n);
                Ok(formatted(v))
            };
            let mut border_fn = |c: &WaffleBorderColorContext<'_>| {
                let mut n = counts.get();
                n[2] += 1;
                counts.set(n);
                Ok(match c.target {
                    WaffleBorderTarget::Group(g) => border(g.source_index),
                    WaffleBorderTarget::Empty(_) => Color(12, 34, 56, 255),
                })
            };
            let mut visited = Vec::new();
            let mut cell_fn = |c: &WaffleCellContext<'_>| {
                assert_eq!(counts.get(), [4, 4, 5], "styles finish before custom ink");
                let expected_fill = c.group.map_or(o.empty_color, |g| fill(g.source_index));
                let expected_border = c
                    .group
                    .map_or(Color(12, 34, 56, 255), |g| border(g.source_index));
                assert_eq!(c.fill, Some(expected_fill));
                assert_eq!(c.stroke.unwrap().color, expected_border);
                visited.push(c.cell.index);
                Ok(c.inherit(c.geometry.clone()))
            };
            let layout = waffle_layout_with_factories_and_cell_factory(
                &config(),
                &data,
                &o,
                &mut WaffleFactories {
                    color: Some(&mut color),
                    formatted_value: Some(&mut format),
                    border_color: Some(&mut border_fn),
                },
                &mut cell_fn,
            )
            .unwrap();
            assert_eq!(counts.get(), [4, 4, 5]);
            assert_eq!(
                visited,
                layout
                    .cells
                    .iter()
                    .filter(|c| c.mark_id.is_some())
                    .map(|c| c.index)
                    .collect::<Vec<_>>()
            );
            for g in &layout.groups {
                assert_eq!(g.color, fill(g.source_index));
                assert_eq!(g.border_color, border(g.source_index));
                assert_eq!(g.formatted_value, formatted(g.source.value));
                let legend = layout
                    .scene
                    .legend
                    .iter()
                    .find(|l| l.id == g.source.id)
                    .unwrap();
                assert_eq!(legend.color, g.color);
            }
            for mark in &layout.scene.marks {
                let group = layout.groups.iter().find(|g| g.source.id == mark.series);
                assert_eq!(mark.fill, Some(group.map_or(o.empty_color, |g| g.color)));
                assert_eq!(
                    mark.stroke.unwrap().color,
                    group.map_or(Color(12, 34, 56, 255), |g| g.border_color)
                );
            }
            for site in &layout.interaction.sites {
                let g = layout
                    .groups
                    .iter()
                    .find(|g| g.source.id == site.series)
                    .unwrap();
                assert_eq!(site.color, g.color);
                assert_eq!(site.value.unwrap().to_bits(), g.source.value.to_bits());
                assert_eq!(
                    site.formatted_value.as_deref(),
                    Some(g.formatted_value.as_str())
                );
            }
            let packet = waffle_cell_widgets(&layout, 9, |c| {
                let b = c.cell.bounds;
                Ok(Some(PlotWidgetPlacement::Rect(LayoutRect {
                    x: b.x,
                    y: b.y,
                    width: b.width,
                    height: b.height,
                })))
            })
            .unwrap();
            let mut shared = None;
            for row in packet.data.values() {
                if let Some(g) = &row.group {
                    assert_eq!(g.as_ref(), &layout.groups[g.source_index]);
                    if g.source_index == 0 {
                        if let Some(first) = &shared {
                            assert!(Arc::ptr_eq(first, g));
                        } else {
                            shared = Some(g.clone());
                        }
                    }
                }
            }
            if mode == WaffleMode::Cells {
                assert!(shared.is_some());
            }
            let frame = waffle_area_interaction(&layout).unwrap();
            assert_eq!(frame.scene, layout.scene);
            let binding = frame.interaction.unwrap();
            assert_eq!(binding.sites.len(), 2);
            assert_eq!(binding.regions.len(), 2);
            for site in binding.sites {
                let g = layout
                    .groups
                    .iter()
                    .find(|g| g.source.id == site.series)
                    .unwrap();
                assert_eq!(site.color, g.color);
                assert_eq!(
                    site.formatted_value.as_deref(),
                    Some(g.formatted_value.as_str())
                );
            }
            assert_eq!(
                counts.get(),
                [4, 4, 5],
                "consumers never replay source factories"
            );
        }
    }
}

#[test]
fn empty_template_runs_once_even_without_any_empty_cell_or_any_group() {
    for data in [vec![RadialDatum::new("full", 100.)], vec![]] {
        let calls = RefCell::new(Vec::new());
        let mut border = |c: &WaffleBorderColorContext<'_>| {
            calls.borrow_mut().push(match c.target {
                WaffleBorderTarget::Group(_) => "group",
                WaffleBorderTarget::Empty(_) => "empty",
            });
            Ok(Color(1, 2, 3, 255))
        };
        waffle_layout_with_factories(
            &config(),
            &data,
            &options(WaffleMode::Cells, WaffleFillDirection::Top),
            &mut WaffleFactories {
                border_color: Some(&mut border),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            *calls.borrow(),
            if data.is_empty() {
                vec!["empty"]
            } else {
                vec!["group", "empty"]
            }
        );
    }
}

#[test]
fn invalid_source_config_and_grid_are_rejected_before_any_factory() {
    for case in 0..9 {
        let mut c = config();
        let mut data = source();
        let mut o = options(WaffleMode::Cells, WaffleFillDirection::Bottom);
        match case {
            0 => c.width = f64::NAN,
            1 => data[0].value = f64::NAN,
            2 => data[1].id = data[0].id.clone(),
            3 => o.rows = 0,
            4 => {
                o.rows = usize::MAX;
                o.columns = 2;
            }
            5 => {
                o.rows = MAX_MARKS + 1;
                o.columns = 1;
            }
            6 => o.padding = 1e10,
            7 => o.corner_radius = f64::INFINITY,
            8 => {
                // The config and grid budget are valid, but three slots cannot
                // be represented at this large x within a one-ULP plot width.
                c.margin.left = f64::from_bits(c.width.to_bits() - 1);
                c.margin.right = 0.;
                o.rows = 1;
                o.columns = 3;
                o.padding = 0.;
                c.validate().unwrap();
            }
            _ => unreachable!(),
        }
        let bits: Vec<_> = data
            .iter()
            .map(|d| (d.id.clone(), d.value.to_bits()))
            .collect();
        let calls = Cell::new(0);
        let mut color = |_: &WaffleGroupColorContext<'_>| {
            calls.set(calls.get() + 1);
            Ok(Color(1, 2, 3, 255))
        };
        let mut format = |_: f64| {
            calls.set(calls.get() + 1);
            Ok("x".into())
        };
        let mut border = |_: &WaffleBorderColorContext<'_>| {
            calls.set(calls.get() + 1);
            Ok(Color(1, 2, 3, 255))
        };
        let error = waffle_layout_with_factories(
            &c,
            &data,
            &o,
            &mut WaffleFactories {
                color: Some(&mut color),
                formatted_value: Some(&mut format),
                border_color: Some(&mut border),
            },
        )
        .unwrap_err();
        if case == 8 {
            assert_eq!(error.0, "Waffle slot projection collapsed");
        }
        assert_eq!(calls.get(), 0, "case {case}");
        assert_eq!(
            data.iter()
                .map(|d| (d.id.clone(), d.value.to_bits()))
                .collect::<Vec<_>>(),
            bits
        );
    }
}

#[test]
fn callback_error_and_utf8_byte_budget_return_no_candidate_or_input_mutation() {
    let data = source();
    let before = data.clone();
    let mut boundary = |_: f64| Ok("a".repeat(MAX_LABEL_BYTES));
    let accepted = waffle_layout_with_factories(
        &config(),
        &data,
        &WaffleLayoutOptions::default(),
        &mut WaffleFactories {
            formatted_value: Some(&mut boundary),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        accepted
            .groups
            .iter()
            .all(|g| g.formatted_value.len() == MAX_LABEL_BYTES)
    );
    let bytes = "界".repeat(MAX_LABEL_BYTES / 3 + 1);
    assert!(bytes.chars().count() < MAX_LABEL_BYTES && bytes.len() > MAX_LABEL_BYTES);
    let border_calls = Cell::new(0);
    let mut format = |_: f64| Ok(bytes.clone());
    let mut border = |_: &WaffleBorderColorContext<'_>| {
        border_calls.set(border_calls.get() + 1);
        Ok(Color(1, 2, 3, 255))
    };
    assert!(
        waffle_layout_with_factories(
            &config(),
            &data,
            &WaffleLayoutOptions::default(),
            &mut WaffleFactories {
                formatted_value: Some(&mut format),
                border_color: Some(&mut border),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(border_calls.get(), 0);
    let seen = Rc::new(RefCell::new(Vec::new()));
    let log = seen.clone();
    let mut color = move |c: &WaffleGroupColorContext<'_>| {
        log.borrow_mut().push(c.source_index);
        if c.source_index == 1 {
            Err(ChartError("authored color refusal".into()))
        } else {
            Ok(fill(c.source_index))
        }
    };
    let error = waffle_layout_with_factories(
        &config(),
        &data,
        &WaffleLayoutOptions::default(),
        &mut WaffleFactories {
            color: Some(&mut color),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(error.0, "authored color refusal");
    assert_eq!(
        *seen.borrow(),
        [0, 1],
        "earlier external callback effects are not rolled back"
    );
    assert_eq!(data, before);
    assert_eq!(data[3].value.to_bits(), (-0f64).to_bits());
}

#[test]
fn explicit_border_factory_can_handle_an_unresolvable_transparent_default() {
    let mut c = config();
    c.theme.background = Color(0, 0, 0, 0);
    let o = WaffleLayoutOptions {
        rows: 1,
        columns: 2,
        total: Some(100.),
        empty_color: Color(2, 3, 4, 0),
        border_color: InheritedColor::Contrast {
            dark: Color(0, 0, 0, 255),
            light: Color(255, 255, 255, 255),
        },
        ..Default::default()
    };
    let data = [RadialDatum::new("a", 50.)];
    let mut color = |_: &WaffleGroupColorContext<'_>| Ok(Color(10, 20, 30, 0));
    assert!(
        waffle_layout_with_factories(
            &c,
            &data,
            &o,
            &mut WaffleFactories {
                color: Some(&mut color),
                ..Default::default()
            }
        )
        .is_err()
    );
    let mut targets = Vec::new();
    let mut border = |c: &WaffleBorderColorContext<'_>| {
        assert_eq!(c.default_color, None);
        targets.push(matches!(c.target, WaffleBorderTarget::Empty(_)));
        Ok(Color(12, 34, 56, 255))
    };
    let layout = waffle_layout_with_factories(
        &c,
        &data,
        &o,
        &mut WaffleFactories {
            color: Some(&mut color),
            border_color: Some(&mut border),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(targets, [false, true]);
    assert_eq!(layout.groups[0].border_color, Color(12, 34, 56, 255));
    assert!(
        waffle_layout(&c, &data, &o).is_err(),
        "legacy empty default remains invalid"
    );
}
