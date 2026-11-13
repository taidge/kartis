use std::collections::BTreeMap;

use kartis::*;
fn frame() -> ChartConfig {
    ChartConfig {
        width: 400.,
        height: 300.,
        margin: Margin {
            left: 40.,
            right: 40.,
            top: 40.,
            bottom: 40.,
        },
        ..Default::default()
    }
}
fn cats() -> Vec<String> {
    ["A", "B", "C"].map(str::to_string).to_vec()
}
fn source() -> Vec<BarSeries> {
    vec![
        BarSeries::new("alpha", vec![Some(7.), Some(-4.), None]),
        BarSeries::new("beta", vec![Some(-2.), Some(-6.), Some(3.)]),
        BarSeries::new("gamma", vec![Some(4.), Some(2.), Some(0.)]),
    ]
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}")
}
#[test]
fn independent_band_padding_and_group_gap_match_the_d3_band_equation() {
    let o = BarOptions {
        value_domain: Some([-10., 15.]),
        ..Default::default()
    };
    let style = BarStyleOptions {
        category_inner_padding: Some(0.2),
        category_outer_padding: Some(0.5),
        inner_padding: 8.,
        ..Default::default()
    };
    let l = bar_layout_with_style(&frame(), &cats(), &source(), &o, &style).unwrap();
    let step = 320. / (3. - 0.2 + 2. * 0.5);
    close(l.category_step, step);
    close(l.category_bandwidth, step * 0.8);
    let a = l
        .bars
        .iter()
        .find(|d| d.series == "alpha" && d.category == "A")
        .unwrap();
    let b = l
        .bars
        .iter()
        .find(|d| d.series == "beta" && d.category == "A")
        .unwrap();
    close(a.band[0], 40. + step * 0.5);
    close(a.band[1], (step * 0.8 - 16.) / 3.);
    close(b.band[0] - a.band[0] - a.band[1], 8.);
    let legacy = bar(&frame(), &cats(), &source(), &o).unwrap();
    let styled =
        bar_layout_with_style(&frame(), &cats(), &source(), &o, &Default::default()).unwrap();
    let Geometry::Rect {
        x,
        y,
        width,
        height,
    } = legacy
        .marks
        .iter()
        .find(|m| m.id == "bar-0-0")
        .unwrap()
        .geometry
    else {
        panic!()
    };
    let r = styled
        .bars
        .iter()
        .find(|d| d.series == "alpha" && d.category == "A")
        .unwrap()
        .visible_rect;
    assert_eq!(
        r,
        ClipRect {
            x,
            y,
            width,
            height
        }
    );
}
#[test]
fn totals_conserve_raw_quantities_and_use_the_real_positive_or_negative_extent() {
    let mut style = BarStyleOptions {
        totals: BarTotals {
            enabled: true,
            mode: BarTotalMode::PositiveNegative,
            ..Default::default()
        },
        ..Default::default()
    };
    for horizontal in [false, true] {
        for layout in [BarLayout::Grouped, BarLayout::Stacked, BarLayout::Percent] {
            let o = BarOptions {
                horizontal,
                layout,
                ..Default::default()
            };
            let l = bar_layout_with_style(&frame(), &cats(), &source(), &o, &style).unwrap();
            let p = l
                .totals
                .iter()
                .find(|t| t.category == "A" && t.kind == BarTotalKind::Positive)
                .unwrap();
            let n = l
                .totals
                .iter()
                .find(|t| t.category == "A" && t.kind == BarTotalKind::Negative)
                .unwrap();
            close(p.value, 11.);
            close(n.value, -2.);
            close(
                p.anchor_value,
                match layout {
                    BarLayout::Grouped => 7.,
                    BarLayout::Stacked => 11.,
                    BarLayout::Percent => 1.,
                },
            );
            close(
                n.anchor_value,
                if layout == BarLayout::Percent {
                    -1.
                } else {
                    -2.
                },
            );
            if layout == BarLayout::Percent {
                close(p.displayed_value.unwrap(), 1.);
                close(n.displayed_value.unwrap(), -1.);
            }
            let center = if horizontal {
                p.position.unwrap()[1]
            } else {
                p.position.unwrap()[0]
            };
            assert!(center > 40. && center < 360.);
        }
    }
    style.totals.mode = BarTotalMode::Net;
    let l = bar_layout_with_style(
        &frame(),
        &cats(),
        &source(),
        &BarOptions {
            layout: BarLayout::Stacked,
            ..Default::default()
        },
        &style,
    )
    .unwrap();
    let a = &l.totals[0];
    close(a.raw_value.unwrap(), 9.);
    close(a.anchor_value, 11.);
    let negative = [
        BarSeries::new("one", vec![Some(-4.)]),
        BarSeries::new("two", vec![Some(-2.)]),
    ];
    let l = bar_layout_with_style(
        &frame(),
        &["negative".into()],
        &negative,
        &BarOptions::default(),
        &style,
    )
    .unwrap();
    close(l.totals[0].value, -6.);
    close(l.totals[0].anchor_value, -4.);
    assert!(l.totals[0].position.unwrap()[1] > 260.);
}
#[test]
fn labels_follow_visual_start_middle_end_and_keep_raw_values_under_percent_geometry() {
    let d = [BarSeries::new("value", vec![Some(5.)])];
    let categories = ["A".into()];
    for horizontal in [false, true] {
        for position in [
            BarLabelPosition::Start,
            BarLabelPosition::Middle,
            BarLabelPosition::End,
        ] {
            let style = BarStyleOptions {
                labels: BarLabels {
                    enabled: true,
                    position,
                    offset: 0.,
                    format: ValueFormat::Affix {
                        prefix: "$".into(),
                        suffix: String::new(),
                        precision: 1,
                    },
                    ..Default::default()
                },
                ..Default::default()
            };
            let o = BarOptions {
                horizontal,
                value_domain: Some([0., 10.]),
                ..Default::default()
            };
            let l = bar_layout_with_style(&frame(), &categories, &d, &o, &style).unwrap();
            let b = &l.bars[0];
            let label = &l.scene.labels[b.label_index.unwrap()];
            assert_eq!(label.text, "$5.0");
            assert_eq!(l.scene.label_series[&b.label_index.unwrap()], "value");
            assert!(l.scene.label_clips.contains_key(&b.label_index.unwrap()));
            let r = b.visible_rect;
            if horizontal {
                close(
                    label.x,
                    r.x + match position {
                        BarLabelPosition::Start => 0.,
                        BarLabelPosition::Middle => r.width / 2.,
                        BarLabelPosition::End => r.width,
                    },
                );
                close(label.y, r.y + r.height / 2.);
            } else {
                close(label.x, r.x + r.width / 2.);
                close(
                    label.y,
                    r.y + match position {
                        BarLabelPosition::Start => r.height,
                        BarLabelPosition::Middle => r.height / 2.,
                        BarLabelPosition::End => 0.,
                    },
                );
            }
        }
    }
    let mut style = BarStyleOptions {
        labels: BarLabels {
            enabled: true,
            ..Default::default()
        },
        totals: BarTotals {
            enabled: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let o = BarOptions {
        layout: BarLayout::Percent,
        ..Default::default()
    };
    let l = bar_layout_with_style(&frame(), &categories, &d, &o, &style).unwrap();
    assert_eq!(l.scene.labels[l.bars[0].label_index.unwrap()].text, "5");
    close(l.totals[0].value, 5.);
    style.labels.value = BarLabelValue::Displayed;
    style.labels.format = ValueFormat::Percent(0);
    style.totals.value = BarLabelValue::Displayed;
    let l = bar_layout_with_style(&frame(), &categories, &d, &o, &style).unwrap();
    assert_eq!(l.scene.labels[l.bars[0].label_index.unwrap()].text, "100%");
    close(l.totals[0].value, 1.);
    style.labels.skip_width = 400.;
    assert!(
        bar_layout_with_style(&frame(), &categories, &d, &o, &style)
            .unwrap()
            .bars[0]
            .label_index
            .is_none()
    );
}
#[test]
fn hidden_relayout_recomputes_slots_totals_and_owned_labels_without_changing_identity() {
    let style = BarStyleOptions {
        labels: BarLabels {
            enabled: true,
            ..Default::default()
        },
        totals: BarTotals {
            enabled: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut o = BarOptions {
        value_domain: Some([-12., 12.]),
        ..Default::default()
    };
    let before = bar_layout_with_style(&frame(), &cats(), &source(), &o, &style).unwrap();
    o.hidden.insert("beta".into());
    let after = bar_layout_with_style(&frame(), &cats(), &source(), &o, &style).unwrap();
    let a = before
        .bars
        .iter()
        .find(|b| b.category == "A" && b.series == "alpha")
        .unwrap();
    let b = after.bars.iter().find(|b| b.id == a.id).unwrap();
    close(b.band[1], a.band[1] * 1.5);
    assert!(!after.scene.label_series.values().any(|s| s == "beta"));
    assert_eq!(after.scene.legend.len(), 3);
    close(before.totals[0].value, 9.);
    close(after.totals[0].value, 11.);
    let mut rows = source();
    rows.reverse();
    let reordered = bar_layout_with_style(&frame(), &cats(), &rows, &o, &style).unwrap();
    for m in &after.bars {
        let r = reordered.bars.iter().find(|r| r.id == m.id).unwrap();
        assert_eq!(m.band, r.band);
        assert_eq!(m.color, r.color);
        assert_eq!(m.interval, r.interval);
    }
    let category = BarStyleOptions {
        color_by: BarColorBy::Category,
        colors: BTreeMap::from([("A".into(), Color(10, 20, 30, 255))]),
        ..style
    };
    let l = bar_layout_with_style(&frame(), &cats(), &source(), &o, &category).unwrap();
    assert!(
        l.bars
            .iter()
            .filter(|b| b.category == "A")
            .all(|b| b.color == Color(10, 20, 30, 255))
    );
}
#[test]
fn category_and_value_crops_keep_original_rounding_and_suppress_offrange_endpoint_labels() {
    let categories = ["A".into()];
    let d = [BarSeries::new("s", vec![Some(8.)])];
    let style = BarStyleOptions {
        labels: BarLabels {
            enabled: true,
            position: BarLabelPosition::End,
            ..Default::default()
        },
        ..Default::default()
    };
    let o = BarOptions {
        padding: 0.,
        corner_radius: 20.,
        category_window: Some([0.25, 0.75]),
        value_domain: Some([0., 10.]),
        ..Default::default()
    };
    let l = bar_layout_with_style(&frame(), &categories, &d, &o, &style).unwrap();
    let b = &l.bars[0];
    close(b.band[0], -120.);
    close(b.band[1], 640.);
    let top = b.visible_rect.y;
    assert_eq!(
        l.scene.hit_test(41., top + 1.).map(|m| m.id.as_str()),
        Some(b.id.as_str())
    );
    assert!(l.scene.hit_test(39., top + 1.).is_none_or(|m| m.id != b.id));
    let o = BarOptions {
        value_domain: Some([2., 6.]),
        ..o
    };
    let l = bar_layout_with_style(&frame(), &categories, &d, &o, &style).unwrap();
    assert!(l.bars[0].label_index.is_none());
    assert_eq!(
        l.scene.hit_test(41., 41.).map(|m| m.id.as_str()),
        Some(l.bars[0].id.as_str())
    );
}
#[test]
fn large_independent_grouped_values_and_finite_cancelling_totals_do_not_overflow() {
    let d = [
        BarSeries::new("a", vec![Some(f64::MAX * 0.75)]),
        BarSeries::new("b", vec![Some(f64::MAX * 0.75)]),
    ];
    let o = BarOptions {
        value_domain: Some([0., f64::MAX]),
        ..Default::default()
    };
    assert_eq!(
        bar_layout_with_style(&frame(), &["A".into()], &d, &o, &Default::default())
            .unwrap()
            .bars
            .len(),
        2
    );
    let mut s = BarStyleOptions {
        totals: BarTotals {
            enabled: true,
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(bar_with_style(&frame(), &["A".into()], &d, &o, &s).is_err());
    s.totals.value = BarLabelValue::Displayed;
    let l = bar_layout_with_style(
        &frame(),
        &["A".into()],
        &d,
        &BarOptions {
            layout: BarLayout::Percent,
            ..Default::default()
        },
        &s,
    )
    .unwrap();
    close(l.totals[0].value, 1.);
    assert_eq!(l.totals[0].raw_value, None);
    let d = [
        BarSeries::new("a", vec![Some(f64::MAX)]),
        BarSeries::new("b", vec![Some(f64::MAX)]),
        BarSeries::new("c", vec![Some(-f64::MAX)]),
    ];
    s.totals.value = BarLabelValue::Raw;
    let l = bar_layout_with_style(&frame(), &["A".into()], &d, &o, &s).unwrap();
    assert_eq!(l.totals[0].raw_value, Some(f64::MAX));
}
#[test]
fn bad_styles_domains_and_output_reject_without_changing_inputs_or_an_accepted_scene() {
    let d = source();
    let mut style = BarStyleOptions::default();
    let accepted = bar_with_style(&frame(), &cats(), &d, &BarOptions::default(), &style).unwrap();
    let saved = accepted.clone();
    style.inner_padding = 400.;
    assert!(bar_with_style(&frame(), &cats(), &d, &BarOptions::default(), &style).is_err());
    assert_eq!(accepted, saved);
    assert_eq!(d, source());
    style.inner_padding = 0.;
    style.colors.insert("unknown".into(), Color(0, 0, 0, 255));
    assert!(bar_with_style(&frame(), &cats(), &d, &BarOptions::default(), &style).is_err());
    style.colors.clear();
    style.category_outer_padding = Some(f64::NAN);
    assert!(bar_with_style(&frame(), &cats(), &d, &BarOptions::default(), &style).is_err());
    let categories = (0..11000).map(|i| format!("c{i}")).collect::<Vec<_>>();
    let d = [BarSeries::new("one", vec![Some(1.); 11000])];
    let style = BarStyleOptions {
        labels: BarLabels {
            enabled: true,
            ..Default::default()
        },
        totals: BarTotals {
            enabled: true,
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(bar_with_style(&frame(), &categories, &d, &BarOptions::default(), &style).is_err());
    let l = bar_with_style(
        &frame(),
        &categories,
        &d,
        &BarOptions {
            category_window: Some([0., 0.001]),
            ..Default::default()
        },
        &style,
    )
    .unwrap();
    assert!(l.labels.len() < 100);
    assert!(l.marks.len() < 200);
}
#[cfg(feature = "raster")]
#[test]
fn cpu_pixels_show_group_gap_colors_and_original_corner_at_a_clipped_edge() {
    use kartis::render::raster::*;
    let mut c = frame();
    c.theme.grid = Color(255, 255, 255, 255);
    let mut o = BarOptions {
        padding: 0.,
        value_domain: Some([0., 10.]),
        ..Default::default()
    };
    o.value_axis.visible = false;
    o.value_axis.grid = false;
    o.category_axis.visible = false;
    let d = [
        BarSeries::new("red", vec![Some(8.)]),
        BarSeries::new("blue", vec![Some(8.)]),
    ];
    let mut s = BarStyleOptions {
        inner_padding: 20.,
        colors: BTreeMap::from([
            ("red".into(), Color(255, 0, 0, 255)),
            ("blue".into(), Color(0, 0, 255, 255)),
        ]),
        ..Default::default()
    };
    let l = bar_layout_with_style(&c, &["A".into()], &d, &o, &s).unwrap();
    let image = render_raster(&l.scene, &FontRegistry::new(), &Default::default()).unwrap();
    let pixel = |x: u32, y: u32| {
        &image.rgba[((y * image.width + x) * 4) as usize..((y * image.width + x) * 4 + 4) as usize]
    };
    assert_eq!(pixel(100, 150), [0, 0, 255, 255]);
    assert_eq!(pixel(200, 150), [255, 255, 255, 255]);
    assert_eq!(pixel(300, 150), [255, 0, 0, 255]);
    o.corner_radius = 20.;
    o.category_window = Some([0.25, 0.75]);
    let one = [BarSeries::new("red", vec![Some(8.)])];
    s.colors.remove("blue");
    let scene = bar_with_style(&c, &["A".into()], &one, &o, &s).unwrap();
    let image = render_raster(&scene, &FontRegistry::new(), &Default::default()).unwrap();
    let i = ((85 * image.width + 41) * 4) as usize;
    assert_eq!(&image.rgba[i..i + 4], &[255, 0, 0, 255]);
}
