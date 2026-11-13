use glory::render::{GraphicStroke, StrokeCap};
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 500.,
        height: 320.,
        ..Default::default()
    }
}
fn data() -> Vec<BoxObservation> {
    let mut out = Vec::new();
    for (group, subgroup, values) in [
        ("A", "s0", [1., 2., 3., 4.]),
        ("A", "s1", [2., 4., 6., 8.]),
        ("B", "s0", [5.; 4]),
    ] {
        for (i, value) in values.into_iter().enumerate() {
            out.push(BoxObservation::new(
                format!("{group}:{subgroup}:{i}"),
                group,
                subgroup,
                Some(value),
            ));
        }
    }
    out.push(BoxObservation::new("null", "B", "s1", None));
    out
}
fn stroke(color: Color, width: f64, cap: StrokeCap) -> GraphicStroke {
    GraphicStroke { color, width, cap }
}
fn options() -> BoxPlotLayoutOptions {
    BoxPlotLayoutOptions {
        groups: Some(vec!["A".into(), "B".into()]),
        subgroups: Some(vec!["s0".into(), "s1".into(), "empty".into()]),
        colors: [
            ("s0".into(), Color(20, 40, 60, 160)),
            ("s1".into(), Color(200, 180, 160, 220)),
            ("empty".into(), Color(90, 100, 110, 0)),
        ]
        .into(),
        border: Some(stroke(Color(9, 8, 7, 137), 3., StrokeCap::Round)),
        median: stroke(Color(70, 80, 90, 97), 4., StrokeCap::Square),
        whisker: stroke(Color(10, 30, 50, 181), 5., StrokeCap::Round),
        fill_opacity: 0.3,
        value_axis: AxisOptions {
            visible: false,
            grid: false,
            ..Default::default()
        },
        group_axis: CategoryAxisOptions {
            visible: false,
            grid: false,
            ..Default::default()
        },
        legend: false,
        ..Default::default()
    }
}
fn roles(mut o: BoxPlotLayoutOptions) -> BoxPlotLayoutOptions {
    o.border_color_profile = Some(InheritedColor::FromContext(vec![ColorModifier::Darker(1.)]));
    o.median_color_profile = Some(InheritedColor::FromContext(vec![
        ColorModifier::Darker(2.),
        ColorModifier::Opacity(0.5),
    ]));
    o.whisker_color_profile = Some(InheritedColor::Fixed(Color(7, 8, 9, 10)));
    o
}
fn part<'a>(l: &'a BoxPlotLayout, b: &ComputedBox, suffix: &str) -> &'a Mark {
    let id = format!("{}:{suffix}", b.id);
    l.scene.marks.iter().find(|m| m.id == id).unwrap()
}
fn expected(sub: &str) -> [Color; 3] {
    if sub == "s0" {
        [
            Color(14, 28, 42, 160),
            Color(10, 20, 29, 128),
            Color(7, 8, 9, 10),
        ]
    } else {
        [
            Color(140, 126, 112, 220),
            Color(98, 88, 78, 128),
            Color(7, 8, 9, 10),
        ]
    }
}

#[test]
fn independent_roles_recolor_every_box_stem_and_cap_without_changing_paint_opacity() {
    for orientation in [
        StatisticsOrientation::Vertical,
        StatisticsOrientation::Horizontal,
    ] {
        for radius in [0., 8.] {
            let o = roles(BoxPlotLayoutOptions {
                orientation,
                border_radius: radius,
                ..options()
            });
            let l = boxplot_layout(&config(), &data(), &o).unwrap();
            assert_eq!(l.boxes.len(), 3);
            for b in &l.boxes {
                let [border, median, whisker] = expected(&b.subgroup);
                assert_eq!(
                    part(&l, b, "border").stroke,
                    Some(GraphicStroke {
                        color: border,
                        ..o.border.unwrap()
                    })
                );
                assert_eq!(
                    part(&l, b, "median").stroke,
                    Some(GraphicStroke {
                        color: median,
                        ..o.median
                    })
                );
                for name in ["whiskers", "caps"] {
                    assert_eq!(
                        part(&l, b, name).stroke,
                        Some(GraphicStroke {
                            color: whisker,
                            ..o.whisker
                        })
                    );
                    assert_eq!(part(&l, b, name).opacity, 1.);
                    assert!(part(&l, b, name).fill.is_none());
                }
                assert_eq!(part(&l, b, "border").opacity, 1.);
                assert_eq!(part(&l, b, "median").opacity, 1.);
                let fill = l.scene.marks.iter().find(|m| m.id == b.id).unwrap();
                assert_eq!(fill.opacity, 0.3);
                assert_eq!(fill.fill, Some(o.colors[&b.subgroup]));
            }
        }
    }
}

