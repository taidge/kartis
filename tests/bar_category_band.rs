use kartis::scales::*;
use kartis::*;
fn near(a: f64, b: f64) {
    assert!((a - b).abs() <= 1e-12, "{a} != {b}");
}
fn config(_horizontal: bool) -> ChartConfig {
    ChartConfig {
        width: 181.,
        height: 181.,
        margin: Margin {
            left: 40.,
            right: 40.,
            top: 40.,
            bottom: 40.,
        },
        ..Default::default()
    }
}
fn profile() -> BandScaleOptions {
    BandScaleOptions {
        padding_inner: 0.2,
        padding_outer: 0.2,
        round: true,
        ..Default::default()
    }
}
#[cfg(feature = "geojson")]
#[test]
fn original_d3_and_nivo_values() {
    let rows: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/bar-category-band/d3-values.json")).unwrap();
    for r in rows.as_array().unwrap() {
        let domain = (0..r["n"].as_u64().unwrap())
            .map(|i| format!("C{i}"))
            .collect();
        let opts = BandScaleOptions {
            padding_inner: r["inner"].as_f64().unwrap(),
            padding_outer: r["outer"].as_f64().unwrap(),
            align: r["align"].as_f64().unwrap(),
            round: r["round"].as_bool().unwrap(),
        };
        let scale = BandScale::with_options(
            domain,
            [
                r["range"][0].as_f64().unwrap(),
                r["range"][1].as_f64().unwrap(),
            ],
            &opts,
        )
        .unwrap();
        near(scale.step(), r["step"].as_f64().unwrap());
        near(scale.bandwidth, r["bandwidth"].as_f64().unwrap());
        for (i, p) in r["positions"].as_array().unwrap().iter().enumerate() {
            near(scale.at(i).unwrap(), p.as_f64().unwrap());
            let center = scale.at(i).unwrap() + scale.bandwidth / 2.;
            if scale.bandwidth > 0. && scale.step() > 0. {
                assert_eq!(scale.invert(center), Some(format!("C{i}").as_str()));
            }
        }
    }
    let rows: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/bar-category-band/nivo-original.json"
    ))
    .unwrap();
    for r in rows["rows"].as_array().unwrap() {
        let size = r["size"].as_f64().unwrap();
        let range = if r["axis"] == "y" {
            [size, 0.]
        } else {
            [0., size]
        };
        let mut p = profile();
        p.round = r["round"].as_bool().unwrap();
        let scale =
            BandScale::with_options(["A", "B", "C"].map(str::to_string).to_vec(), range, &p)
                .unwrap();
        near(scale.step(), r["step"].as_f64().unwrap());
        near(scale.bandwidth, r["bandwidth"].as_f64().unwrap());
        for (i, v) in r["positions"].as_array().unwrap().iter().enumerate() {
            near(scale.at(i).unwrap(), v.as_f64().unwrap());
        }
    }
}
#[test]
fn configured_bar_mapping_is_shared_for_every_orientation_and_layout() -> Result<()> {
    let cats = ["A", "B", "C"].map(str::to_string);
    let data = [
        BarSeries::new("z", vec![Some(0.), None, Some(-3.)]),
        BarSeries::new("a", vec![Some(2.), Some(4.), Some(6.)]),
    ];
    for reverse in [false, true] {
        for horizontal in [false, true] {
            for layout in [BarLayout::Grouped, BarLayout::Stacked, BarLayout::Percent] {
                for window in [None, Some([0.1, 0.9])] {
                    let mut o = BarOptions {
                        horizontal,
                        layout,
                        category_window: window,
                        ..Default::default()
                    };
                    o.category_axis.grid = true;
                    let style = BarStyleOptions {
                        totals: BarTotals {
                            enabled: true,
                            ..Default::default()
                        },
                        category_scale: Some(profile()),
                        category_reverse: reverse,
                        inner_padding: 2.,
                        ..Default::default()
                    };
                    let l = bar_layout_with_style(&config(horizontal), &cats, &data, &o, &style)?;
                    let scale = l.category_scale.as_ref().unwrap();
                    near(l.category_step, scale.step());
                    near(l.category_bandwidth, scale.bandwidth);
                    near(
                        scale.at(0).unwrap(),
                        if reverse {
                            141. + (-32. + 101. * window.unwrap_or([0., 1.])[0])
                                / (window.unwrap_or([0., 1.])[1] - window.unwrap_or([0., 1.])[0])
                        } else {
                            40. + (7. - 101. * window.unwrap_or([0., 1.])[0])
                                / (window.unwrap_or([0., 1.])[1] - window.unwrap_or([0., 1.])[0])
                        },
                    );
                    for b in &l.bars {
                        assert_eq!(b.category, cats[b.source_category]);
                        assert_eq!(b.series, data[b.source_series].id);
                        assert_eq!(
                            b.raw_value,
                            data[b.source_series].values[b.source_category].unwrap()
                        );
                        let groups = if layout == BarLayout::Grouped { 2. } else { 1. };
                        let breadth =
                            (scale.bandwidth - if groups == 2. { 2. } else { 0. }) / groups;
                        let slot = if b.series == "a" { 0. } else { 1. };
                        let expected = scale.at(b.source_category).unwrap()
                            + if groups == 2. {
                                slot * (breadth + 2.)
                            } else {
                                0.
                            };
                        near(b.band[0], expected);
                        near(b.band[1], breadth);
                    }
                    for (i, cat) in cats.iter().enumerate() {
                        let center = scale.at(i).unwrap() + scale.bandwidth / 2.;
                        let label = l
                            .scene
                            .labels
                            .iter()
                            .find(|label| label.text == *cat)
                            .unwrap();
                        near(
                            if horizontal {
                                label.y - config(horizontal).theme.font_size / 3.
                            } else {
                                label.x
                            },
                            center,
                        );
                        assert!(
                            l.scene
                                .marks
                                .iter()
                                .any(|m| m.id == format!("category-grid-{i}"))
                        );
                    }
                    for total in &l.totals {
                        if let Some(p) = total.position {
                            near(
                                p[usize::from(horizontal)],
                                scale.map(&total.category).unwrap() + scale.bandwidth / 2.,
                            );
                        }
                    }
                    assert!(!l.bars.iter().any(|b| b.series == "z" && b.category == "B"));
                    assert!(
                        l.bars
                            .iter()
                            .any(|b| b.series == "z" && b.category == "A" && b.raw_value == 0.)
                    );
                    let svg = render::to_svg(&l.scene)?;
                    let canvas = render::to_canvas_script(&l.scene, "bands")?;
                    assert!(svg.contains("data-value=\"0\""));
                    assert!(canvas.contains("fillRect") || canvas.contains("rect("));
                    let mut hidden = o.clone();
                    hidden.hidden.insert("a".into());
                    let h =
                        bar_layout_with_style(&config(horizontal), &cats, &data, &hidden, &style)?;
                    assert!(
                        h.bars
                            .iter()
                            .all(|b| b.series == "z" && b.source_series == 0)
                    );
                    assert_eq!(h.category_scale, l.category_scale);
                }
            }
        }
    }
    Ok(())
}
#[test]
fn rejected_options_and_window_do_not_mutate_inputs() {
    let keys = vec!["A".into()];
    for bad in [
        BandScaleOptions {
            align: f64::NAN,
            ..profile()
        },
        BandScaleOptions {
            padding_inner: 1.1,
            ..profile()
        },
        BandScaleOptions {
            padding_outer: -0.1,
            ..profile()
        },
    ] {
        assert!(BandScale::with_options(keys.clone(), [0., 101.], &bad).is_err());
    }
    let scale = BandScale::with_options(keys.clone(), [0., 101.], &profile()).unwrap();
    assert!(scale.clone().windowed([0.5, 0.5]).is_err());
    assert_eq!(scale.options(), Some(&profile()));
    let data = [BarSeries::new("s", vec![Some(1.)])];
    let conflict = BarStyleOptions {
        category_scale: Some(profile()),
        category_inner_padding: Some(0.2),
        ..Default::default()
    };
    assert!(
        bar_layout_with_style(&config(false), &keys, &data, &Default::default(), &conflict)
            .is_err()
    );
    assert_eq!(data[0].values, vec![Some(1.)]);
}

#[test]
fn zero_bandwidth_and_configured_value_scale_are_supported() -> Result<()> {
    let cats = ["A", "B", "C"].map(str::to_string);
    let data = [BarSeries::new("s", vec![Some(0.), Some(2.), None])];
    let style = BarStyleOptions {
        category_scale: Some(BandScaleOptions {
            padding_inner: 1.,
            ..profile()
        }),
        ..Default::default()
    };
    let l = bar_layout_with_style(&config(false), &cats, &data, &Default::default(), &style)?;
    near(l.category_scale.as_ref().unwrap().bandwidth, 0.);
    let scaled = bar_layout_with_scale_options(
        &config(false),
        &cats,
        &data,
        &Default::default(),
        &style,
        &ContinuousScaleOptions::default(),
    )?;
    assert_eq!(l.category_scale, scaled.layout.category_scale);
    let p = BandScale::with_options(cats.to_vec(), [-20., 81.], &profile())?;
    let zoom = p.clone().windowed([0.1, 0.9])?.windowed([0.2, 0.8])?;
    assert_eq!(zoom, p.windowed([0.2, 0.8])?);
    Ok(())
}
