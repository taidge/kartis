use kartis::*;
fn samples(group: &str, sub: &str, values: &[f64]) -> Vec<BoxObservation> {
    values
        .iter()
        .enumerate()
        .map(|(i, v)| BoxObservation::new(format!("{group}/{sub}/{i}"), group, sub, Some(*v)))
        .collect()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
#[test]
fn nivo_r7_default_quantiles_are_not_tukey_whiskers() {
    let data = samples("A", "one", &[0., 1., 2., 3., 4., 5., 6., 7., 8., 9.]);
    let l = boxplot_layout(&ChartConfig::default(), &data, &Default::default()).unwrap();
    let b = &l.boxes[0];
    for (a, e) in b.quantiles.into_iter().zip([0.9, 2.25, 4.5, 6.75, 8.1]) {
        close(a, e)
    }
    close(b.mean, 4.5);
    let first_box = l
        .scene
        .marks
        .iter()
        .position(|m| m.id.starts_with("box:"))
        .unwrap();
    assert!(
        l.scene
            .marks
            .iter()
            .enumerate()
            .filter(|(_, m)| m.id.contains("grid"))
            .all(|(i, _)| i < first_box)
    );
    assert_eq!(b.count, 10);
    assert_eq!(b.extrema, [0., 9.]);
    assert_eq!(b.whiskers, [0.9, 8.1]);
    assert_eq!(
        b.outliers.iter().map(|d| d.value).collect::<Vec<_>>(),
        vec![0., 9.]
    );
    let mut reversed = data.clone();
    reversed.reverse();
    assert_eq!(
        l,
        boxplot_layout(&ChartConfig::default(), &reversed, &Default::default()).unwrap()
    );
}
#[test]
fn nullable_empty_cells_explicit_order_and_hidden_subgroup_reflow() {
    let mut d = samples("B", "z", &[1., 2., 3.]);
    d.extend(samples("B", "a", &[-3., 0., 4.]));
    d.push(BoxObservation::new("none", "A", "a", None));
    let mut o = BoxPlotLayoutOptions {
        groups: Some(vec!["B".into(), "A".into(), "reserved".into()]),
        subgroups: Some(vec!["z".into(), "a".into()]),
        ..Default::default()
    };
    let before = boxplot_layout(&ChartConfig::default(), &d, &o).unwrap();
    assert_eq!(before.boxes.len(), 2);
    assert_eq!(before.groups, vec!["B", "A", "reserved"]);
    assert!(before.boxes[0].center < before.boxes[1].center);
    o.hidden.insert(boxplot_series_id("a"));
    let after = boxplot_layout(&ChartConfig::default(), &d, &o).unwrap();
    assert_eq!(after.boxes.len(), 1);
    close(
        after.boxes[0].breadth,
        2. * before.boxes[0].breadth + o.inner_padding,
    );
    assert_eq!(after.boxes[0].id, before.boxes[0].id);
    assert!(
        after
            .scene
            .marks
            .iter()
            .filter(|m| m.id.starts_with("box:"))
            .all(|m| m.series == boxplot_series_id("z"))
    );
    o.hidden.insert(boxplot_series_id("z"));
    o.scale = ScaleKind::Log;
    assert!(boxplot_layout(&ChartConfig::default(), &d, &o).is_err()); // Mixed/zero log source is invalid, including empty-domain inference.
}
#[test]
fn all_hidden_log_domain_keeps_tick_or_source_sign_without_accepting_mixed_or_zero() {
    let c = ChartConfig::default();
    let negative = samples("A", "s", &[-8., -4., -1.]);
    let mut o = BoxPlotLayoutOptions {
        scale: ScaleKind::Log,
        value_axis: AxisOptions {
            tick_values: Some(vec![-8., -4., -1.]),
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(boxplot_layout(&c, &negative, &o).is_ok());
    o.hidden.insert(boxplot_series_id("s"));
    let hidden = boxplot_layout(&c, &negative, &o).unwrap();
    assert_eq!(hidden.domain, [-10., -1.]);
    assert!(hidden.boxes.is_empty());
    o.value_axis.tick_values = None;
    assert_eq!(
        boxplot_layout(&c, &negative, &o).unwrap().domain,
        [-10., -1.]
    );
    let positive = samples("A", "s", &[1., 4., 8.]);
    assert_eq!(boxplot_layout(&c, &positive, &o).unwrap().domain, [1., 10.]);
    for ticks in [vec![-1., 1.], vec![-1., 0.]] {
        o.value_axis.tick_values = Some(ticks);
        assert!(boxplot_layout(&c, &negative, &o).is_err());
    }
    o.value_axis.tick_values = None;
    for values in [vec![-1., 1.], vec![-1., 0.]] {
        assert!(boxplot_layout(&c, &samples("A", "s", &values), &o).is_err());
    }
    assert_eq!(boxplot_layout(&c, &[], &o).unwrap().domain, [1., 10.]);
}
#[test]
fn tukey_source_outliers_and_transposed_geometry_use_value_and_group_axes() {
    let d = samples("A", "", &[0., 1., 2., 3., 4., 5., 100.]);
    let c = ChartConfig::default();
    let mut o = BoxPlotLayoutOptions {
        whiskers: BoxWhiskers::Tukey { factor: 1.5 },
        domain: Some([0., 100.]),
        ..Default::default()
    };
    let v = boxplot_layout(&c, &d, &o).unwrap();
    assert_eq!(v.boxes[0].whiskers, [0., 5.]);
    assert_eq!(v.boxes[0].outliers[0].source_id, "A//6");
    assert_eq!(v.scene.legend[0].id, boxplot_series_id(""));
    o.orientation = StatisticsOrientation::Horizontal;
    let h = boxplot_layout(&c, &d, &o).unwrap();
    let r = h.boxes[0].rect;
    close(r.x, c.margin.left + c.inner_width() * 0.015);
    close(r.width, c.inner_width() * 0.03);
    close(r.height, h.boxes[0].breadth);
    let source = h
        .scene
        .marks
        .iter()
        .find(|m| m.id.contains(":outlier:"))
        .unwrap();
    assert_eq!(source.value, 100.);
    assert_eq!(source.series, boxplot_series_id(""));
    let body = h
        .scene
        .marks
        .iter()
        .find(|m| m.id == h.boxes[0].id)
        .unwrap();
    assert_eq!(body.label, "A");
    assert_eq!(body.value, h.boxes[0].quantiles[2]);
    let median = h
        .scene
        .marks
        .iter()
        .find(|m| m.id == format!("{}:median", h.boxes[0].id))
        .unwrap();
    assert_eq!(median.label, "A · median");
    assert_eq!(median.value, body.value);
    assert!(
        h.scene
            .marks
            .iter()
            .filter(|m| m.id.ends_with(":whiskers") || m.id.ends_with(":caps"))
            .all(|m| m.label.is_empty() && m.series == boxplot_series_id(""))
    );
    assert!(h.boxes[0].mean > body.value); // Full aggregate stays in metadata.
}
#[test]
fn independent_fill_border_rounding_and_outlier_shape_are_real_geometry() {
    let d = samples("A", "boxplot-series:1:x", &[0., 1., 2., 3., 4., 5., 100.]);
    let o = BoxPlotLayoutOptions {
        fill_opacity: 0.2,
        border_radius: 1000.,
        border: Some(glory::render::GraphicStroke {
            color: Color(0, 0, 0, 255),
            width: 2.,
            cap: glory::render::StrokeCap::Butt,
        }),
        outlier_shape: BoxOutlierShape::Diamond,
        ..Default::default()
    };
    let l = boxplot_layout(&ChartConfig::default(), &d, &o).unwrap();
    let b = &l.boxes[0];
    let m = l.scene.marks.iter().find(|m| m.id == b.id).unwrap();
    assert_eq!(m.opacity, 0.2);
    assert!(
        matches!(&m.geometry,Geometry::Path(p) if p.iter().filter(|p|matches!(p,glory::render::PathCommand::Cubic{..})).count()==4)
    );
    let border = l
        .scene
        .marks
        .iter()
        .find(|m| m.id == format!("{}:border", b.id))
        .unwrap();
    assert_eq!(border.opacity, 1.);
    assert!(border.fill.is_none());
    assert_eq!(border.series, boxplot_series_id(&b.subgroup));
    assert!(
        matches!(&l.scene.marks.iter().find(|m|m.id.contains(":outlier:")).unwrap().geometry,Geometry::Path(p) if p.len()==5)
    );
}
#[test]
fn extremes_invalid_data_and_budgets_are_checked_without_mutation() {
    let c = ChartConfig::default();
    let d = samples("A", "one", &[f64::MAX; 20]);
    let l = boxplot_layout(&c, &d, &Default::default()).unwrap();
    assert_eq!(l.boxes[0].mean, f64::MAX);
    assert_eq!(l.boxes[0].quantiles, [f64::MAX; 5]);
    let wide = samples("A", "one", &[-f64::MAX, 0., f64::MAX]);
    let o = BoxPlotLayoutOptions {
        scale: ScaleKind::SymLog { constant: 1. },
        whiskers: BoxWhiskers::Tukey { factor: 0. },
        ..Default::default()
    };
    assert!(boxplot_layout(&c, &wide, &o).is_ok());
    let saved = l.scene.clone();
    let mut bad = d.clone();
    bad[0].value = Some(f64::NAN);
    assert!(boxplot_layout(&c, &bad, &Default::default()).is_err());
    bad = d.clone();
    bad[0].id = bad[1].id.clone();
    assert!(boxplot_layout(&c, &bad, &Default::default()).is_err());
    assert!(
        boxplot_layout(
            &c,
            &samples("a", "s", &vec![0.; 10001]),
            &Default::default()
        )
        .is_err()
    );
    assert!(
        boxplot_layout(
            &c,
            &d,
            &BoxPlotLayoutOptions {
                quantiles: [0., 0.5, 0.4, 0.7, 1.],
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        boxplot_layout(
            &c,
            &d,
            &BoxPlotLayoutOptions {
                groups: Some(vec!["missing".into()]),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(l.scene, saved);
    let small = samples("A", "s", &[0., 1.]);
    let two = boxplot_layout(
        &c,
        &small,
        &BoxPlotLayoutOptions {
            whiskers: BoxWhiskers::Tukey { factor: 0. },
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(two.boxes[0].whiskers, [0.25, 0.75]);
    let mut aliases = samples("A", "x", &[1., 2.]);
    aliases.extend(samples("A", &boxplot_series_id("x"), &[3., 4.]));
    let remaining = boxplot_layout(
        &c,
        &aliases,
        &BoxPlotLayoutOptions {
            hidden: std::collections::BTreeSet::from([boxplot_series_id("x")]),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(remaining.boxes.len(), 1);
    assert_eq!(remaining.boxes[0].subgroup, boxplot_series_id("x"));
}
#[cfg(feature = "raster")]
#[test]
fn cpu_fill_alpha_does_not_fade_box_border_or_median() {
    use kartis::render::raster::*;
    let c = ChartConfig {
        width: 200.,
        height: 180.,
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 20.,
            bottom: 20.,
        },
        ..Default::default()
    };
    let d = samples("A", "s", &[0., 1., 2., 3., 4.]);
    let o = BoxPlotLayoutOptions {
        domain: Some([0., 4.]),
        fill_opacity: 0.25,
        colors: std::collections::BTreeMap::from([("s".into(), Color(255, 0, 0, 255))]),
        border: Some(glory::render::GraphicStroke {
            color: Color(0, 0, 0, 255),
            width: 2.,
            cap: glory::render::StrokeCap::Butt,
        }),
        value_axis: AxisOptions {
            visible: false,
            grid: false,
            ..Default::default()
        },
        group_axis: CategoryAxisOptions {
            visible: false,
            ..Default::default()
        },
        ..Default::default()
    };
    let l = boxplot_layout(&c, &d, &o).unwrap();
    let mut s = l.scene;
    s.labels.clear();
    s.legend.clear();
    let image = render_raster(&s, &FontRegistry::default(), &RasterOptions::default()).unwrap();
    let pixel = |x: usize, y: usize| {
        let i = (y * image.width as usize + x) * 4;
        &image.rgba[i..i + 4]
    };
    assert_eq!(pixel(100, 80), &[255, 191, 191, 255]);
    let r = l.boxes[0].rect;
    assert!(pixel(r.x.floor() as usize, 80)[0] < 80);
    assert!(pixel(100, 90)[0] < 100);
}