#[test]
fn theme_resolves_each_own_fixed_stroke_and_a_profile_does_not_enable_a_missing_border() {
    for border in [None, options().border] {
        let o = BoxPlotLayoutOptions {
            border,
            ..options()
        };
        let plain = boxplot_layout(&config(), &data(), &o).unwrap();
        let themed = boxplot_layout(
            &config(),
            &data(),
            &BoxPlotLayoutOptions {
                border_color_profile: Some(InheritedColor::Theme),
                median_color_profile: Some(InheritedColor::Theme),
                whisker_color_profile: Some(InheritedColor::Theme),
                ..o.clone()
            },
        )
        .unwrap();
        assert_eq!(plain, themed);
        assert_eq!(
            render::to_svg(&plain.scene).unwrap(),
            render::to_svg(&themed.scene).unwrap()
        );
        assert_eq!(
            render::to_canvas_script(&plain.scene, "canvas").unwrap(),
            render::to_canvas_script(&themed.scene, "canvas").unwrap()
        );
        let fixed = boxplot_layout(
            &config(),
            &data(),
            &BoxPlotLayoutOptions {
                border_color_profile: Some(InheritedColor::Fixed(Color(1, 2, 3, 4))),
                ..o
            },
        )
        .unwrap();
        for b in &fixed.boxes {
            assert_eq!(
                fixed
                    .scene
                    .marks
                    .iter()
                    .find(|m| m.id == format!("{}:border", b.id))
                    .map(|m| m.stroke.unwrap().color),
                border.map(|_| Color(1, 2, 3, 4))
            );
        }
    }
}

#[test]
fn role_changes_leave_all_statistics_domains_geometry_axes_legends_and_outliers_exact() {
    let mut source = data();
    source.push(BoxObservation::new("outlier", "A", "s0", Some(100.)));
    let o = BoxPlotLayoutOptions {
        whiskers: BoxWhiskers::Tukey { factor: 1.5 },
        outlier_fill: Some(Color(123, 45, 67, 89)),
        outlier_border: Some(stroke(Color(2, 3, 4, 5), 2., StrokeCap::Butt)),
        legend: true,
        value_axis: AxisOptions {
            grid: true,
            ..Default::default()
        },
        group_axis: Default::default(),
        ..options()
    };
    let plain = boxplot_layout(&config(), &source, &o).unwrap();
    let colored = boxplot_layout(&config(), &source, &roles(o)).unwrap();
    assert_eq!(plain.boxes, colored.boxes);
    assert_eq!(plain.domain, colored.domain);
    assert_eq!(plain.groups, colored.groups);
    assert_eq!(plain.subgroups, colored.subgroups);
    assert_eq!(plain.category_scale, colored.category_scale);
    let mut normalized = colored.scene.clone();
    for m in &mut normalized.marks {
        if colored.boxes.iter().any(|b| {
            ["border", "median", "whiskers", "caps"]
                .iter()
                .any(|s| m.id == format!("{}:{s}", b.id))
        }) {
            let old = plain.scene.marks.iter().find(|old| old.id == m.id).unwrap();
            m.stroke.as_mut().unwrap().color = old.stroke.unwrap().color;
        }
    }
    assert_eq!(normalized, plain.scene);
    assert!(
        colored
            .scene
            .marks
            .iter()
            .any(|m| m.id.contains(":outlier:"))
    );
}

