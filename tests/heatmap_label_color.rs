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
fn options() -> HeatmapLayoutOptions {
    HeatmapLayoutOptions {
        axes: HeatmapAxes::none(),
        hover_target: HeatmapHoverTarget::Cell,
        colors: ColorScaleOptions {
            domain: Some([0., 10.]),
            colors: vec![Color(20, 40, 60, 160), Color(200, 180, 160, 220)],
            unknown: Color(90, 100, 110, 64),
            ..Default::default()
        },
        empty_label: "no data".into(),
        ..Default::default()
    }
}
fn labels(layout: &HeatmapLayout) -> Vec<Color> {
    layout.scene.labels.iter().map(|l| l.color).collect()
}

#[test]
fn actual_original_nivo_generator_fixture_matches_nonzero_alpha_roles() {
    fn color(s: &str) -> Color {
        let v = s
            .split(':')
            .map(|v| v.parse::<u8>().unwrap())
            .collect::<Vec<_>>();
        Color(v[0], v[1], v[2], v[3])
    }
    for line in include_str!("fixtures/heatmap-label-color-nivo.csv").lines() {
        let parts = line.split('|').collect::<Vec<_>>();
        let source = color(parts[1]);
        let profile = match parts[0] {
            "fixed" => InheritedColor::Fixed(Color(12, 34, 56, 78)),
            "theme" => InheritedColor::Theme,
            "darker" => InheritedColor::FromContext(vec![ColorModifier::Darker(1.)]),
            "ordered" => InheritedColor::FromContext(vec![
                ColorModifier::Darker(1.),
                ColorModifier::Brighter(1.),
                ColorModifier::Opacity(0.5),
            ]),
            _ => panic!("unknown fixture profile"),
        };
        let o = HeatmapLayoutOptions {
            label_color_profile: Some(profile),
            label_style: HeatmapTextStyle {
                color: Some(Color(70, 80, 90, 123)),
                ..Default::default()
            },
            colors: ColorScaleOptions {
                domain: Some([0., 1.]),
                colors: vec![source, source],
                unknown: source,
                ..Default::default()
            },
            ..options()
        };
        let layout = heatmap_layout(&config(), &data(), &o).unwrap();
        assert_eq!(labels(&layout), vec![color(parts[2]); 3], "{line}");
    }
}

#[test]
fn opt_in_theme_preserves_existing_role_color_and_fixed_overrides_it() {
    let mut c = config();
    c.theme.text = Color(1, 2, 3, 91);
    for style in [None, Some(Color(70, 80, 90, 123))] {
        let o = HeatmapLayoutOptions {
            label_style: HeatmapTextStyle {
                color: style,
                ..Default::default()
            },
            ..options()
        };
        let old = heatmap_layout(&c, &data(), &o).unwrap();
        let themed = heatmap_layout(
            &c,
            &data(),
            &HeatmapLayoutOptions {
                label_color_profile: Some(InheritedColor::Theme),
                ..o.clone()
            },
        )
        .unwrap();
        assert_eq!(old.scene, themed.scene);
        assert_eq!(labels(&themed), vec![style.unwrap_or(c.theme.text); 3]);
        let fixed = heatmap_layout(
            &c,
            &data(),
            &HeatmapLayoutOptions {
                label_color_profile: Some(InheritedColor::Fixed(Color(12, 34, 56, 78))),
                ..o
            },
        )
        .unwrap();
        assert_eq!(labels(&fixed), vec![Color(12, 34, 56, 78); 3]);
    }
}

#[test]
fn inherited_data_palette_including_null_unknown_resolves_before_factory_paint() {
    let o = HeatmapLayoutOptions {
        label_color_profile: Some(InheritedColor::FromContext(vec![ColorModifier::Darker(1.)])),
        ..options()
    };
    let mut observed = vec![];
    let layout = heatmap_layout_with_cell_factory(&config(), &data(), &o, |ctx| {
        observed.push((ctx.datum.value, ctx.color));
        let mut ink = ctx.inherit(ctx.geometry.clone());
        ink.fill = Some(Color(255, 0, 255, 255));
        Ok(ink)
    })
    .unwrap();
    assert_eq!(
        observed,
        vec![
            (None, Color(90, 100, 110, 64)),
            (Some(0.), Color(20, 40, 60, 160)),
            (Some(10.), Color(200, 180, 160, 220))
        ]
    );
    assert_eq!(
        labels(&layout),
        vec![
            Color(63, 70, 77, 64),
            Color(14, 28, 42, 160),
            Color(140, 126, 112, 220)
        ]
    );
    assert!(
        layout
            .cells
            .iter()
            .all(|c| c.color == Color(255, 0, 255, 255))
    );
    assert_eq!(layout.scene.labels[0].text, "no data");
    assert_eq!(
        layout
            .interaction_binding()
            .unwrap()
            .sites
            .iter()
            .map(|s| s.value)
            .collect::<Vec<_>>(),
        vec![None, Some(0.), Some(10.)]
    );
}

