use std::collections::{BTreeMap, BTreeSet};

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
        theme: Theme {
            colors: vec![Color(255, 0, 0, 255), Color(0, 0, 255, 255)],
            ..Default::default()
        },
        ..Default::default()
    }
}
fn categories() -> Vec<String> {
    ["A", "B", "C", "D"].map(str::to_string).to_vec()
}
fn source() -> Vec<BarSeries> {
    vec![
        BarSeries::new("z", vec![None, Some(0.), Some(5.), None]),
        BarSeries::new("a", vec![Some(2.), None, Some(-1.), None]),
    ]
}
fn options() -> BarOptions {
    BarOptions {
        value_domain: Some([-2., 8.]),
        ..Default::default()
    }
}
fn style() -> BarStyleOptions {
    BarStyleOptions {
        legend_source: BarLegendSource::Indexes,
        series_order: Some(vec!["z".into(), "a".into()]),
        colors: BTreeMap::from([
            ("z".into(), Color(255, 0, 0, 255)),
            ("a".into(), Color(0, 0, 255, 255)),
        ]),
        ..Default::default()
    }
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() <= 1e-12, "{a} != {b}");
}
fn css(s: &str) -> Color {
    match s {
        "#ff0000" => Color(255, 0, 0, 255),
        "#0000ff" => Color(0, 0, 255, 255),
        _ => panic!("unexpected native color {s}"),
    }
}

#[test]
fn actual_original_nivo_unsorted_source_bars_and_index_legends_match_32_profiles() {
    let fixture = include_str!("fixtures/bar-index-legend/values.txt");
    let mut layouts = BTreeMap::new();
    for line in fixture.lines().filter(|l| l.starts_with("H|")) {
        let f = line.split('|').collect::<Vec<_>>();
        let id = f[1].parse::<usize>().unwrap();
        let mut o = options();
        o.layout = if f[2] == "grouped" {
            BarLayout::Grouped
        } else {
            BarLayout::Stacked
        };
        o.horizontal = f[3] == "1";
        if !f[7].is_empty() {
            o.hidden = f[7].split(',').map(str::to_string).collect();
        }
        let mut s = style();
        s.series_order = Some(f[6].split(',').map(str::to_string).collect());
        s.category_scale = Some(scales::BandScaleOptions {
            padding_inner: 0.2,
            padding_outer: 0.2,
            round: f[5] == "1",
            ..Default::default()
        });
        s.category_reverse = o.horizontal;
        let p = ContinuousScaleOptions {
            reverse: f[4] == "1",
            round: f[5] == "1",
            ..Default::default()
        };
        layouts.insert(
            id,
            bar_layout_with_scale_options(&frame(), &categories(), &source(), &o, &s, &p).unwrap(),
        );
    }
    assert_eq!(layouts.len(), 32);
    let mut counts = [0; 3];
    let mut legend_slots = BTreeMap::<usize, usize>::new();
    let mut actual_bars = BTreeMap::<usize, usize>::new();
    for line in fixture
        .lines()
        .filter(|l| l.starts_with("L|") || l.starts_with("B|"))
    {
        let f = line.split('|').collect::<Vec<_>>();
        let id = f[1].parse::<usize>().unwrap();
        let l = &layouts[&id].layout;
        if f[0] == "L" {
            let slot = legend_slots.entry(id).or_default();
            let item = &l.scene.legend[*slot];
            *slot += 1;
            assert_eq!(item.id, f[2]);
            assert_eq!(item.label, f[3]);
            assert_eq!(item.color, css(f[4]));
            assert_eq!(l.scene.legend_roles[&item.id], LegendRole::Category);
            counts[0] += 1;
        } else if f[4] == "null" {
            assert!(
                !l.bars
                    .iter()
                    .any(|b| b.series == f[2] && b.category == f[3])
            );
            counts[1] += 1;
        } else {
            let b = l
                .bars
                .iter()
                .find(|b| b.series == f[2] && b.category == f[3])
                .unwrap();
            near(b.raw_value, f[4].parse().unwrap());
            let r = b.visible_rect;
            for (a, e) in [r.x, r.y, r.width, r.height].into_iter().zip(&f[5..9]) {
                near(a, e.parse().unwrap());
            }
            assert_eq!(b.color, css(f[9]));
            assert_eq!(b.source_series, usize::from(f[2] == "a"));
            assert_eq!(
                b.source_category,
                categories().iter().position(|c| c == f[3]).unwrap()
            );
            *actual_bars.entry(id).or_default() += 1;
            counts[2] += 1;
        }
    }
    for (id, l) in &layouts {
        assert_eq!(legend_slots[id], l.layout.scene.legend.len());
        assert_eq!(actual_bars[id], l.layout.bars.len());
    }
    assert!(
        counts[0] > 100 && counts[1] > 100 && counts[2] > 100,
        "{counts:?}"
    );
}

