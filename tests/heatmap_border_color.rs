use std::cell::{Cell, RefCell};

use glory::render::{GraphicStroke, StrokeCap};
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 320.,
        height: 180.,
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 20.,
            bottom: 20.,
        },
        ..Default::default()
    }
}
fn data() -> Vec<HeatCell> {
    vec![
        HeatCell {
            row: "r".into(),
            column: "a".into(),
            value: None,
        },
        HeatCell::new("r", "b", 0.),
        HeatCell::new("r", "c", 10.),
    ]
}
fn fixed_stroke() -> GraphicStroke {
    GraphicStroke {
        color: Color(70, 80, 90, 101),
        width: 2.5,
        cap: StrokeCap::Round,
    }
}
fn options() -> HeatmapLayoutOptions {
    HeatmapLayoutOptions {
        axes: HeatmapAxes::none(),
        border: Some(fixed_stroke()),
        hover_target: HeatmapHoverTarget::Cell,
        colors: ColorScaleOptions {
            domain: Some([0., 10.]),
            colors: vec![Color(20, 40, 60, 160), Color(200, 180, 160, 220)],
            unknown: Color(90, 100, 110, 64),
            ..Default::default()
        },
        ..Default::default()
    }
}
fn mark<'a>(layout: &'a HeatmapLayout, column: &str) -> &'a Mark {
    let id = HeatmapCellKey::new("r", column).mark_id();
    layout.scene.marks.iter().find(|m| m.id == id).unwrap()
}
fn darker() -> InheritedColor {
    InheritedColor::FromContext(vec![ColorModifier::Darker(1.)])
}
fn expected(index: usize) -> Color {
    [
        Color(63, 70, 77, 64),
        Color(14, 28, 42, 160),
        Color(140, 126, 112, 220),
    ][index]
}

#[test]
fn inherited_borders_use_actual_null_zero_numeric_palette_for_all_ordinary_shapes() {
    for shape in [HeatmapShape::Rect, HeatmapShape::Circle] {
        for radius in [0., 8.] {
            let o = HeatmapLayoutOptions {
                shape,
                corner_radius: radius,
                border_color_profile: Some(darker()),
                label_color_profile: Some(InheritedColor::Fixed(Color(4, 5, 6, 77))),
                ..options()
            };
            let l = heatmap_layout(&config(), &data(), &o).unwrap();
            for (i, column) in ["a", "b", "c"].into_iter().enumerate() {
                let m = mark(&l, column);
                assert_eq!(
                    m.stroke,
                    Some(GraphicStroke {
                        color: expected(i),
                        ..fixed_stroke()
                    })
                );
                assert_eq!(m.fill, Some(l.color_scale.map(data()[i].value)));
                assert_eq!(m.opacity, 1.);
            }
            assert_eq!(
                l.cells.iter().map(|c| c.value).collect::<Vec<_>>(),
                [None, Some(0.), Some(10.)]
            );
            assert_eq!(
                l.scene.labels.iter().map(|l| l.color).collect::<Vec<_>>(),
                vec![Color(4, 5, 6, 77); 3]
            );
        }
    }
}

#[test]
fn none_theme_fixed_and_absent_stroke_keep_their_existing_meanings() {
    for border in [None, Some(fixed_stroke())] {
        let o = HeatmapLayoutOptions {
            border,
            ..options()
        };
        let legacy = heatmap_layout(&config(), &data(), &o).unwrap();
        let themed = heatmap_layout(
            &config(),
            &data(),
            &HeatmapLayoutOptions {
                border_color_profile: Some(InheritedColor::Theme),
                ..o.clone()
            },
        )
        .unwrap();
        assert_eq!(legacy.scene, themed.scene);
        assert_eq!(legacy.cells, themed.cells);
        assert_eq!(
            render::to_svg(&legacy.scene).unwrap(),
            render::to_svg(&themed.scene).unwrap()
        );
        assert_eq!(
            render::to_canvas_script(&legacy.scene, "c").unwrap(),
            render::to_canvas_script(&themed.scene, "c").unwrap()
        );
        let fixed = heatmap_layout(
            &config(),
            &data(),
            &HeatmapLayoutOptions {
                border_color_profile: Some(InheritedColor::Fixed(Color(12, 34, 56, 78))),
                ..o
            },
        )
        .unwrap();
        for m in &fixed.scene.marks {
            assert_eq!(
                m.stroke,
                border.map(|s| GraphicStroke {
                    color: Color(12, 34, 56, 78),
                    ..s
                })
            );
        }
    }
}