#[test]
fn ordered_modifiers_use_shared_alpha_replacement_semantics() {
    let o = HeatmapLayoutOptions {
        opacity: 0.5,
        label_color_profile: Some(InheritedColor::FromContext(vec![
            ColorModifier::Darker(1.),
            ColorModifier::Brighter(1.),
            ColorModifier::Opacity(0.5),
        ])),
        ..options()
    };
    let layout = heatmap_layout(&config(), &data(), &o).unwrap();
    assert_eq!(
        labels(&layout),
        vec![
            Color(90, 100, 110, 64),
            Color(20, 40, 60, 64),
            Color(200, 180, 160, 64)
        ]
    );
}

#[test]
fn profile_alpha_multiplies_factory_and_controlled_opacity_once_and_restores() {
    let o = HeatmapLayoutOptions {
        opacity: 0.8,
        active_opacity: 0.9,
        inactive_opacity: 0.2,
        label_color_profile: Some(InheritedColor::Fixed(Color(40, 50, 60, 101))),
        ..options()
    };
    let mut layout = heatmap_layout_with_cell_factory(&config(), &data(), &o, |ctx| {
        let mut ink = ctx.inherit(ctx.geometry.clone());
        ink.opacity = 0.5;
        Ok(ink)
    })
    .unwrap();
    let base = layout.scene.clone();
    assert_eq!(labels(&layout), vec![Color(40, 50, 60, 40); 3]);
    for _ in 0..3 {
        layout
            .set_active(Some(HeatmapCellKey::new("r", "a")))
            .unwrap();
        assert_eq!(
            labels(&layout),
            vec![
                Color(40, 50, 60, 45),
                Color(40, 50, 60, 10),
                Color(40, 50, 60, 10)
            ]
        );
        assert_eq!(
            layout.cells.iter().map(|c| c.opacity).collect::<Vec<_>>(),
            vec![0.45, 0.1, 0.1]
        );
        layout.interaction_binding().unwrap();
        layout.set_active(None).unwrap();
        assert_eq!(layout.scene, base);
    }
}

