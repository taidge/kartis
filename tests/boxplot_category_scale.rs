use kartis::*;
fn config() -> ChartConfig {
    ChartConfig {
        width: 221.,
        height: 240.,
        margin: Margin {
            left: 60.,
            right: 60.,
            top: 24.,
            bottom: 24.,
        },
        ..Default::default()
    }
}
fn observations() -> Vec<BoxObservation> {
    let mut data = vec![];
    for group in ["A", "B", "C"] {
        for (i, value) in [1., 2., 3.].into_iter().enumerate() {
            data.push(BoxObservation::new(
                format!("{group}/{i}"),
                group,
                "",
                Some(value),
            ));
        }
    }
    data
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() <= 1e-12, "{a} != {b}");
}
#[test]
fn original_d3_band_profiles_drive_axes_boxes_and_outliers() -> Result<()> {
    let mut count = 0;
    for line in include_str!("fixtures/boxplot-category-band.csv")
        .lines()
        .skip(1)
    {
        let f: Vec<_> = line.split(',').collect();
        let horizontal = f[0] == "1";
        let reverse = f[1] == "1";
        let round = f[2] == "1";
        let options = scales::BandScaleOptions {
            padding_inner: f[3].parse().unwrap(),
            padding_outer: f[4].parse().unwrap(),
            align: f[5].parse().unwrap(),
            round,
        };
        let expected_step: f64 = f[6].parse().unwrap();
        let expected_band: f64 = f[7].parse().unwrap();
        let starts: Vec<f64> = f[8..11].iter().map(|s| s.parse().unwrap()).collect();
        let style = BoxPlotLayoutOptions {
            orientation: if horizontal {
                StatisticsOrientation::Horizontal
            } else {
                StatisticsOrientation::Vertical
            },
            category_scale: Some(options),
            category_reverse: reverse,
            ..Default::default()
        };
        let layout = boxplot_layout(&config(), &observations(), &style)?;
        let band = layout.category_scale.as_ref().unwrap();
        assert_eq!(band.options(), Some(&options));
        close(band.step(), expected_step);
        close(band.bandwidth, expected_band);
        for (i, g) in ["A", "B", "C"].iter().enumerate() {
            close(band.map(g).unwrap(), starts[i]);
            let center = starts[i] + expected_band / 2.;
            let b = layout.boxes.iter().find(|b| &b.group == g).unwrap();
            close(b.center, center);
            close(b.breadth, expected_band);
            close(
                if horizontal {
                    b.rect.y + b.rect.height / 2.
                } else {
                    b.rect.x + b.rect.width / 2.
                },
                center,
            );
            let axis = layout.scene.labels.iter().find(|l| &l.text == g).unwrap();
            close(
                if horizontal {
                    axis.y - config().theme.font_size / 3.
                } else {
                    axis.x
                },
                center,
            );
            for mark in layout
                .scene
                .marks
                .iter()
                .filter(|m| m.id.starts_with(&format!("{}:outlier:", b.id)))
            {
                let Geometry::Circle { x, y, .. } = mark.geometry else {
                    panic!("circle outlier expected")
                };
                close(if horizontal { y } else { x }, center);
            }
        }
        count += 1;
    }
    assert_eq!(count, 96);
    Ok(())
}
#[test]
fn parent_101_span_witness_uses_same_axis_and_body_centres() -> Result<()> {
    let style = BoxPlotLayoutOptions {
        category_scale: Some(scales::BandScaleOptions {
            padding_inner: 0.1,
            padding_outer: 0.1,
            align: 0.5,
            round: true,
        }),
        inner_padding: 0.,
        ..Default::default()
    };
    let l = boxplot_layout(&config(), &observations(), &style)?;
    for (b, c) in l.boxes.iter().zip([78.5, 110.5, 142.5]) {
        assert_eq!(b.center, c);
        assert_eq!(b.breadth, 29.);
        assert_eq!(
            l.scene.labels.iter().find(|x| x.text == b.group).unwrap().x,
            c
        );
    }
    let old = boxplot_layout(&config(), &observations(), &Default::default())?;
    assert!(old.category_scale.is_none());
    for (b, c) in old
        .boxes
        .iter()
        .zip([77.91935483870968, 110.5, 143.0806451612903])
    {
        close(b.center, c);
    }
    Ok(())
}
#[test]
fn subgroup_gap_and_hidden_reflow_do_not_round_twice_or_drop_empty_groups() -> Result<()> {
    let mut data = vec![];
    for g in ["A", "B", "C"] {
        for s in ["p", "q"] {
            data.push(BoxObservation::new(format!("{g}{s}"), g, s, Some(2.)));
        }
    }
    let mut o = BoxPlotLayoutOptions {
        groups: Some(["C", "A", "B", "empty"].map(str::to_string).to_vec()),
        subgroups: Some(["q", "p"].map(str::to_string).to_vec()),
        category_scale: Some(scales::BandScaleOptions {
            padding_inner: 0.2,
            padding_outer: 0.3,
            align: 0.5,
            round: true,
        }),
        inner_padding: 1.7,
        ..Default::default()
    };
    let both = boxplot_layout(&config(), &data, &o)?;
    let band = both.category_scale.as_ref().unwrap();
    let bw = (band.bandwidth - 1.7) / 2.;
    for g in ["A", "B", "C"] {
        for (s, index) in [("q", 0), ("p", 1)] {
            let b = both
                .boxes
                .iter()
                .find(|b| b.group == g && b.subgroup == s)
                .unwrap();
            close(b.breadth, bw);
            close(
                b.center,
                band.map(g).unwrap() + index as f64 * (bw + 1.7) + bw / 2.,
            );
        }
    }
    assert!(!both.boxes.iter().any(|b| b.group == "empty"));
    o.hidden.insert(boxplot_series_id("p"));
    let hidden = boxplot_layout(&config(), &data, &o)?;
    assert_eq!(hidden.category_scale, both.category_scale);
    assert_eq!(hidden.groups, both.groups);
    for b in &hidden.boxes {
        assert_eq!(b.subgroup, "q");
        close(b.breadth, band.bandwidth);
        close(b.center, band.map(&b.group).unwrap() + band.bandwidth / 2.);
    }
    Ok(())
}
#[test]
fn zero_band_gap_and_invalid_profiles_reject_without_dropping_validation() -> Result<()> {
    for hidden in [false, true] {
        let mut o = BoxPlotLayoutOptions {
            category_scale: Some(scales::BandScaleOptions {
                padding_inner: 1.,
                ..Default::default()
            }),
            ..Default::default()
        };
        if hidden {
            o.hidden.insert(boxplot_series_id(""));
        }
        let result = boxplot_layout(&config(), &observations(), &o);
        if hidden {
            let empty = result?;
            assert!(empty.boxes.is_empty());
            assert_eq!(empty.category_scale.unwrap().bandwidth, 0.);
        } else {
            assert!(result.unwrap_err().0.contains("subgroup gap"));
        }
    }
    let source = observations();
    for empty in [false, true] {
        for bad in [
            scales::BandScaleOptions {
                align: f64::NAN,
                ..Default::default()
            },
            scales::BandScaleOptions {
                align: 1.1,
                ..Default::default()
            },
            scales::BandScaleOptions {
                padding_outer: 4.1,
                ..Default::default()
            },
            scales::BandScaleOptions {
                padding_inner: 1.1,
                ..Default::default()
            },
        ] {
            let o = BoxPlotLayoutOptions {
                category_scale: Some(bad),
                ..Default::default()
            };
            assert!(boxplot_layout(&config(), if empty { &[] } else { &source }, &o).is_err());
        }
    }
    let reversal = BoxPlotLayoutOptions {
        category_reverse: true,
        ..Default::default()
    };
    assert!(boxplot_layout(&config(), &[], &reversal).is_err());
    let mut o = BoxPlotLayoutOptions {
        category_scale: Some(Default::default()),
        ..Default::default()
    };
    o.padding = f64::NAN;
    assert!(boxplot_layout(&config(), &[], &o).is_err());
    let data = [
        BoxObservation::new("p", "A", "p", Some(1.)),
        BoxObservation::new("q", "A", "q", Some(2.)),
    ];
    let o = BoxPlotLayoutOptions {
        category_scale: Some(scales::BandScaleOptions {
            padding_inner: 0.1,
            padding_outer: 0.1,
            align: 0.5,
            round: true,
        }),
        inner_padding: 100.,
        ..Default::default()
    };
    assert!(boxplot_layout(&config(), &data, &o).is_err());
    Ok(())
}
#[test]
fn configured_value_scale_and_group_direction_share_separate_metadata() -> Result<()> {
    for horizontal in [false, true] {
        for reverse in [false, true] {
            let o = BoxPlotLayoutOptions {
                orientation: if horizontal {
                    StatisticsOrientation::Horizontal
                } else {
                    StatisticsOrientation::Vertical
                },
                category_scale: Some(scales::BandScaleOptions {
                    padding_inner: 0.1,
                    padding_outer: 0.1,
                    align: 0.5,
                    round: true,
                }),
                category_reverse: reverse,
                ..Default::default()
            };
            let l = boxplot_layout_with_scale_options(
                &config(),
                &observations(),
                &o,
                &ContinuousScaleOptions {
                    kind: ScaleKind::SymLog { constant: 0.4 },
                    reverse: true,
                    round: true,
                    ..Default::default()
                },
            )?;
            let scale = l.layout.category_scale.as_ref().unwrap();
            let range = if horizontal { [24., 216.] } else { [60., 161.] };
            assert_eq!(
                scale.range(),
                if reverse { [range[1], range[0]] } else { range }
            );
            assert_eq!(l.layout.boxes.len(), 3);
            for b in &l.layout.boxes {
                close(
                    b.center,
                    scale.map(&b.group).unwrap() + scale.bandwidth / 2.,
                );
            }
        }
    }
    Ok(())
}