#[test]
fn selected_color_profile_also_drives_border_inheritance() {
    let mut cells = data();
    cells.push(HeatCell::new("r", "mid", 5.));
    let o = HeatmapLayoutOptions {
        border_color_profile: Some(InheritedColor::FromContext(vec![])),
        ..options()
    };
    let l =
        heatmap_layout_with_color_profile(&config(), &cells, &o, ColorInterpolation::Lab).unwrap();
    for cell in &l.cells {
        let m = l.scene.marks.iter().find(|m| m.id == cell.mark_id).unwrap();
        assert_eq!(m.stroke.unwrap().color, l.color_scale.map(cell.value));
    }
    let rgb = heatmap_layout(&config(), &cells, &o).unwrap();
    assert_ne!(mark(&l, "mid").stroke, mark(&rgb, "mid").stroke);
}

#[test]
fn geometry_and_text_factories_receive_resolved_strokes_without_repainting_numeric_roles() {
    let events = RefCell::new(vec![]);
    let o = HeatmapLayoutOptions {
        column_order: Some(vec!["c".into(), "a".into(), "b".into()]),
        border_color_profile: Some(darker()),
        ..options()
    };
    let mut cell = |ctx: &HeatmapCellContext<'_>| {
        events.borrow_mut().push(('c', ctx.source_index));
        assert_eq!(
            ctx.stroke,
            Some(GraphicStroke {
                color: expected(ctx.source_index),
                ..fixed_stroke()
            })
        );
        assert_eq!(
            ctx.color,
            [
                Color(90, 100, 110, 64),
                Color(20, 40, 60, 160),
                Color(200, 180, 160, 220),
            ][ctx.source_index]
        );
        let mut ink = ctx.inherit(ctx.geometry.clone());
        ink.fill = Some(Color(255, 0, 255, 255));
        Ok(ink)
    };
    let mut label = |ctx: &HeatmapLabelContext<'_>| {
        events.borrow_mut().push(('l', ctx.source_index));
        Ok(format!("source:{}:{}", ctx.source_index, ctx.default_text))
    };
    let l = heatmap_layout_with_factories(
        &config(),
        &data(),
        &o,
        HeatmapFactories {
            cell: Some(&mut cell),
            label: Some(&mut label),
        },
    )
    .unwrap();
    assert_eq!(
        *events.borrow(),
        vec![('l', 2), ('c', 2), ('l', 0), ('c', 0), ('l', 1), ('c', 1)]
    );
    let binding = l.interaction_binding().unwrap();
    assert_eq!(
        binding.sites.iter().map(|s| s.value).collect::<Vec<_>>(),
        [Some(10.), None, Some(0.)]
    );
    assert!(
        binding
            .sites
            .iter()
            .all(|s| s.color == Color(255, 0, 255, 255))
    );
    assert_eq!(
        l.scene
            .labels
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>(),
        ["source:2:10", "source:0:missing", "source:1:0"]
    );
}

#[test]
fn degenerate_default_and_inherited_custom_cells_never_gain_a_border_or_fill() {
    for shape in [HeatmapShape::Rect, HeatmapShape::Circle] {
        let o = HeatmapLayoutOptions {
            shape,
            size_variation: Some(HeatmapSizeVariation {
                domain: Some([0., 10.]),
                sizes: [0., 1.],
            }),
            border_color_profile: Some(darker()),
            ..options()
        };
        let l = heatmap_layout(&config(), &data(), &o).unwrap();
        assert!(mark(&l, "b").stroke.is_none() && mark(&l, "b").fill.is_none());
        assert!(mark(&l, "a").stroke.is_some());
        let mut saw_zero = false;
        let custom = heatmap_layout_with_cell_factory(&config(), &data(), &o, |ctx| {
            if ctx.datum.value == Some(0.) {
                saw_zero = true;
                assert!(ctx.stroke.is_none() && ctx.fill.is_none());
            }
            Ok(ctx.inherit(ctx.geometry.clone()))
        })
        .unwrap();
        assert!(saw_zero);
        assert!(mark(&custom, "b").stroke.is_none() && mark(&custom, "b").fill.is_none());
        assert_eq!(
            custom
                .interaction_binding()
                .unwrap()
                .sites
                .iter()
                .map(|s| s.value)
                .collect::<Vec<_>>(),
            [None, Some(0.), Some(10.)]
        );
    }
}