#[test]
fn contrast_uses_original_palette_surface_not_custom_paint_representative() {
    let o = HeatmapLayoutOptions {
        label_color_profile: Some(InheritedColor::Contrast {
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
    let layout = heatmap_layout_with_cell_factory(&config(), &data(), &o, |ctx| {
        let mut ink = ctx.inherit(ctx.geometry.clone());
        ink.fill = Some(if ctx.color == Color(0, 0, 0, 255) {
            Color(255, 255, 255, 255)
        } else {
            Color(0, 0, 0, 255)
        });
        Ok(ink)
    })
    .unwrap();
    assert_eq!(
        labels(&layout),
        vec![
            Color(255, 255, 255, 255),
            Color(255, 255, 255, 255),
            Color(0, 0, 0, 255)
        ]
    );
    assert_eq!(
        layout.cells.iter().map(|c| c.color).collect::<Vec<_>>(),
        vec![
            Color(255, 255, 255, 255),
            Color(255, 255, 255, 255),
            Color(0, 0, 0, 255)
        ]
    );
}

#[test]
fn invalid_profiles_reject_even_empty_hidden_and_disabled_labels_before_callbacks() {
    let invalid = [
        InheritedColor::FromContext(vec![ColorModifier::Darker(f64::NAN)]),
        InheritedColor::FromContext(vec![ColorModifier::Opacity(1.1)]),
        InheritedColor::FromContext(vec![ColorModifier::Brighter(1.); 33]),
    ];
    for profile in invalid {
        for scenario in 0..4 {
            let cells = if scenario == 0 { vec![] } else { data() };
            let original = cells.clone();
            let mut o = HeatmapLayoutOptions {
                label_color_profile: Some(profile.clone()),
                ..options()
            };
            if scenario == 1 {
                o.labels = false;
            }
            if scenario == 2 {
                o.hidden_rows.insert("r".into());
            }
            if scenario == 3 {
                o.hidden_columns.insert("a".into());
                o.labels = false;
            }
            let mut calls = 0;
            assert!(
                heatmap_layout_with_cell_factory(&config(), &cells, &o, |ctx| {
                    calls += 1;
                    Ok(ctx.inherit(ctx.geometry.clone()))
                })
                .is_err()
            );
            assert_eq!(calls, 0);
            assert_eq!(cells, original);
        }
    }
}

#[test]
fn actual_invalid_contrast_context_is_preflighted_including_hidden_nulls() {
    let mut c = config();
    c.theme.background = Color(255, 255, 255, 0);
    let cells = vec![
        HeatCell::new("r", "a", 0.),
        HeatCell {
            row: "r".into(),
            column: "z".into(),
            value: None,
        },
    ];
    for hidden in [false, true] {
        let mut o = HeatmapLayoutOptions {
            labels: false,
            label_color_profile: Some(InheritedColor::Contrast {
                dark: Color(0, 0, 0, 255),
                light: Color(255, 255, 255, 255),
            }),
            colors: ColorScaleOptions {
                colors: vec![Color(20, 30, 40, 255), Color(220, 230, 240, 255)],
                unknown: Color(20, 30, 40, 0),
                ..Default::default()
            },
            ..options()
        };
        if hidden {
            o.hidden_columns.insert("z".into());
        }
        let mut calls = 0;
        assert!(
            heatmap_layout_with_cell_factory(&c, &cells, &o, |ctx| {
                calls += 1;
                Ok(ctx.inherit(ctx.geometry.clone()))
            })
            .is_err()
        );
        assert_eq!(calls, 0);
        // An empty source has no fabricated palette context to evaluate.
        assert!(
            heatmap_layout(
                &c,
                &[],
                &HeatmapLayoutOptions {
                    hidden_columns: Default::default(),
                    ..o
                }
            )
            .is_ok()
        );
    }
    let opaque = HeatmapLayoutOptions {
        labels: false,
        label_color_profile: Some(InheritedColor::Contrast {
            dark: Color(0, 0, 0, 255),
            light: Color(255, 255, 255, 255),
        }),
        colors: ColorScaleOptions {
            colors: vec![Color(20, 30, 40, 255), Color(220, 230, 240, 255)],
            unknown: Color(20, 30, 40, 255),
            ..Default::default()
        },
        ..options()
    };
    assert!(
        heatmap_layout(&c, &cells, &opaque).is_ok(),
        "opaque actual surface is legal over translucent backdrop"
    );
}

#[test]
fn owned_label_color_and_existing_row_owner_tampering_reject_binding_and_hover() {
    let cells = [
        HeatCell {
            row: "a".into(),
            column: "x".into(),
            value: None,
        },
        HeatCell::new("b", "x", 0.),
    ];
    let source = heatmap_layout_with_cell_factory(
        &config(),
        &cells,
        &HeatmapLayoutOptions {
            label_color_profile: Some(InheritedColor::FromContext(vec![])),
            ..options()
        },
        |ctx| Ok(ctx.inherit(ctx.geometry.clone())),
    )
    .unwrap();
    for color_mutation in [true, false] {
        let mut layout = source.clone();
        if color_mutation {
            layout.scene.labels[0].color = Color(1, 2, 3, 4);
        } else {
            layout.scene.label_series.insert(0, "b".into());
        }
        let before = layout.scene.clone();
        let metadata = layout.cells.clone();
        let active = layout.active.clone();
        assert!(layout.interaction_binding().is_err());
        assert!(
            layout
                .set_active(Some(HeatmapCellKey::new("a", "x")))
                .is_err()
        );
        assert_eq!(layout.scene, before);
        assert_eq!(layout.cells, metadata);
        assert_eq!(layout.active, active);
    }
}

#[test]
fn profile_does_not_color_axes_or_change_domains_geometry_and_null_source() {
    let o = HeatmapLayoutOptions {
        axes: HeatmapAxes::default(),
        ..options()
    };
    let base = heatmap_layout(&config(), &data(), &o).unwrap();
    let mut colored = heatmap_layout(
        &config(),
        &data(),
        &HeatmapLayoutOptions {
            label_color_profile: Some(InheritedColor::Fixed(Color(33, 44, 55, 66))),
            ..o
        },
    )
    .unwrap();
    assert_eq!(base.cells, colored.cells);
    assert_eq!(base.color_scale, colored.color_scale);
    assert_eq!(base.scene.marks, colored.scene.marks);
    for (index, (a, b)) in base
        .scene
        .labels
        .iter()
        .zip(&mut colored.scene.labels)
        .enumerate()
    {
        if base.scene.label_series.contains_key(&index) {
            assert_eq!(b.color, Color(33, 44, 55, 66));
            b.color = a.color;
        } else {
            assert_eq!(b, a, "axis label must keep original color and geometry");
        }
    }
    assert_eq!(base.scene, colored.scene);
}

#[test]
fn source_alpha_example_restores_exact_rgba_after_custom_factory_and_hover() {
    let o = HeatmapLayoutOptions {
        opacity: 0.8,
        active_opacity: 0.9,
        inactive_opacity: 0.2,
        label_color_profile: Some(InheritedColor::FromContext(vec![ColorModifier::Darker(1.)])),
        ..options()
    };
    let mut layout = heatmap_layout_with_cell_factory(&config(), &data(), &o, |ctx| {
        let mut ink = ctx.inherit(ctx.geometry.clone());
        ink.fill = Some(Color(255, 0, 255, 255));
        ink.opacity = 0.5;
        Ok(ink)
    })
    .unwrap();
    assert_eq!(layout.scene.labels[1].color, Color(14, 28, 42, 64));
    let before = layout.scene.clone();
    layout
        .set_active(Some(HeatmapCellKey::new("r", "b")))
        .unwrap();
    assert_eq!(layout.scene.labels[1].color, Color(14, 28, 42, 72));
    layout
        .set_active(Some(HeatmapCellKey::new("r", "a")))
        .unwrap();
    assert_eq!(layout.scene.labels[1].color, Color(14, 28, 42, 16));
    layout.set_active(None).unwrap();
    assert_eq!(layout.scene, before);
    layout.interaction_binding().unwrap();
}
