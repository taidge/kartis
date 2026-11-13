use kartis::stack::{StackOffset, StackOptions, StackOrder};
use kartis::*;
fn frame() -> ChartConfig {
    ChartConfig {
        width: 420.,
        height: 260.,
        margin: Margin {
            left: 10.,
            right: 10.,
            top: 10.,
            bottom: 10.,
        },
        ..Default::default()
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}
fn data(case: usize) -> (Vec<MarimekkoCategory>, Vec<BarSeries>) {
    let (weights, rows): (&[f64], Vec<(&str, Vec<f64>)>) = match case {
        0 => (&[1., 3.], vec![("p", vec![2., 1.]), ("q", vec![6., 3.])]),
        1 => (
            &[2., 0., 5.],
            vec![("p", vec![1., 20., 4.]), ("q", vec![3., 10., 2.])],
        ),
        2 => (
            &[3., 4.],
            vec![
                ("p", vec![-2., 4.]),
                ("q", vec![3., -1.]),
                ("r", vec![-4., 2.]),
            ],
        ),
        _ => (
            &[0.01, 0.71],
            vec![("p", vec![0.01, 0.71]), ("q", vec![0.11, 0.31])],
        ),
    };
    (
        weights
            .iter()
            .enumerate()
            .map(|(i, w)| MarimekkoCategory::new(format!("c{i}"), *w))
            .collect(),
        rows.into_iter()
            .map(|(id, values)| BarSeries::new(id, values.into_iter().map(Some).collect()))
            .collect(),
    )
}
fn offset(s: &str) -> StackOffset {
    match s {
        "expand" => StackOffset::Expand,
        "diverging" => StackOffset::Diverging,
        "silouhette" => StackOffset::Silhouette,
        "wiggle" => StackOffset::Wiggle,
        _ => StackOffset::Zero,
    }
}
#[test]
fn actual_nivo_hooks_oracle_matches_328_cells_with_independent_scales_offsets_padding_and_nice() {
    let mut count = 0;
    for row in include_str!("fixtures/marimekko-nivo.csv").lines().skip(2) {
        let f = row.split('|').collect::<Vec<_>>();
        let n = |i: usize| f[i].parse::<f64>().unwrap();
        let (categories, dimensions) = data(f[0].parse().unwrap());
        let o = MarimekkoLayoutOptions {
            horizontal: f[2] == "1",
            outer_padding: n(3),
            inner_padding: n(4),
            stack: StackOptions {
                offset: offset(f[1]),
                ..Default::default()
            },
            ..Default::default()
        };
        let l = marimekko_layout(&frame(), &categories, &dimensions, &o).unwrap();
        let cell = l
            .cells
            .iter()
            .find(|d| d.source_category == f[5].parse::<usize>().unwrap() && d.dimension == f[6])
            .unwrap();
        for (a, b) in [cell.rect.x, cell.rect.y, cell.rect.width, cell.rect.height]
            .into_iter()
            .zip([n(7) + 10., n(8) + 10., n(9), n(10)])
        {
            close(a, b)
        }
        close(cell.interval[0], n(11));
        close(cell.interval[1], n(12));
        close(l.thickness_domain[1], n(14));
        close(l.dimensions_domain[0], n(15));
        close(l.dimensions_domain[1], n(16));
        close(cell.displayed_value.unwrap(), n(13));
        count += 1;
    }
    assert_eq!(count, 328);
}
#[test]
fn thickness_is_independent_of_dimensions_and_none_is_not_expand() {
    let (cats, d) = data(0);
    let c = frame();
    let mut o = MarimekkoLayoutOptions {
        thickness_nice: None,
        dimensions_nice: None,
        inner_padding: 0.,
        ..Default::default()
    };
    let l = marimekko_layout(&c, &cats, &d, &o).unwrap();
    let a = l.categories[0].band.unwrap();
    let b = l.categories[1].band.unwrap();
    close(a[1], 100.);
    close(b[1], 300.);
    close(l.cells[1].rect.height / l.cells[0].rect.height, 0.5);
    let doubled = d
        .iter()
        .map(|s| BarSeries::new(&s.id, s.values.iter().map(|v| v.map(|v| v * 7.)).collect()))
        .collect::<Vec<_>>();
    let other = marimekko_layout(&c, &cats, &doubled, &o).unwrap();
    assert_eq!(
        l.categories.iter().map(|d| d.band).collect::<Vec<_>>(),
        other.categories.iter().map(|d| d.band).collect::<Vec<_>>()
    );
    o.stack.offset = StackOffset::Expand;
    let expanded = marimekko_layout(&c, &cats, &d, &o).unwrap();
    close(expanded.cells[0].displayed_value.unwrap(), 0.25);
    assert_eq!(expanded.cells[0].raw_value, Some(2.));
    assert_eq!(expanded.dimensions_domain, [0., 1.]);
    let changed = [
        MarimekkoCategory::new("c0", 3.),
        MarimekkoCategory::new("c1", 1.),
    ];
    let swapped = marimekko_layout(&c, &changed, &d, &o).unwrap();
    assert_eq!(expanded.stack, swapped.stack);
    close(swapped.categories[0].band.unwrap()[1], 300.);
    let (small, values) = data(3);
    let nice = marimekko_layout(&c, &small, &values, &Default::default()).unwrap();
    close(nice.thickness_domain[1], 0.8);
    close(
        nice.categories
            .iter()
            .map(|d| d.band.unwrap()[1])
            .sum::<f64>(),
        nice.available_thickness * 0.72 / 0.8,
    );
}
#[test]
fn signed_intervals_keep_raw_sign_and_wiggle_uses_column_order_not_physical_width() {
    let (cats, d) = data(2);
    let mut o = MarimekkoLayoutOptions {
        stack: StackOptions {
            offset: StackOffset::Diverging,
            ..Default::default()
        },
        thickness_nice: None,
        dimensions_nice: None,
        ..Default::default()
    };
    let l = marimekko_layout(&frame(), &cats, &d, &o).unwrap();
    assert_eq!(l.stack.domain, [-6., 6.]);
    for (id, pair, raw) in [
        ("p", [-2., 0.], -2.),
        ("q", [0., 3.], 3.),
        ("r", [-6., -2.], -4.),
    ] {
        let cell = l
            .cells
            .iter()
            .find(|x| x.source_category == 0 && x.dimension == id)
            .unwrap();
        assert_eq!(cell.interval, pair);
        assert_eq!(cell.raw_value, Some(raw));
        assert_eq!(cell.displayed_value, Some(raw.abs()));
        let mark = &l.scene.marks[cell.mark_index.unwrap()];
        assert_eq!(mark.value, raw);
    }
    for off in [
        StackOffset::Expand,
        StackOffset::Wiggle,
        StackOffset::Silhouette,
    ] {
        o.stack.offset = off;
        assert!(marimekko_layout(&frame(), &cats, &d, &o).is_err());
    }
    let (cats, d) = data(0);
    o.stack.offset = StackOffset::Wiggle;
    let first = marimekko_layout(&frame(), &cats, &d, &o).unwrap();
    let weights = [
        MarimekkoCategory::new("c0", 100.),
        MarimekkoCategory::new("c1", 1.),
    ];
    let second = marimekko_layout(&frame(), &weights, &d, &o).unwrap();
    assert_eq!(first.stack, second.stack);
    assert_eq!(first.stack.baseline, vec![0., 2.]);
}
#[test]
fn zero_thickness_retains_padding_and_stack_domain_but_never_emits_stroke_or_controls() {
    let (cats, d) = data(1);
    let o = MarimekkoLayoutOptions {
        outer_padding: 5.,
        inner_padding: 3.,
        thickness_nice: None,
        dimensions_nice: None,
        style: MarimekkoDimensionStyle {
            border: Some(glory::render::GraphicStroke {
                color: Color(0, 0, 0, 255),
                width: 5.,
                cap: glory::render::StrokeCap::Square,
            }),
            ..Default::default()
        },
        labels: BarLabels {
            enabled: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let l = marimekko_layout(&frame(), &cats, &d, &o).unwrap();
    assert_eq!(l.dimensions_domain, [0., 30.]);
    close(l.available_thickness, 384.);
    let zero = l.categories[1].band.unwrap();
    assert_eq!(zero[1], 0.);
    close(
        zero[0],
        l.categories[0].band.unwrap().into_iter().sum::<f64>() + 3.,
    );
    close(l.categories[2].band.unwrap()[0], zero[0] + 3.);
    assert!(
        l.cells
            .iter()
            .filter(|d| d.source_category == 1)
            .all(|d| d.mark_index.is_none() && d.label_index.is_none() && d.raw_value.is_some())
    );
    let zeros = cats
        .iter()
        .map(|d| MarimekkoCategory::new(&d.id, 0.))
        .collect::<Vec<_>>();
    let all = marimekko_layout(&frame(), &zeros, &d, &o).unwrap();
    assert!(all.cells.iter().all(|d| d.mark_index.is_none()));
    assert!(all.scene.marks.iter().all(|d| d.series.is_empty()));
    let nullable = [BarSeries::new("missing", vec![None, Some(0.), None])];
    let nulls = marimekko_layout(&frame(), &cats, &nullable, &o).unwrap();
    assert_eq!(nulls.cells[0].raw_value, None);
    assert_eq!(nulls.cells[1].raw_value, Some(0.));
    assert!(nulls.cells.iter().all(|d| d.mark_index.is_none()));
}
#[test]
fn hidden_dimensions_preserve_thickness_hidden_categories_relayout_and_source_ids_survive() {
    let (cats, d) = data(0);
    let mut o = MarimekkoLayoutOptions {
        thickness_nice: None,
        dimensions_nice: None,
        labels: BarLabels {
            enabled: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let before = marimekko_layout(&frame(), &cats, &d, &o).unwrap();
    o.hidden_dimensions.insert("q".into());
    let hidden = marimekko_layout(&frame(), &cats, &d, &o).unwrap();
    assert_eq!(
        hidden.categories.iter().map(|d| d.band).collect::<Vec<_>>(),
        before.categories.iter().map(|d| d.band).collect::<Vec<_>>()
    );
    assert_eq!(hidden.dimension_source_indices, vec![0]);
    assert_eq!(hidden.cells[0].id, before.cells[0].id);
    assert!(hidden.scene.label_series.values().all(|s| s == "p"));
    o.hidden_categories.insert("c0".into());
    let one = marimekko_layout(&frame(), &cats, &d, &o).unwrap();
    assert_eq!(one.category_source_indices, vec![1]);
    close(one.categories[1].band.unwrap()[1], 400.);
    assert_eq!(one.cells[0].source_category, 1);
    assert_eq!(one.cells[0].id, before.cells[1].id);
    o.hidden_dimensions.insert("p".into());
    let all = marimekko_layout(&frame(), &cats, &d, &o).unwrap();
    assert!(all.cells.is_empty());
    assert_eq!(all.stack.baseline, vec![0.]);
    assert!(all.categories[1].band.is_some());
    o.hidden_dimensions.clear();
    o.hidden_categories.clear();
    o.stack.order = StackOrder::Reverse;
    let reversed = marimekko_layout(&frame(), &cats, &d, &o).unwrap();
    assert_eq!(
        before.cells.iter().map(|d| &d.id).collect::<Vec<_>>(),
        reversed.cells.iter().map(|d| &d.id).collect::<Vec<_>>()
    );
    assert_ne!(before.cells[0].interval, reversed.cells[0].interval);
}
#[test]
fn styling_labels_four_axes_and_cropped_original_shapes_are_authored_scene_data() {
    let (cats, d) = data(0);
    let c = ChartConfig {
        margin: Margin {
            left: 55.,
            right: 55.,
            top: 40.,
            bottom: 45.,
        },
        ..frame()
    };
    let mut o = MarimekkoLayoutOptions {
        dimensions_domain: Some([1., 3.]),
        dimensions_nice: None,
        style: MarimekkoDimensionStyle {
            fill_opacity: 0.25,
            border: Some(glory::render::GraphicStroke {
                color: Color(0, 0, 0, 255),
                width: 2.,
                cap: glory::render::StrokeCap::Butt,
            }),
            ..Default::default()
        },
        axes: MarimekkoAxes {
            top: Some(AxisOptions::default()),
            right: Some(AxisOptions::default()),
            bottom: None,
            left: Some(AxisOptions::default()),
        },
        category_axis: Some(MarimekkoCategoryAxis {
            side: AxisSide::Bottom,
            options: CategoryAxisOptions::default(),
        }),
        labels: BarLabels {
            enabled: true,
            value: BarLabelValue::Displayed,
            format: ValueFormat::Fixed(1),
            skip_width: 5.,
            skip_height: 5.,
            ..Default::default()
        },
        ..Default::default()
    };
    o.styles.insert(
        "p".into(),
        MarimekkoDimensionStyle {
            paint: Some(FillPaint::Linear {
                start: [55., 40.],
                end: [365., 215.],
                stops: vec![
                    GradientStop::new(0., Color(255, 0, 0, 255)),
                    GradientStop::new(1., Color(0, 0, 255, 255)),
                ],
            }),
            ..o.style.clone()
        },
    );
    let l = marimekko_layout(&c, &cats, &d, &o).unwrap();
    let cell = &l.cells[0];
    assert!(cell.rect.y + cell.rect.height > c.height - c.margin.bottom);
    assert_eq!(
        l.scene.marks[cell.mark_index.unwrap()].geometry,
        Geometry::Rect {
            x: cell.rect.x,
            y: cell.rect.y,
            width: cell.rect.width,
            height: cell.rect.height
        }
    );
    let body = &l.scene.marks[cell.mark_index.unwrap()];
    assert_eq!(body.opacity, 0.25);
    assert!(body.fill_paint.is_some());
    assert!(render::to_svg(&l.scene).unwrap().contains("linearGradient"));
    let border = l
        .scene
        .marks
        .iter()
        .find(|m| m.id == format!("{}:border", body.id))
        .unwrap();
    assert_eq!(border.opacity, 1.);
    assert!(border.label.is_empty());
    assert!(
        l.scene
            .marks
            .iter()
            .any(|m| m.id.starts_with("mekko-axis-top"))
    );
    assert!(
        l.scene
            .marks
            .iter()
            .any(|m| m.id.starts_with("mekko-axis-right"))
    );
    assert!(l.scene.labels.iter().any(|l| l.text == "c0"));
    assert!(!l.scene.label_clips.is_empty());
}
#[test]
fn invalid_input_budget_and_precision_failures_leave_sources_and_accepted_scene_unchanged() {
    let (cats, d) = data(0);
    let c = frame();
    let accepted = marimekko_layout(&c, &cats, &d, &Default::default()).unwrap();
    let saved = accepted.clone();
    for kind in 0..7 {
        let mut o = MarimekkoLayoutOptions::default();
        let mut cs = cats.clone();
        let mut ds = d.clone();
        match kind {
            0 => cs[0].thickness = -1.,
            1 => cs[0].id = cs[1].id.clone(),
            2 => ds[0].values[0] = Some(f64::NAN),
            3 => o.outer_padding = 300.,
            4 => o.thickness_nice = Some(0),
            5 => {
                o.styles.insert("missing".into(), Default::default());
            }
            _ => ds[0].values.pop().map(|_| ()).unwrap(),
        };
        assert!(marimekko_layout(&c, &cs, &ds, &o).is_err());
    }
    assert_eq!(accepted, saved);
    assert_eq!(d, data(0).1);
    let large = (0..20001)
        .map(|i| MarimekkoCategory::new(i.to_string(), 1.))
        .collect::<Vec<_>>();
    assert!(marimekko_layout(&c, &large, &[], &Default::default()).is_err());
    let extreme = [
        MarimekkoCategory::new("A", f64::MAX),
        MarimekkoCategory::new("B", f64::MAX),
    ];
    assert!(marimekko_layout(&c, &extreme, &d, &Default::default()).is_err());
    let tiny = [
        MarimekkoCategory::new("A", f64::MAX),
        MarimekkoCategory::new("B", f64::from_bits(1)),
    ];
    assert!(marimekko_layout(&c, &tiny, &d, &Default::default()).is_err());
    let cancel = [
        BarSeries::new("big", vec![Some(1e300)]),
        BarSeries::new("small", vec![Some(1.)]),
    ];
    assert!(
        marimekko_layout(
            &c,
            &[MarimekkoCategory::new("A", 1.)],
            &cancel,
            &Default::default()
        )
        .is_err()
    );
}
#[cfg(feature = "raster")]
#[test]
fn cpu_pixels_prove_padding_zero_thickness_no_phantom_stroke_and_independent_bands() {
    use kartis::render::raster::*;
    let c = ChartConfig {
        width: 240.,
        height: 140.,
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 20.,
            bottom: 20.,
        },
        ..Default::default()
    };
    let cats = [
        MarimekkoCategory::new("A", 1.),
        MarimekkoCategory::new("zero", 0.),
        MarimekkoCategory::new("B", 3.),
    ];
    let d = [BarSeries::new("red", vec![Some(1.), Some(100.), Some(1.)])];
    let o = MarimekkoLayoutOptions {
        thickness_nice: None,
        dimensions_nice: None,
        outer_padding: 10.,
        inner_padding: 10.,
        dimensions_domain: Some([0., 1.]),
        grid_y: MarimekkoGrid::default(),
        style: MarimekkoDimensionStyle {
            color: Some(Color(255, 0, 0, 255)),
            border: Some(glory::render::GraphicStroke {
                color: Color(0, 0, 0, 255),
                width: 2.,
                cap: glory::render::StrokeCap::Butt,
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    let mut s = marimekko_with_options(&c, &cats, &d, &o).unwrap();
    s.legend.clear();
    let im = render_raster(&s, &FontRegistry::default(), &Default::default()).unwrap();
    let pixel = |x: usize, y: usize| {
        let i = (y * im.width as usize + x) * 4;
        &im.rgba[i..i + 4]
    };
    assert_eq!(pixel(40, 60), &[255, 0, 0, 255]);
    assert_eq!(pixel(100, 60), &[255, 0, 0, 255]);
    assert_eq!(pixel(80, 60), &[255, 255, 255, 255]);
    assert_eq!(pixel(25, 60), &[255, 255, 255, 255]);
}
