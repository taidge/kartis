use glory::render::{GraphicStroke, StrokeCap};
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 500.,
        height: 320.,
        ..Default::default()
    }
}
fn observations() -> Vec<BoxObservation> {
    let mut source = Vec::new();
    for (group, subgroup, values) in [
        ("共有/甲", "x", [1., 2., 3., 4.]),
        ("共有/甲", "共有/甲", [2., 3., 4., 5.]),
        ("x", "x", [1., 2., 3., 4.]),
        ("x", "共有/甲", [2., 3., 4., 5.]),
    ] {
        for (i, value) in values.into_iter().enumerate() {
            source.push(BoxObservation::new(
                format!("{group}|{subgroup}|{i}"),
                group,
                subgroup,
                Some(value),
            ));
        }
    }
    source.push(BoxObservation::new("null-only", "null", "x", None));
    source
}
fn options() -> BoxPlotLayoutOptions {
    BoxPlotLayoutOptions {
        groups: Some(vec![
            "reserved".into(),
            "x".into(),
            "共有/甲".into(),
            "null".into(),
        ]),
        subgroups: Some(vec!["共有/甲".into(), "x".into(), "reserved".into()]),
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
        ..Default::default()
    }
}
fn stroke(color: Color, width: f64) -> GraphicStroke {
    GraphicStroke {
        color,
        width,
        cap: StrokeCap::Square,
    }
}
fn part<'a>(l: &'a BoxPlotLayout, b: &ComputedBox, suffix: &str) -> &'a Mark {
    l.scene
        .marks
        .iter()
        .find(|m| m.id == format!("{}:{suffix}", b.id))
        .unwrap()
}
fn numeric(group: &str) -> Color {
    match group {
        "x" => Color(20, 40, 60, 160),
        "共有/甲" => Color(200, 180, 160, 220),
        _ => panic!("not a real source group"),
    }
}
fn group_options() -> BoxPlotLayoutOptions {
    BoxPlotLayoutOptions {
        color_by: BoxPlotColorBy::Group,
        colors: [
            ("x".into(), numeric("x")),
            ("共有/甲".into(), numeric("共有/甲")),
        ]
        .into(),
        ..options()
    }
}