#[test]
fn border_rgba_does_not_premultiply_cell_factory_or_hover_opacity_and_restores() {
    let o = HeatmapLayoutOptions {
        opacity: 0.8,
        active_opacity: 0.9,
        inactive_opacity: 0.2,
        border_color_profile: Some(InheritedColor::FromContext(vec![
            ColorModifier::Darker(1.),
            ColorModifier::Opacity(0.5),
        ])),
        label_color_profile: Some(InheritedColor::Fixed(Color(40, 50, 60, 101))),
        ..options()
    };
    let mut l = heatmap_layout_with_cell_factory(&config(), &data(), &o, |ctx| {
        let mut ink = ctx.inherit(ctx.geometry.clone());
        ink.opacity = 0.5;
        Ok(ink)
    })
    .unwrap();
    let base = l.scene.clone();
    let strokes = base.marks.iter().map(|m| m.stroke).collect::<Vec<_>>();
    assert!(
        base.marks
            .iter()
            .all(|m| m.stroke.unwrap().color.3 == 128 && m.opacity == 0.4)
    );
    assert!(base.labels.iter().all(|l| l.color == Color(40, 50, 60, 40)));
    for _ in 0..3 {
        l.set_active(Some(HeatmapCellKey::new("r", "a"))).unwrap();
        assert_eq!(
            l.scene.marks.iter().map(|m| m.stroke).collect::<Vec<_>>(),
            strokes
        );
        assert_eq!(
            l.scene.marks.iter().map(|m| m.opacity).collect::<Vec<_>>(),
            [0.45, 0.1, 0.1]
        );
        assert_eq!(
            l.scene.labels.iter().map(|l| l.color).collect::<Vec<_>>(),
            [
                Color(40, 50, 60, 45),
                Color(40, 50, 60, 10),
                Color(40, 50, 60, 10)
            ]
        );
        l.interaction_binding().unwrap();
        l.set_active(None).unwrap();
        assert_eq!(l.scene, base);
    }
}

#[test]
fn explicit_factory_stroke_overrides_still_freeze_and_tampering_rejects_atomically() {
    let o = HeatmapLayoutOptions {
        border_color_profile: Some(darker()),
        ..options()
    };
    let chosen = GraphicStroke {
        color: Color(1, 2, 3, 47),
        width: 4.,
        cap: StrokeCap::Butt,
    };
    let l = heatmap_layout_with_cell_factory(&config(), &data(), &o, |ctx| {
        let mut ink = ctx.inherit(ctx.geometry.clone());
        ink.stroke = Some(chosen);
        Ok(ink)
    })
    .unwrap();
    assert!(l.scene.marks.iter().all(|m| m.stroke == Some(chosen)));
    for corruption in 0..4 {
        let mut bad = l.clone();
        match corruption {
            0 => bad.scene.marks[0].stroke.as_mut().unwrap().color = Color(255, 0, 0, 255),
            1 => bad.scene.marks[0].stroke.as_mut().unwrap().width = 5.,
            2 => bad.cells[0].value = Some(0.),
            _ => bad.scene.labels[0].text = "forged".into(),
        }
        let before = bad.scene.clone();
        assert!(bad.interaction_binding().is_err());
        assert!(bad.set_active(Some(HeatmapCellKey::new("r", "a"))).is_err());
        assert_eq!(bad.scene, before);
    }
}