#[test]
fn authored_order_drives_actual_group_stack_percent_palette_and_keys_without_changing_origins() {
    let c = frame();
    let cats = vec!["only".into()];
    let d = vec![
        BarSeries::new("z", vec![Some(2.)]),
        BarSeries::new("a", vec![Some(3.)]),
    ];
    for mode in [BarLayout::Grouped, BarLayout::Stacked, BarLayout::Percent] {
        let o = BarOptions {
            layout: mode,
            ..Default::default()
        };
        let old = bar_layout_with_style(&c, &cats, &d, &o, &Default::default()).unwrap();
        assert_eq!(
            old.bars
                .iter()
                .map(|b| b.series.as_str())
                .collect::<Vec<_>>(),
            ["a", "z"]
        );
        let s = BarStyleOptions {
            series_order: Some(vec!["z".into(), "a".into()]),
            ..Default::default()
        };
        let new = bar_layout_with_style(&c, &cats, &d, &o, &s).unwrap();
        assert_eq!(
            new.bars
                .iter()
                .map(|b| b.series.as_str())
                .collect::<Vec<_>>(),
            ["z", "a"]
        );
        assert_eq!(new.bars[0].source_series, 0);
        assert_eq!(new.bars[1].source_series, 1);
        assert_eq!(new.bars[0].color, Color(255, 0, 0, 255));
        assert_eq!(new.bars[1].color, Color(0, 0, 255, 255));
        assert_eq!(
            new.scene
                .legend
                .iter()
                .map(|i| i.id.as_str())
                .collect::<Vec<_>>(),
            ["z", "a"]
        );
        assert!(new.scene.legend_roles.is_empty());
        match mode {
            BarLayout::Grouped => assert!(new.bars[0].band[0] < new.bars[1].band[0]),
            BarLayout::Stacked => {
                assert_eq!(new.bars[0].interval, [0., 2.]);
                assert_eq!(new.bars[1].interval, [2., 5.]);
            }
            BarLayout::Percent => {
                near(new.bars[0].interval[1], 0.4);
                near(new.bars[1].interval[0], 0.4);
                near(new.bars[1].interval[1], 1.);
            }
        }
        let mut hidden = o.clone();
        hidden.hidden.insert("z".into());
        let l = bar_layout_with_style(&c, &cats, &d, &hidden, &s).unwrap();
        assert_eq!(l.bars[0].series, "a");
        assert_eq!(l.bars[0].source_series, 1);
        assert_eq!(l.bars[0].color, Color(0, 0, 255, 255));
        assert_eq!(l.scene.legend, new.scene.legend);
    }
}