#[test]
fn group_numeric_colors_reach_body_roles_and_default_outliers_without_changing_series_identity() {
    for orientation in [
        StatisticsOrientation::Vertical,
        StatisticsOrientation::Horizontal,
    ] {
        let o = BoxPlotLayoutOptions {
            orientation,
            border: Some(stroke(Color(9, 8, 7, 130), 3.)),
            border_color_profile: Some(InheritedColor::FromContext(vec![ColorModifier::Darker(
                1.,
            )])),
            median_color_profile: Some(InheritedColor::FromContext(vec![
                ColorModifier::Darker(2.),
                ColorModifier::Opacity(0.5),
            ])),
            whisker_color_profile: Some(InheritedColor::FromContext(vec![])),
            fill_opacity: 0.3,
            ..group_options()
        };
        let source = observations();
        let old_source = source.clone();
        let l = boxplot_layout(&config(), &source, &o).unwrap();
        assert_eq!(source, old_source);
        assert_eq!(l.boxes.len(), 4);
        let mut outliers = 0;
        for b in &l.boxes {
            let color = numeric(&b.group);
            let expected = if b.group == "x" {
                [Color(14, 28, 42, 160), Color(10, 20, 29, 128)]
            } else {
                [Color(140, 126, 112, 220), Color(98, 88, 78, 128)]
            };
            let body = l.scene.marks.iter().find(|m| m.id == b.id).unwrap();
            assert_eq!(body.fill, Some(color));
            assert_eq!(body.opacity, 0.3);
            assert_eq!(part(&l, b, "border").stroke.unwrap().color, expected[0]);
            assert_eq!(part(&l, b, "median").stroke.unwrap().color, expected[1]);
            for suffix in ["whiskers", "caps"] {
                assert_eq!(part(&l, b, suffix).stroke.unwrap().color, color);
            }
            for mark in l
                .scene
                .marks
                .iter()
                .filter(|m| m.id == b.id || m.id.starts_with(&format!("{}:", b.id)))
            {
                assert_eq!(mark.series, boxplot_series_id(&b.subgroup));
                if mark.id.contains(":outlier:") {
                    outliers += 1;
                    assert_eq!(mark.fill, Some(color));
                }
                if mark.stroke.is_some() {
                    assert_eq!(mark.opacity, 1.);
                }
            }
        }
        assert!(outliers > 0);
        assert_eq!(
            l.scene
                .legend
                .iter()
                .map(|v| v.id.as_str())
                .collect::<Vec<_>>(),
            ["x", "共有/甲"]
        );
        for item in &l.scene.legend {
            assert_eq!(item.color, numeric(&item.id));
            assert_eq!(l.scene.legend_role(&item.id), Some(LegendRole::Category));
        }
        l.scene.validate().unwrap();
        assert!(render::to_svg(&l.scene).unwrap().contains("<svg"));
        assert!(
            !render::to_canvas_script(&l.scene, "canvas")
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn complete_group_order_reserves_palette_slots_while_legends_only_list_real_visible_boxes() {
    let mut c = config();
    c.theme.colors = vec![
        Color(1, 2, 3, 255),
        Color(11, 12, 13, 255),
        Color(21, 22, 23, 255),
        Color(31, 32, 33, 255),
    ];
    let o = BoxPlotLayoutOptions {
        color_by: BoxPlotColorBy::Group,
        ..options()
    };
    let source = observations();
    let full = boxplot_layout(&c, &source, &o).unwrap();
    assert_eq!(
        full.scene
            .legend
            .iter()
            .map(|x| (&*x.id, x.color))
            .collect::<Vec<_>>(),
        [
            ("x", Color(11, 12, 13, 255)),
            ("共有/甲", Color(21, 22, 23, 255))
        ]
    );
    let hidden = boxplot_layout(
        &c,
        &source,
        &BoxPlotLayoutOptions {
            hidden: [boxplot_series_id("x")].into(),
            ..o.clone()
        },
    )
    .unwrap();
    assert_eq!(hidden.boxes.len(), 2);
    assert!(hidden.boxes.iter().all(|b| b.subgroup == "共有/甲"));
    assert_eq!(hidden.scene.legend, full.scene.legend);
    let raw_group_hidden = boxplot_layout(
        &c,
        &source,
        &BoxPlotLayoutOptions {
            hidden: ["x".into()].into(),
            ..o.clone()
        },
    )
    .unwrap();
    assert_eq!(
        raw_group_hidden, full,
        "raw Group category IDs must not become subgroup visibility inputs"
    );
    let all_hidden = boxplot_layout(
        &c,
        &source,
        &BoxPlotLayoutOptions {
            hidden: [boxplot_series_id("x"), boxplot_series_id("共有/甲")].into(),
            ..o
        },
    )
    .unwrap();
    assert!(
        all_hidden.boxes.is_empty()
            && all_hidden.scene.legend.is_empty()
            && all_hidden.scene.legend_roles.is_empty()
    );
}

#[test]
fn default_subgroup_keeps_reserved_series_legend_and_empty_name_samples() {
    let source = [
        BoxObservation::new("a", "A", "", Some(0.)),
        BoxObservation::new("none", "A", "null", None),
    ];
    let l = boxplot_layout(
        &config(),
        &source,
        &BoxPlotLayoutOptions {
            subgroups: Some(vec!["reserved".into(), "".into(), "null".into()]),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        BoxPlotLayoutOptions::default().color_by,
        BoxPlotColorBy::SubGroup
    );
    assert_eq!(
        l.scene
            .legend
            .iter()
            .map(|x| (&*x.id, &*x.label))
            .collect::<Vec<_>>(),
        [
            (boxplot_series_id("reserved").as_str(), "reserved"),
            (boxplot_series_id("").as_str(), "Samples"),
            (boxplot_series_id("null").as_str(), "null")
        ]
    );
    assert!(l.scene.legend_roles.is_empty());
    for item in &l.scene.legend {
        assert_eq!(l.scene.legend_role(&item.id), Some(LegendRole::Series));
    }
}

#[test]
fn group_legend_category_ids_keep_raw_unicode_slashes_and_cannot_implicitly_hide_series() {
    let source = observations();
    let l = boxplot_layout(&config(), &source, &group_options()).unwrap();
    assert_eq!(
        l.scene
            .legend
            .iter()
            .map(|x| x.label.as_str())
            .collect::<Vec<_>>(),
        ["x", "共有/甲"]
    );
    for item in &l.scene.legend {
        assert_eq!(l.scene.legend_role(&item.id), Some(LegendRole::Category));
        assert!(!l.scene.legend_hidden(&item.id, &[item.id.clone()].into()));
        assert!(
            !l.scene
                .legend_hidden(&item.id, &[boxplot_series_id(&item.id)].into())
        );
    }
    let filtered = l.scene.without_series(&[boxplot_series_id("x")].into());
    assert!(
        filtered
            .marks
            .iter()
            .all(|m| m.series != boxplot_series_id("x"))
    );
    assert_eq!(
        filtered.legend, l.scene.legend,
        "programmatic subgroup visibility must preserve Category legend identities"
    );
}

#[test]
fn selected_color_key_domain_is_validated_even_when_hidden_empty_or_without_legend() {
    for color_by in [BoxPlotColorBy::SubGroup, BoxPlotColorBy::Group] {
        let valid = if color_by == BoxPlotColorBy::Group {
            "A"
        } else {
            "s"
        };
        let invalid = if color_by == BoxPlotColorBy::Group {
            "s"
        } else {
            "A"
        };
        for source in [
            vec![],
            vec![BoxObservation::new("none", "A", "s", None)],
            vec![BoxObservation::new("a", "A", "s", Some(1.))],
        ] {
            let mut o = BoxPlotLayoutOptions {
                color_by,
                groups: Some(vec!["A".into()]),
                subgroups: Some(vec!["s".into()]),
                hidden: [boxplot_series_id("s")].into(),
                legend: false,
                colors: [(valid.into(), Color(1, 2, 3, 4))].into(),
                ..Default::default()
            };
            assert!(boxplot_layout(&config(), &source, &o).is_ok());
            o.colors = [(invalid.into(), Color(1, 2, 3, 4))].into();
            assert_eq!(
                boxplot_layout(&config(), &source, &o).unwrap_err().0,
                "unknown boxplot color key"
            );
        }
    }
}

#[test]
fn zero_constant_null_and_empty_group_qualification_do_not_depend_on_paint_extent() {
    let source = [
        BoxObservation::new("zero", "zero", "s", Some(0.)),
        BoxObservation::new("constant", "constant", "s", Some(7.)),
        BoxObservation::new("none", "null", "s", None),
    ];
    let o = BoxPlotLayoutOptions {
        color_by: BoxPlotColorBy::Group,
        groups: Some(vec![
            "reserved".into(),
            "null".into(),
            "zero".into(),
            "constant".into(),
        ]),
        subgroups: Some(vec!["s".into()]),
        median: stroke(Color(1, 2, 3, 4), 0.),
        whisker: stroke(Color(1, 2, 3, 4), 0.),
        border: Some(stroke(Color(1, 2, 3, 4), -0.)),
        fill_opacity: 0.,
        ..Default::default()
    };
    let l = boxplot_layout(&config(), &source, &o).unwrap();
    assert_eq!(l.boxes.len(), 2);
    assert_eq!(
        l.scene
            .legend
            .iter()
            .map(|x| x.id.as_str())
            .collect::<Vec<_>>(),
        ["zero", "constant"]
    );
    // Unchanged axes/grid keep their own strokes. Only authored box parts
    // are disabled by the zero box/median/whisker widths in this input.
    for b in &l.boxes {
        let own = l
            .scene
            .marks
            .iter()
            .filter(|m| m.id == b.id || m.id.starts_with(&format!("{}:", b.id)))
            .collect::<Vec<_>>();
        assert!(!own.is_empty());
        assert!(own.iter().all(|m| m.stroke.is_none()));
    }
    assert!(l.scene.marks.iter().any(|m| m.stroke.is_some()));
    let e = boxplot_layout(&config(), &[], &o).unwrap();
    assert!(e.boxes.is_empty() && e.scene.legend.is_empty() && e.scene.legend_roles.is_empty());
}

#[test]
fn group_palette_changes_only_numeric_paint_and_roles_across_bands_crop_and_value_scales() {
    for orientation in [
        StatisticsOrientation::Vertical,
        StatisticsOrientation::Horizontal,
    ] {
        for kind in [
            ScaleKind::Linear,
            ScaleKind::Log,
            ScaleKind::Pow { exponent: -1. },
            ScaleKind::Utc,
        ] {
            let o = BoxPlotLayoutOptions {
                orientation,
                domain: Some([2., 3.]),
                category_scale: Some(scales::BandScaleOptions {
                    round: true,
                    padding_inner: 0.2,
                    padding_outer: 0.3,
                    align: 0.25,
                }),
                category_reverse: true,
                ..options()
            };
            let source = observations();
            let scale = ContinuousScaleOptions {
                kind,
                reverse: true,
                round: true,
                clamp: true,
                ..Default::default()
            };
            let old = boxplot_layout_with_scale_options(&config(), &source, &o, &scale).unwrap();
            let new = boxplot_layout_with_scale_options(
                &config(),
                &source,
                &BoxPlotLayoutOptions {
                    color_by: BoxPlotColorBy::Group,
                    ..o
                },
                &scale,
            )
            .unwrap();
            assert_eq!(old.layout.boxes, new.layout.boxes);
            assert_eq!(old.layout.domain, new.layout.domain);
            assert_eq!(old.layout.category_scale, new.layout.category_scale);
            assert_eq!(old.layout.groups, new.layout.groups);
            assert_eq!(old.layout.subgroups, new.layout.subgroups);
            assert_eq!(old.layout.scene.labels, new.layout.scene.labels);
            assert_eq!(
                old.value_scale.source_domain(),
                new.value_scale.source_domain()
            );
            for (a, b) in old.layout.scene.marks.iter().zip(&new.layout.scene.marks) {
                let mut a = a.clone();
                let mut b = b.clone();
                a.fill = None;
                b.fill = None;
                a.stroke = a.stroke.map(|s| GraphicStroke {
                    color: Color(0, 0, 0, 255),
                    ..s
                });
                b.stroke = b.stroke.map(|s| GraphicStroke {
                    color: Color(0, 0, 0, 255),
                    ..s
                });
                assert_eq!(a, b);
            }
            for b in &old.layout.boxes {
                for q in b.quantiles {
                    assert_eq!(
                        old.map_statistic(q).unwrap().to_bits(),
                        new.map_statistic(q).unwrap().to_bits()
                    );
                }
            }
        }
    }
}

#[test]
fn group_theme_fallbacks_disabled_border_and_context_errors_keep_existing_contracts() {
    let o = BoxPlotLayoutOptions {
        border: Some(stroke(Color(1, 2, 3, 45), 0.)),
        border_color_profile: Some(InheritedColor::Theme),
        median: stroke(Color(4, 5, 6, 78), 2.),
        median_color_profile: Some(InheritedColor::Theme),
        whisker: stroke(Color(7, 8, 9, 101), 3.),
        whisker_color_profile: Some(InheritedColor::Theme),
        outlier_fill: Some(Color(99, 88, 77, 66)),
        ..group_options()
    };
    let l = boxplot_layout(&config(), &observations(), &o).unwrap();
    for b in &l.boxes {
        assert!(
            !l.scene
                .marks
                .iter()
                .any(|m| m.id == format!("{}:border", b.id))
        );
        assert_eq!(
            part(&l, b, "median").stroke.unwrap().color,
            Color(4, 5, 6, 78)
        );
        assert_eq!(
            part(&l, b, "whiskers").stroke.unwrap().color,
            Color(7, 8, 9, 101)
        );
    }
    assert!(
        l.scene
            .marks
            .iter()
            .filter(|m| m.id.contains(":outlier:"))
            .all(|m| m.fill == Some(Color(99, 88, 77, 66)))
    );
    let mut c = config();
    c.theme.background = Color(0, 0, 0, 0);
    let contrast = BoxPlotLayoutOptions {
        border_color_profile: Some(InheritedColor::Contrast {
            dark: Color(0, 0, 0, 255),
            light: Color(255, 255, 255, 255),
        }),
        ..o
    };
    assert!(
        boxplot_layout(&c, &observations(), &contrast)
            .unwrap_err()
            .0
            .contains("opaque effective background")
    );
    assert!(boxplot_layout(&c, &[], &contrast).is_ok());
    let invalid = BoxPlotLayoutOptions {
        border_color_profile: Some(InheritedColor::FromContext(vec![ColorModifier::Opacity(
            f64::NAN,
        )])),
        ..contrast
    };
    assert!(boxplot_layout(&c, &[], &invalid).is_err());
}

#[cfg(feature = "geojson")]
#[test]
fn frozen_original_group_mode_role_records_match_without_relabeling_alpha0_boundaries() {
    use serde_json::Value;
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/radial-box-stroke/values.json")).unwrap();
    let rgba = |v: &Value| {
        Color(
            v[0].as_u64().unwrap() as u8,
            v[1].as_u64().unwrap() as u8,
            v[2].as_u64().unwrap() as u8,
            v[3].as_u64().unwrap() as u8,
        )
    };
    let profile = |name: &str, role: &str| match name {
        "native-default" => InheritedColor::FromContext(if role == "median" {
            vec![ColorModifier::Darker(2.)]
        } else {
            vec![]
        }),
        "inherit" => InheritedColor::FromContext(vec![]),
        "darker" => InheritedColor::FromContext(vec![ColorModifier::Darker(1.)]),
        "ordered" => InheritedColor::FromContext(vec![
            ColorModifier::Brighter(1.),
            ColorModifier::Opacity(0.5),
            ColorModifier::Darker(1.),
        ]),
        "alpha0-visible" => {
            InheritedColor::FromContext(vec![ColorModifier::Darker(1.), ColorModifier::Opacity(1.)])
        }
        "fixed" => InheritedColor::Fixed(Color(12, 34, 56, 78)),
        "theme" => InheritedColor::Theme,
        _ => panic!("unmapped fixture profile"),
    };
    let (mut cases, mut ordinary, mut boundaries) = (0, 0, 0);
    for case in fixture["box"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["colorBy"] == "group")
    {
        cases += 1;
        assert_eq!(
            case["rustComparableMode"], false,
            "historical fixture flag remains unchanged"
        );
        let rows = case["boxes"].as_array().unwrap();
        let mut source = Vec::new();
        for row in rows {
            for (i, raw) in row["sourceSummary"]["extrema"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
            {
                source.push(BoxObservation::new(
                    format!(
                        "{}/{}/{}",
                        row["group"].as_str().unwrap(),
                        row["subGroup"].as_str().unwrap(),
                        i
                    ),
                    row["group"].as_str().unwrap(),
                    row["subGroup"].as_str().unwrap(),
                    raw.as_f64(),
                ));
            }
        }
        let name = case["profile"].as_str().unwrap();
        let o = BoxPlotLayoutOptions {
            color_by: BoxPlotColorBy::Group,
            groups: Some(vec!["A".into(), "B".into(), "C".into()]),
            subgroups: Some(vec!["x".into(), "y".into(), "z".into()]),
            colors: rows
                .iter()
                .map(|r| {
                    (
                        r["group"].as_str().unwrap().to_owned(),
                        rgba(&r["numericRGBA"]),
                    )
                })
                .collect(),
            orientation: if case["layout"] == "horizontal" {
                StatisticsOrientation::Horizontal
            } else {
                StatisticsOrientation::Vertical
            },
            border: Some(stroke(Color(70, 80, 90, 123), 2.)),
            median: stroke(Color(120, 20, 80, 90), 2.),
            whisker: stroke(Color(20, 90, 120, 200), 2.),
            border_color_profile: Some(profile(name, "border")),
            median_color_profile: Some(profile(name, "median")),
            whisker_color_profile: Some(profile(name, "whisker")),
            legend: false,
            ..Default::default()
        };
        let l = boxplot_layout(&config(), &source, &o).unwrap();
        assert_eq!(l.boxes.len(), 5);
        for row in rows {
            let b = l
                .boxes
                .iter()
                .find(|b| {
                    b.group == row["group"].as_str().unwrap()
                        && b.subgroup == row["subGroup"].as_str().unwrap()
                })
                .unwrap();
            assert_eq!(
                l.scene.marks.iter().find(|m| m.id == b.id).unwrap().fill,
                Some(rgba(&row["numericRGBA"]))
            );
            for role in ["border", "median", "whisker"] {
                let native = &row["roles"][role];
                let suffix = if role == "whisker" { "whiskers" } else { role };
                let actual = part(&l, b, suffix).stroke.unwrap().color;
                if native["cssAlpha0ParserBoundary"].as_bool().unwrap() {
                    boundaries += 1;
                    assert_ne!(actual, rgba(&native["rgba"]));
                    assert_eq!(actual.3, rgba(&native["rgba"]).3);
                } else {
                    ordinary += 1;
                    assert_eq!(
                        actual,
                        rgba(&native["rgba"]),
                        "{} {} {role}",
                        case["id"],
                        row["key"]
                    );
                }
                if role == "whisker" {
                    assert_eq!(part(&l, b, "caps").stroke.unwrap().color, actual);
                }
            }
        }
    }
    assert_eq!((cases, ordinary, boundaries), (14, 190, 20));
}