#[test]
fn intrinsic_errors_reject_before_any_factory_for_empty_hidden_disabled_and_zero_output() {
    for profile in [
        InheritedColor::FromContext(vec![ColorModifier::Darker(f64::NAN)]),
        InheritedColor::FromContext(vec![ColorModifier::Opacity(1.1)]),
        InheritedColor::FromContext(vec![ColorModifier::Brighter(1.); 33]),
    ] {
        for scenario in 0..6 {
            let cells = if scenario == 0 { vec![] } else { data() };
            let original = cells.clone();
            let mut o = HeatmapLayoutOptions {
                border_color_profile: Some(profile.clone()),
                ..options()
            };
            match scenario {
                1 => o.border = None,
                2 => {
                    o.hidden_rows.insert("r".into());
                }
                3 => {
                    o.hidden_columns.insert("a".into());
                }
                4 => o.labels = false,
                5 => {
                    o.size_variation = Some(HeatmapSizeVariation {
                        domain: Some([0., 10.]),
                        sizes: [0., 0.],
                    })
                }
                _ => {}
            }
            let calls = Cell::new(0);
            let mut cell = |ctx: &HeatmapCellContext<'_>| {
                calls.set(calls.get() + 1);
                Ok(ctx.inherit(ctx.geometry.clone()))
            };
            let mut label = |ctx: &HeatmapLabelContext<'_>| {
                calls.set(calls.get() + 1);
                Ok(ctx.default_text.to_owned())
            };
            assert!(
                heatmap_layout_with_factories(
                    &config(),
                    &cells,
                    &o,
                    HeatmapFactories {
                        cell: Some(&mut cell),
                        label: Some(&mut label)
                    }
                )
                .is_err()
            );
            assert_eq!(calls.get(), 0);
            assert_eq!(cells, original);
        }
    }
}

#[test]
fn actual_contrast_preflight_includes_hidden_null_and_borderless_source_but_not_absent_pairs() {
    let mut c = config();
    c.theme.background = Color(255, 255, 255, 0);
    for scenario in 0..4 {
        let mut o = HeatmapLayoutOptions {
            labels: false,
            border_color_profile: Some(InheritedColor::Contrast {
                dark: Color(0, 0, 0, 255),
                light: Color(255, 255, 255, 255),
            }),
            colors: ColorScaleOptions {
                domain: Some([0., 10.]),
                colors: vec![Color(20, 30, 40, 255), Color(220, 230, 240, 255)],
                unknown: Color(20, 30, 40, 0),
                ..Default::default()
            },
            ..options()
        };
        match scenario {
            1 => {
                o.hidden_columns.insert("a".into());
            }
            2 => o.border = None,
            3 => {
                o.size_variation = Some(HeatmapSizeVariation {
                    domain: Some([0., 10.]),
                    sizes: [0., 0.],
                })
            }
            _ => {}
        }
        let calls = Cell::new(0);
        assert!(
            heatmap_layout_with_cell_factory(&c, &data(), &o, |ctx| {
                calls.set(calls.get() + 1);
                Ok(ctx.inherit(ctx.geometry.clone()))
            })
            .is_err()
        );
        assert_eq!(calls.get(), 0);
        o.hidden_columns.clear();
        assert!(heatmap_layout(&c, &[], &o).is_ok());
        // Omitting the null source has no invented sparse contrast surface.
        assert!(heatmap_layout(&c, &data()[1..], &o).is_ok());
    }
}

#[test]
fn contrast_uses_original_palette_surface_and_source_budget_stays_checked() {
    let o = HeatmapLayoutOptions {
        border_color_profile: Some(InheritedColor::Contrast {
            dark: Color(0, 0, 0, 255),
            light: Color(255, 255, 255, 255),
        }),
        colors: ColorScaleOptions {
            domain: Some([0., 10.]),
            colors: vec![Color(0, 0, 0, 255), Color(255, 255, 255, 255)],
            unknown: Color(0, 0, 0, 255),
            ..Default::default()
        },
        ..options()
    };
    let l = heatmap_layout_with_cell_factory(&config(), &data(), &o, |ctx| {
        let mut ink = ctx.inherit(ctx.geometry.clone());
        ink.fill = Some(Color(255, 0, 255, 255));
        Ok(ink)
    })
    .unwrap();
    assert_eq!(
        l.scene
            .marks
            .iter()
            .map(|m| m.stroke.unwrap().color)
            .collect::<Vec<_>>(),
        [
            Color(255, 255, 255, 255),
            Color(255, 255, 255, 255),
            Color(0, 0, 0, 255)
        ]
    );
    let oversized = vec![HeatCell::new("r", "a", 0.); MAX_HEATMAP_CELLS + 1];
    let calls = Cell::new(0);
    let error = heatmap_layout_with_cell_factory(&config(), &oversized, &o, |ctx| {
        calls.set(calls.get() + 1);
        Ok(ctx.inherit(ctx.geometry.clone()))
    })
    .unwrap_err();
    assert!(error.0.contains("cell budget"));
    assert_eq!(calls.get(), 0);
}