#[test]
fn hidden_subgroups_keep_their_full_palette_slots_and_null_cells_do_not_form_boxes() {
    let mut c = config();
    c.theme.colors = vec![
        Color(20, 40, 60, 160),
        Color(200, 180, 160, 220),
        Color(90, 100, 110, 64),
    ];
    let mut o = roles(options());
    o.colors.clear();
    o.hidden.insert(boxplot_series_id("s0"));
    let l = boxplot_layout(&c, &data(), &o).unwrap();
    assert_eq!(l.boxes.len(), 1);
    let b = &l.boxes[0];
    assert_eq!((&*b.group, &*b.subgroup), ("A", "s1"));
    assert_eq!(
        part(&l, b, "border").stroke.unwrap().color,
        Color(140, 126, 112, 220)
    );
    assert_eq!(l.subgroups, ["s0", "s1", "empty"]);
    assert_eq!(l.boxes[0].count, 4);
    assert!(!l.boxes.iter().any(|b| b.group == "B" && b.subgroup == "s1"));
}

#[test]
fn invalid_intrinsic_profiles_reject_empty_hidden_null_and_disabled_ink() {
    for profile in [
        InheritedColor::FromContext(vec![ColorModifier::Opacity(f64::NAN)]),
        InheritedColor::FromContext(vec![ColorModifier::Darker(f64::INFINITY)]),
        InheritedColor::FromContext(vec![ColorModifier::Brighter(1.); 33]),
    ] {
        for role in 0..3 {
            for scenario in 0..5 {
                let mut o = options();
                match role {
                    0 => o.border_color_profile = Some(profile.clone()),
                    1 => o.median_color_profile = Some(profile.clone()),
                    _ => o.whisker_color_profile = Some(profile.clone()),
                }
                let source = match scenario {
                    0 => vec![],
                    1 => {
                        o.hidden = [
                            boxplot_series_id("s0"),
                            boxplot_series_id("s1"),
                            boxplot_series_id("empty"),
                        ]
                        .into();
                        data()
                    }
                    2 => vec![BoxObservation::new("missing", "A", "s0", None)],
                    3 => {
                        o.border = None;
                        o.median.width = 0.;
                        o.whisker.width = 0.;
                        o.cap_size = 0.;
                        data()
                    }
                    _ => data(),
                };
                let original = source.clone();
                assert!(boxplot_layout(&config(), &source, &o).is_err());
                assert_eq!(source, original);
            }
        }
    }
}

#[test]
fn contrast_checks_only_real_visible_box_surfaces_even_when_stroke_or_fill_ink_is_disabled() {
    let mut c = config();
    c.theme.background = Color(255, 255, 255, 0);
    let contrast = InheritedColor::Contrast {
        dark: Color(0, 0, 0, 255),
        light: Color(255, 255, 255, 255),
    };
    for role in 0..3 {
        let mut o = options();
        match role {
            0 => {
                o.border_color_profile = Some(contrast.clone());
                o.border = None;
            }
            1 => {
                o.median_color_profile = Some(contrast.clone());
                o.median.width = 0.;
            }
            _ => {
                o.whisker_color_profile = Some(contrast.clone());
                o.whisker.width = 0.;
                o.cap_size = 0.;
            }
        }
        o.fill_opacity = 0.;
        assert!(boxplot_layout(&c, &data(), &o).is_err());
        // No summary exists for empty, nullable-only or hidden cells; these
        // inputs must not fabricate a contrast surface from reserved slots.
        assert!(boxplot_layout(&c, &[], &o).is_ok());
        assert!(boxplot_layout(&c, &[BoxObservation::new("n", "A", "s0", None)], &o).is_ok());
        o.hidden = [
            boxplot_series_id("s0"),
            boxplot_series_id("s1"),
            boxplot_series_id("empty"),
        ]
        .into();
        assert!(boxplot_layout(&c, &data(), &o).is_ok());
        o.hidden.clear();
        o.colors.insert("s0".into(), Color(0, 0, 0, 255));
        o.colors.insert("s1".into(), Color(255, 255, 255, 255));
        assert!(boxplot_layout(&c, &data(), &o).is_ok());
    }
}