#[test]
fn index_source_keeps_null_zero_crop_and_window_before_ink_filters() {
    let s = style();
    let base = bar_layout_with_style(&frame(), &categories(), &source(), &options(), &s).unwrap();
    assert_eq!(base.scene.legend.len(), 4);
    assert_eq!(base.bars.len(), 4);
    assert!(base.bars.iter().any(|b| b.raw_value == 0.));
    assert!(!base.bars.iter().any(|b| b.category == "D"));
    for window in [None, Some([0.5, 1.])] {
        let mut o = options();
        o.category_window = window;
        let l = bar_layout_with_style(&frame(), &categories(), &source(), &o, &s).unwrap();
        assert_eq!(l.scene.legend, base.scene.legend);
        if window.is_some() {
            assert!(
                l.bars
                    .iter()
                    .all(|b| b.category == "C" || b.category == "D")
            );
        }
    }
    let mut crop = options();
    crop.value_domain = Some([10., 20.]);
    let l = bar_layout_with_style(&frame(), &categories(), &source(), &crop, &s).unwrap();
    assert!(l.bars.is_empty());
    assert_eq!(l.scene.legend, base.scene.legend);
    crop = options();
    crop.hidden.insert("z".into());
    let l = bar_layout_with_style(&frame(), &categories(), &source(), &crop, &s).unwrap();
    assert!(
        l.scene
            .legend
            .iter()
            .all(|i| i.color == Color(0, 0, 255, 255))
    );
    assert_eq!(l.scene.legend.len(), 4);
    crop.hidden.insert("a".into());
    let l = bar_layout_with_style(&frame(), &categories(), &source(), &crop, &s).unwrap();
    assert!(l.scene.legend.is_empty());
    assert!(l.scene.legend_roles.is_empty());
    let l = bar_layout_with_style(
        &frame(),
        &categories(),
        &[],
        &options(),
        &BarStyleOptions {
            legend_source: BarLegendSource::Indexes,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(l.scene.legend.is_empty());
}

#[test]
fn horizontal_index_order_reverses_once_and_category_palette_can_use_explicit_authored_domain() {
    let cats = vec!["B".into(), "A".into()];
    let d = vec![BarSeries::new("z", vec![None, Some(0.)])];
    let palette = OrdinalColorOptions {
        palette: OrdinalColorPalette::Custom(vec![Color(30, 40, 50, 255), Color(60, 70, 80, 255)]),
        domain: Some(
            cats.iter()
                .map(|id: &String| CategoryKey::from(id.as_str()))
                .collect(),
        ),
        ..Default::default()
    }
    .resolve([])
    .unwrap();
    for reverse in [false, true] {
        let mut s = BarStyleOptions {
            legend_source: BarLegendSource::Indexes,
            color_by: BarColorBy::Category,
            category_scale: Some(Default::default()),
            category_reverse: reverse,
            ..Default::default()
        };
        s = s.with_ordinal_colors(&cats, &d, &palette).unwrap();
        let o = BarOptions {
            horizontal: true,
            ..Default::default()
        };
        let l = bar_layout_with_style(&frame(), &cats, &d, &o, &s).unwrap();
        assert_eq!(
            l.scene
                .legend
                .iter()
                .map(|i| i.id.as_str())
                .collect::<Vec<_>>(),
            ["A", "B"]
        );
        assert_eq!(l.scene.legend[0].color, Color(60, 70, 80, 255));
        assert_eq!(l.scene.legend[1].color, Color(30, 40, 50, 255));
    }
}

#[test]
fn zero_category_band_and_disabled_legend_do_not_create_or_overcharge_index_items() {
    let mut s = style();
    s.category_scale = Some(scales::BandScaleOptions {
        padding_inner: 1.,
        ..Default::default()
    });
    let l = bar_layout_with_style(&frame(), &categories(), &source(), &options(), &s).unwrap();
    assert!(l.scene.legend.is_empty());
    assert!(l.bars.is_empty());
    let cats = (0..257).map(|i| format!("C{i}")).collect::<Vec<_>>();
    let d = vec![BarSeries::new("a", vec![None; 257])];
    let mut s = BarStyleOptions {
        legend_source: BarLegendSource::Indexes,
        ..Default::default()
    };
    let mut calls = 0;
    assert!(
        bar_layout_with_bar_factory(&frame(), &cats, &d, &Default::default(), &s, |c| {
            calls += 1;
            Ok(c.inherit(c.geometry.clone()))
        })
        .unwrap_err()
        .0
        .contains("256")
    );
    assert_eq!(calls, 0);
    s.legend = false;
    let l = bar_layout_with_style(&frame(), &cats, &d, &Default::default(), &s).unwrap();
    assert!(l.scene.legend.is_empty());
    s.legend = true;
    s.category_scale = Some(scales::BandScaleOptions {
        padding_inner: 1.,
        ..Default::default()
    });
    assert!(
        bar_layout_with_style(&frame(), &cats, &d, &Default::default(), &s)
            .unwrap()
            .scene
            .legend
            .is_empty()
    );
    let mut o = BarOptions::default();
    o.hidden.insert("a".into());
    s.category_scale = None;
    assert!(
        bar_layout_with_style(&frame(), &cats, &d, &o, &s)
            .unwrap()
            .scene
            .legend
            .is_empty()
    );
}

#[test]
fn order_is_complete_and_validated_before_factories_even_hidden_disabled_or_empty() {
    for order in [
        vec![],
        vec!["z".into()],
        vec!["z".into(), "z".into()],
        vec!["z".into(), "unknown".into()],
        vec!["z".into(), "a".into(), "extra".into()],
    ] {
        for hidden in [false, true] {
            let mut o = options();
            if hidden {
                o.hidden.extend(["z".into(), "a".into()]);
            }
            let s = BarStyleOptions {
                legend: false,
                series_order: Some(order.clone()),
                ..Default::default()
            };
            let before = source();
            let mut calls = 0;
            assert!(
                bar_layout_with_bar_factory(&frame(), &categories(), &before, &o, &s, |c| {
                    calls += 1;
                    Ok(c.inherit(c.geometry.clone()))
                })
                .is_err()
            );
            assert_eq!(calls, 0);
            assert_eq!(before, source());
        }
    }
    let empty = BarStyleOptions {
        series_order: Some(vec![]),
        ..Default::default()
    };
    assert!(bar_layout_with_style(&frame(), &[], &[], &Default::default(), &empty).is_ok());
    let unknown = BarStyleOptions {
        series_order: Some(vec!["x".into()]),
        ..Default::default()
    };
    assert!(bar_layout_with_style(&frame(), &[], &[], &Default::default(), &unknown).is_err());
}

#[test]
fn custom_paint_text_opacity_and_hover_keep_pre_factory_index_palette_and_private_frame()
-> Result<()> {
    let mut s = style();
    s.labels.enabled = true;
    s.labels.zero_extent = BarZeroExtentLabels::ValueAxis;
    s.labels.color_profile = Some(InheritedColor::Fixed(Color(40, 50, 60, 101)));
    let mut events = vec![];
    let mut labels = 0;
    let mut label = |c: &BarLabelContext<'_>| {
        labels += 1;
        Ok(format!("{}={}", c.category, c.default_text))
    };
    let mut bar = |c: &BarDatumContext<'_>| {
        events.push((c.series.id.clone(), c.source_series, c.source_category));
        let mut m = c.inherit(c.geometry.clone());
        m.fill = Some(Color(0, 255, 0, 255));
        m.opacity = 0.5;
        Ok(m)
    };
    let mut l = bar_layout_with_scale_options_and_factories(
        &frame(),
        &categories(),
        &source(),
        &options(),
        &s,
        &Default::default(),
        BarFactories {
            bar: Some(&mut bar),
            label: Some(&mut label),
        },
    )?;
    assert_eq!(events.len(), 4);
    assert_eq!(labels, 4);
    assert_eq!(
        events.iter().map(|v| v.0.as_str()).collect::<Vec<_>>(),
        ["z", "z", "a", "a"]
    );
    assert!(
        l.layout
            .layout
            .scene
            .legend
            .iter()
            .all(|i| i.color == Color(255, 0, 0, 255))
    );
    let binding = l.layout.interaction_binding()?;
    assert!(
        binding
            .sites
            .iter()
            .all(|site| site.color == Color(0, 255, 0, 255))
    );
    let original = l.layout.layout.scene.clone();
    let id = binding.sites[0].mark_id.clone();
    l.layout.set_active(Some(&id), 0.2)?;
    l.layout.set_active(None, 0.2)?;
    assert_eq!(l.layout.layout.scene, original);
    let mut bad = l.layout.clone();
    bad.layout.scene.legend_roles.clear();
    assert!(bad.interaction_binding().is_err());
    assert!(bad.set_active(None, 0.2).is_err());
    assert_eq!(
        original
            .labels
            .iter()
            .filter(|l| l.color == Color(40, 50, 60, 51))
            .count(),
        4
    );
    Ok(())
}

#[test]
fn category_equal_series_has_explicit_role_and_existing_explicit_series_filter_remains_series_only()
{
    let cats = vec!["a".into(), "z".into()];
    let d = vec![
        BarSeries::new("a", vec![Some(2.), None]),
        BarSeries::new("z", vec![Some(0.), Some(1.)]),
    ];
    let s = BarStyleOptions {
        legend_source: BarLegendSource::Indexes,
        ..Default::default()
    };
    let l = bar_layout_with_style(&frame(), &cats, &d, &Default::default(), &s).unwrap();
    assert_eq!(
        l.scene.legend_roles,
        BTreeMap::from([
            ("a".into(), LegendRole::Category),
            ("z".into(), LegendRole::Category)
        ])
    );
    let filtered = l.scene.without_series(&BTreeSet::from(["a".into()]));
    assert!(filtered.marks.iter().all(|m| m.series != "a"));
    assert_eq!(filtered.legend, l.scene.legend);
    assert_eq!(filtered.legend_roles, l.scene.legend_roles);
    filtered.validate().unwrap();
    // Actual category legend key/mouse behavior belongs to shared host tests;
    // this core test does not fake a host click or promise automatic category hide.
}
