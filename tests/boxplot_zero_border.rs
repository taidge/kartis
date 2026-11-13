use glory::render::{GraphicStroke, StrokeCap};
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 420.,
        height: 240.,
        ..Default::default()
    }
}
fn data() -> Vec<BoxObservation> {
    [1., 2., 3., 4.]
        .into_iter()
        .enumerate()
        .map(|(i, v)| BoxObservation::new(i.to_string(), "A", "s", Some(v)))
        .collect()
}
fn stroke(width: f64) -> GraphicStroke {
    GraphicStroke {
        width,
        color: Color(70, 80, 90, 101),
        cap: StrokeCap::Square,
    }
}
fn options() -> BoxPlotLayoutOptions {
    BoxPlotLayoutOptions {
        groups: Some(vec!["A".into()]),
        subgroups: Some(vec!["s".into()]),
        colors: [("s".into(), Color(20, 40, 60, 160))].into(),
        fill_opacity: 0.3,
        median: GraphicStroke {
            width: 3.,
            color: Color(12, 34, 56, 78),
            cap: StrokeCap::Round,
        },
        whisker: GraphicStroke {
            width: 4.,
            color: Color(87, 65, 43, 21),
            cap: StrokeCap::Butt,
        },
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
fn layout(
    source: &[BoxObservation],
    mut o: BoxPlotLayoutOptions,
    horizontal: bool,
    scaled: bool,
) -> Result<BoxPlotLayout> {
    o.orientation = if horizontal {
        StatisticsOrientation::Horizontal
    } else {
        StatisticsOrientation::Vertical
    };
    if scaled {
        Ok(boxplot_layout_with_scale_options(
            &config(),
            source,
            &o,
            &ContinuousScaleOptions {
                reverse: true,
                round: true,
                ..Default::default()
            },
        )?
        .layout)
    } else {
        boxplot_layout(&config(), source, &o)
    }
}

#[test]
fn zero_and_negative_zero_border_are_exactly_none_in_both_layouts_and_orientations() {
    for horizontal in [false, true] {
        for scaled in [false, true] {
            for width in [0., -0.] {
                for profile in [
                    None,
                    Some(InheritedColor::Theme),
                    Some(InheritedColor::Fixed(Color(1, 2, 3, 4))),
                    Some(InheritedColor::FromContext(vec![ColorModifier::Darker(1.)])),
                ] {
                    let o = BoxPlotLayoutOptions {
                        border_color_profile: profile,
                        ..options()
                    };
                    let none = layout(&data(), o.clone(), horizontal, scaled).unwrap();
                    let zero=layout(&data(),BoxPlotLayoutOptions{border:Some(stroke(width)),..o},horizontal,scaled).expect("zero border width must disable border ink instead of creating an invalid Scene mark");
                    assert_eq!(zero, none);
                    assert!(!zero.scene.marks.iter().any(|m| m.id.ends_with(":border")));
                    assert_eq!(
                        render::to_svg(&zero.scene).unwrap(),
                        render::to_svg(&none.scene).unwrap()
                    );
                    assert_eq!(
                        render::to_canvas_script(&zero.scene, "canvas").unwrap(),
                        render::to_canvas_script(&none.scene, "canvas").unwrap()
                    );
                }
            }
        }
    }
}

#[test]
fn real_box_contrast_is_still_validated_with_zero_border_or_no_fill_ink() {
    let mut c = config();
    c.theme.background = Color(255, 255, 255, 0);
    let o = BoxPlotLayoutOptions {
        border: Some(stroke(0.)),
        border_color_profile: Some(InheritedColor::Contrast {
            dark: Color(0, 0, 0, 255),
            light: Color(255, 255, 255, 255),
        }),
        fill_opacity: 0.,
        ..options()
    };
    for horizontal in [false, true] {
        for scaled in [false, true] {
            for constant in [false, true] {
                let source = if constant {
                    vec![BoxObservation::new("constant", "A", "s", Some(0.))]
                } else {
                    data()
                };
                let original = source.clone();
                let mut profile = o.clone();
                profile.orientation = if horizontal {
                    StatisticsOrientation::Horizontal
                } else {
                    StatisticsOrientation::Vertical
                };
                let result = if scaled {
                    boxplot_layout_with_scale_options(
                        &c,
                        &source,
                        &profile,
                        &ContinuousScaleOptions::default(),
                    )
                    .map(|l| l.layout)
                } else {
                    boxplot_layout(&c, &source, &profile)
                };
                let error = result.unwrap_err();
                assert!(error.0.contains("opaque effective background"));
                assert_eq!(source, original);
            }
        }
    }
}

#[test]
fn zero_border_empty_null_and_hidden_sources_have_no_invented_context_but_validate_modifiers() {
    let mut c = config();
    c.theme.background = Color(255, 255, 255, 0);
    let o = BoxPlotLayoutOptions {
        border: Some(stroke(0.)),
        border_color_profile: Some(InheritedColor::Contrast {
            dark: Color(0, 0, 0, 255),
            light: Color(255, 255, 255, 255),
        }),
        ..options()
    };
    for scenario in 0..3 {
        let mut profile = o.clone();
        let source = match scenario {
            0 => vec![],
            1 => vec![BoxObservation::new("null", "A", "s", None)],
            _ => {
                profile.hidden.insert(boxplot_series_id("s"));
                data()
            }
        };
        let zero = boxplot_layout(&c, &source, &profile).unwrap();
        let none = boxplot_layout(
            &c,
            &source,
            &BoxPlotLayoutOptions {
                border: None,
                ..profile.clone()
            },
        )
        .unwrap();
        assert_eq!(zero, none);
        profile.border_color_profile =
            Some(InheritedColor::FromContext(vec![ColorModifier::Opacity(
                f64::NAN,
            )]));
        assert!(boxplot_layout(&c, &source, &profile).is_err());
    }
}

#[test]
fn positive_border_width_and_cap_are_preserved_without_changing_other_strokes_or_fill() {
    for horizontal in [false, true] {
        for scaled in [false, true] {
            let o = BoxPlotLayoutOptions {
                border_color_profile: Some(InheritedColor::Fixed(Color(1, 2, 3, 4))),
                ..options()
            };
            let plain = layout(&data(), o.clone(), horizontal, scaled).unwrap();
            let positive = layout(
                &data(),
                BoxPlotLayoutOptions {
                    border: Some(stroke(3.5)),
                    ..o
                },
                horizontal,
                scaled,
            )
            .unwrap();
            let borders = positive
                .scene
                .marks
                .iter()
                .filter(|m| m.id.ends_with(":border"))
                .collect::<Vec<_>>();
            assert_eq!(borders.len(), 1);
            assert_eq!(
                borders[0].stroke,
                Some(GraphicStroke {
                    color: Color(1, 2, 3, 4),
                    ..stroke(3.5)
                })
            );
            assert!(borders[0].fill.is_none());
            assert_eq!(borders[0].opacity, 1.);
            let mut normalized = positive.clone();
            normalized
                .scene
                .marks
                .retain(|m| !m.id.ends_with(":border"));
            assert_eq!(normalized, plain);
        }
    }
}

#[test]
fn zero_border_on_configured_utc_keeps_fractional_statistics_and_query_projection_exact() {
    let source = (0..4)
        .map(|i| BoxObservation::new(i.to_string(), "A", "s", Some(1_600_000_000_000. + i as f64)))
        .collect::<Vec<_>>();
    let scale = ContinuousScaleOptions {
        kind: ScaleKind::Utc,
        reverse: true,
        round: true,
        ..Default::default()
    };
    for orientation in [
        StatisticsOrientation::Vertical,
        StatisticsOrientation::Horizontal,
    ] {
        let o = BoxPlotLayoutOptions {
            orientation,
            border_color_profile: Some(InheritedColor::FromContext(vec![ColorModifier::Opacity(
                0.5,
            )])),
            ..options()
        };
        let none = boxplot_layout_with_scale_options(&config(), &source, &o, &scale).unwrap();
        let zero = boxplot_layout_with_scale_options(
            &config(),
            &source,
            &BoxPlotLayoutOptions {
                border: Some(stroke(0.)),
                ..o
            },
            &scale,
        )
        .expect("configured UTC zero border is disabled");
        assert_eq!(zero.layout, none.layout);
        assert_eq!(
            zero.value_scale.source_domain(),
            none.value_scale.source_domain()
        );
        for q in none.layout.boxes[0].quantiles {
            assert_eq!(
                zero.map_statistic(q).unwrap().to_bits(),
                none.map_statistic(q).unwrap().to_bits()
            );
        }
    }
}

#[test]
fn zero_outlier_border_is_none_and_positive_stroke_preserves_outlier_fill_and_geometry() {
    for horizontal in [false, true] {
        for scaled in [false, true] {
            let o = BoxPlotLayoutOptions {
                outlier_radius: 5.,
                outlier_fill: Some(Color(17, 29, 41, 130)),
                ..options()
            };
            let none = layout(&data(), o.clone(), horizontal, scaled).unwrap();
            let outlier_count = none
                .scene
                .marks
                .iter()
                .filter(|m| m.id.contains(":outlier:"))
                .count();
            assert!(
                outlier_count > 0,
                "this regression must contain real outlier marks"
            );
            for width in [0., -0.] {
                let zero = layout(&data(), BoxPlotLayoutOptions { outlier_border: Some(stroke(width)), ..o.clone() }, horizontal, scaled)
                    .expect("zero outlier stroke must disable border ink instead of creating an invalid Scene mark");
                assert_eq!(zero, none);
                assert_eq!(
                    render::to_svg(&zero.scene).unwrap(),
                    render::to_svg(&none.scene).unwrap()
                );
                assert_eq!(
                    render::to_canvas_script(&zero.scene, "canvas").unwrap(),
                    render::to_canvas_script(&none.scene, "canvas").unwrap()
                );
            }
            let mut positive = layout(
                &data(),
                BoxPlotLayoutOptions {
                    outlier_border: Some(stroke(2.5)),
                    ..o
                },
                horizontal,
                scaled,
            )
            .unwrap();
            let mut actual_count = 0;
            for mark in positive
                .scene
                .marks
                .iter_mut()
                .filter(|m| m.id.contains(":outlier:"))
            {
                actual_count += 1;
                assert_eq!(mark.stroke, Some(stroke(2.5)));
                assert_eq!(mark.fill, Some(Color(17, 29, 41, 130)));
                mark.stroke = None;
            }
            assert_eq!(actual_count, outlier_count);
            assert_eq!(positive, none);
        }
    }
}