#[test]
fn contrast_uses_original_numeric_surface_and_keeps_alpha_separate_from_fill_opacity() {
    let contrast = InheritedColor::Contrast {
        dark: Color(0, 0, 0, 255),
        light: Color(255, 255, 255, 255),
    };
    let mut o = options();
    o.fill_opacity = 0.;
    o.colors.insert("s0".into(), Color(0, 0, 0, 255));
    o.colors.insert("s1".into(), Color(255, 255, 255, 255));
    o.border_color_profile = Some(contrast.clone());
    o.median_color_profile = Some(contrast.clone());
    o.whisker_color_profile = Some(contrast);
    let l = boxplot_layout(&config(), &data(), &o).unwrap();
    for b in &l.boxes {
        let expected = if b.subgroup == "s0" {
            Color(255, 255, 255, 255)
        } else {
            Color(0, 0, 0, 255)
        };
        for s in ["border", "median", "whiskers", "caps"] {
            assert_eq!(part(&l, b, s).stroke.unwrap().color, expected);
            assert_eq!(part(&l, b, s).opacity, 1.);
        }
    }
}

#[test]
fn constant_boxes_and_zero_width_or_caps_preserve_existing_geometry_and_disable_rules() {
    let source = [
        BoxObservation::new("a", "A", "s0", Some(0.)),
        BoxObservation::new("b", "A", "s0", Some(0.)),
    ];
    for orientation in [
        StatisticsOrientation::Vertical,
        StatisticsOrientation::Horizontal,
    ] {
        let o = roles(BoxPlotLayoutOptions {
            orientation,
            cap_size: 0.,
            median: GraphicStroke {
                width: 0.,
                ..options().median
            },
            ..options()
        });
        let l = boxplot_layout(&config(), &source, &o).unwrap();
        assert_eq!(l.boxes.len(), 1);
        assert_eq!(l.boxes[0].quantiles, [0.; 5]);
        assert_eq!(l.domain, [0., 0.]);
        assert_eq!(
            part(&l, &l.boxes[0], "border").stroke.unwrap().color,
            Color(14, 28, 42, 160)
        );
        assert!(
            !l.scene
                .marks
                .iter()
                .any(|m| m.id.ends_with(":median") || m.id.ends_with(":caps"))
        );
        assert!(l.scene.marks.iter().any(|m| m.id.ends_with(":whiskers")));
    }
}

#[test]
fn shared_scale_entry_keeps_fractional_utc_statistics_and_domain_queries_unmodified() {
    let source = (0..4)
        .map(|i| {
            BoxObservation::new(
                format!("t{i}"),
                "A",
                "s0",
                Some(1_600_000_000_000. + i as f64),
            )
        })
        .collect::<Vec<_>>();
    let scale = ContinuousScaleOptions {
        kind: ScaleKind::Utc,
        reverse: true,
        round: true,
        ..Default::default()
    };
    let plain = boxplot_layout_with_scale_options(&config(), &source, &options(), &scale).unwrap();
    let colored =
        boxplot_layout_with_scale_options(&config(), &source, &roles(options()), &scale).unwrap();
    assert_eq!(plain.layout.boxes, colored.layout.boxes);
    assert_eq!(plain.layout.domain, colored.layout.domain);
    assert_eq!(
        plain.value_scale.source_domain(),
        colored.value_scale.source_domain()
    );
    for value in plain.layout.boxes[0].quantiles {
        assert_eq!(
            plain.map_statistic(value).unwrap().to_bits(),
            colored.map_statistic(value).unwrap().to_bits()
        );
    }
}

#[test]
fn typed_zero_alpha_palette_channels_and_existing_sample_budget_are_not_rewritten() {
    let mut o = options();
    o.colors.insert("s0".into(), Color(20, 40, 60, 0));
    o.border_color_profile = Some(InheritedColor::FromContext(vec![
        ColorModifier::Darker(1.),
        ColorModifier::Opacity(0.5),
    ]));
    let l = boxplot_layout(&config(), &data(), &o).unwrap();
    for b in l.boxes.iter().filter(|b| b.subgroup == "s0") {
        assert_eq!(
            part(&l, b, "border").stroke.unwrap().color,
            Color(14, 28, 42, 128)
        );
    }
    let excessive = (0..10001)
        .map(|i| BoxObservation::new(format!("n{i}"), "A", "s0", Some(1.)))
        .collect::<Vec<_>>();
    let error = boxplot_layout(&config(), &excessive, &o).unwrap_err();
    assert!(error.0.contains("sample budget"));
}
