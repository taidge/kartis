//! Configurable rank bands, civil calendars, statistical samples and hierarchy focus.
use std::collections::BTreeSet;

use glory::render::{GraphicStroke, StrokeCap};
use kartis::*;
fn stroke(color: Color, width: f64) -> GraphicStroke {
    GraphicStroke {
        color,
        width,
        cap: StrokeCap::Butt,
    }
}
fn note(scene: &mut Scene, c: &ChartConfig, text: &str) {
    if !scene.draw_order.is_empty() {
        scene.draw_order.push(DrawItem::Label(scene.labels.len()));
    }
    scene.labels.push(Label {
        x: c.margin.left,
        y: 22.,
        text: text.into(),
        color: Color(80, 105, 125, 255),
        size: 12.,
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
}
fn key(scene: &mut Scene, c: &ChartConfig, items: &[(&str, Color)]) {
    for (i, (text, color)) in items.iter().enumerate() {
        if !scene.draw_order.is_empty() {
            scene.draw_order.push(DrawItem::Label(scene.labels.len()));
        }
        scene.labels.push(Label {
            x: c.margin.left + i as f64 * 125.,
            y: 43.,
            text: (*text).into(),
            color: *color,
            size: 12.,
            anchor: TextAnchor::Start,
            rotation: 0.,
        });
    }
}
fn rank_data(sparse: bool) -> Vec<AreaBumpSeries> {
    [
        (&"Search", [24., 12., 32., 16., 28.]),
        (&"Direct", [12., 26., 10., 30., 14.]),
        (&"Partners", [8., 14., 20., 12., 22.]),
    ]
    .into_iter()
    .map(|(id, values)| {
        AreaBumpSeries::new(
            *id,
            values
                .into_iter()
                .enumerate()
                .filter(|(i, _)| !(sparse && *id == "Search" && *i == 2))
                .map(|(i, v)| AreaBumpDatum::new(format!("Q{}", i + 1), v))
                .collect(),
        )
    })
    .collect()
}
fn calendar_colors() -> ColorScaleOptions {
    ColorScaleOptions {
        domain: Some([0., 15.]),
        colors: vec![
            Color(224, 237, 245, 255),
            Color(40, 135, 150, 255),
            Color(75, 55, 140, 255),
        ],
        unknown: Color(235, 238, 242, 255),
        ..Default::default()
    }
}
fn calendar_options(start: CalendarCivilDate, end: CalendarCivilDate) -> CalendarLayoutOptions {
    CalendarLayoutOptions {
        start: Some(start),
        end: Some(end),
        first_weekday: CalendarWeekday::Monday,
        weekday_ticks: vec![0, 2, 4, 6],
        colors: calendar_colors(),
        day_spacing: 3.,
        day_radius: 4.,
        month_border: Some(stroke(Color(65, 85, 110, 255), 1.3)),
        month_outline_padding: 0.3,
        day_labels: CalendarDayLabel::DayOfMonth,
        day_label_style: CalendarTextStyle {
            size: Some(11.),
            ..Default::default()
        },
        value_format: ValueFormat::Fixed(0),
        empty_label: "missing".into(),
        ..Default::default()
    }
}
fn box_samples() -> Vec<BoxObservation> {
    let mut result = Vec::new();
    for (g, shift) in [("Week 1", 0.), ("Week 2", 3.), ("Week 3", -2.)] {
        for (s, offset) in [("Alpha", 0.), ("Beta", 4.)] {
            for (i, value) in [-8., 0., 4., 8., 12., 40.].into_iter().enumerate() {
                result.push(BoxObservation::new(
                    format!("{g}/{s}/{i}"),
                    g,
                    s,
                    Some(value + shift + offset),
                ));
            }
        }
    }
    result.push(BoxObservation::new("missing", "Week 1", "Alpha", None));
    result
}
fn bullets() -> Vec<BulletDatum> {
    vec![
        BulletDatum {
            id: "North".into(),
            ranges: vec![-10., 10., 35.],
            measures: vec![8., 22.],
            markers: vec![27.],
        },
        BulletDatum {
            id: "South".into(),
            ranges: vec![0., 25., 50.],
            measures: vec![18., 38.],
            markers: vec![43.],
        },
        BulletDatum {
            id: "Online".into(),
            ranges: vec![5., 20., 45.],
            measures: vec![14., 32.],
            markers: vec![36.],
        },
    ]
}
fn hierarchy() -> Vec<HierarchyDatum> {
    let mut result = vec![HierarchyDatum::new("Portfolio", None, 0.)];
    for g in ["Cloud", "Devices", "Services"] {
        result.push(HierarchyDatum::new(g, Some("Portfolio".into()), 0.));
    }
    for (id, parent, value) in [
        ("Compute", "Cloud", 18.),
        ("Storage", "Cloud", 12.),
        ("API", "Cloud", 10.),
        ("Phone", "Devices", 14.),
        ("Tablet", "Devices", 8.),
        ("Support", "Services", 9.),
        ("Training", "Services", 6.),
    ] {
        result.push(HierarchyDatum::new(id, Some(parent.into()), value));
    }
    result
}
fn hierarchy_style() -> HierarchyStyleOptions {
    HierarchyStyleOptions {
        parent: HierarchyNodeStyle {
            fill: HierarchyFill::Fixed(Color(225, 233, 242, 255)),
            border: Some(stroke(Color(255, 255, 255, 255), 1.)),
            ..Default::default()
        },
        leaf: HierarchyNodeStyle {
            border: Some(stroke(Color(255, 255, 255, 255), 1.)),
            ..Default::default()
        },
        leaf_labels: HierarchyLabelStyle {
            size: Some(13.),
            skip_width: 30.,
            skip_height: 20.,
            skip_radius: 18.,
            ..Default::default()
        },
        parent_labels: HierarchyLabelStyle {
            enabled: true,
            size: Some(13.),
            ..Default::default()
        },
        ..Default::default()
    }
}
pub fn scenes(frame: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let mut output = Vec::new();
    let c = ChartConfig {
        margin: Margin {
            left: 85.,
            right: 95.,
            top: 78.,
            bottom: 50.,
        },
        ..frame.clone()
    };
    let base = AreaBumpOptions {
        x_order: Some((1..=5).map(|q| AreaBumpX::from(format!("Q{q}"))).collect()),
        spacing: 8.,
        x_padding: 0.55,
        axis_top: None,
        start_labels: AreaBumpLabels {
            padding: 10.,
            size: Some(12.),
            ..Default::default()
        },
        end_labels: AreaBumpLabels {
            padding: 10.,
            size: Some(12.),
            ..Default::default()
        },
        ..Default::default()
    };
    let mut smooth = area_bump(&c, &rank_data(false), &base)?;
    note(
        &mut smooth,
        &c,
        "Ranks change at each quarter · one common value-to-thickness scale",
    );
    key(
        &mut smooth,
        &c,
        &[
            ("Search", c.theme.color(2)),
            ("Direct", c.theme.color(0)),
            ("Partners", c.theme.color(1)),
        ],
    );
    output.push(("AreaBump variable-width smooth bands", smooth));
    let mut sparse = area_bump(
        &c,
        &rank_data(true),
        &AreaBumpOptions {
            missing: AreaBumpMissing::Gap,
            interpolation: AreaBumpInterpolation::Linear,
            ..base.clone()
        },
    )?;
    note(
        &mut sparse,
        &c,
        "Search has no Q3 sample · explicit gap preserves the missing interval",
    );
    output.push(("AreaBump sparse linear gaps", sparse));
    let mut focused = area_bump(
        &c,
        &rank_data(false),
        &AreaBumpOptions {
            active_ids: BTreeSet::from(["Direct".into()]),
            active_style: AreaBumpFocusStyle {
                fill_opacity: 1.,
                border_width: 2.,
                border_opacity: 1.,
            },
            ..base
        },
    )?;
    note(
        &mut focused,
        &c,
        "Controlled focus: Direct stays opaque · ranks and thickness stay fixed",
    );
    output.push(("AreaBump controlled series focus", focused));

    let cc = ChartConfig {
        margin: Margin {
            left: 70.,
            right: 30.,
            top: 75.,
            bottom: 45.,
        },
        ..frame.clone()
    };
    let mut cross = (24..=31)
        .map(|d| CalendarCell::new((2023, 12, d), Some(f64::from(d - 24))))
        .collect::<Vec<_>>();
    cross.extend(
        (1..=10).map(|d| {
            CalendarCell::new((2024, 1, d), if d == 4 { None } else { Some(f64::from(d)) })
        }),
    );
    let mut compact = calendar_layout(
        &cc,
        &cross,
        &CalendarLayoutOptions {
            square: false,
            year_labels: Some(true),
            ..calendar_options((2023, 12, 24), (2024, 1, 10))
        },
    )?
    .scene;
    note(
        &mut compact,
        &cc,
        "Only Dec 24–Jan 10 · Monday-first weeks · gray is a missing value",
    );
    output.push(("Compact calendar across New Year", compact));
    let mut leap = (24..=29)
        .map(|d| {
            CalendarCell::new(
                (2024, 2, d),
                if d == 29 {
                    None
                } else {
                    Some(f64::from(d - 24))
                },
            )
        })
        .collect::<Vec<_>>();
    leap.extend((1..=8).map(|d| CalendarCell::new((2024, 3, d), Some(f64::from(d + 5)))));
    let mut vertical = calendar_layout(
        &cc,
        &leap,
        &CalendarLayoutOptions {
            orientation: CalendarOrientation::Vertical,
            month_label_position: CalendarLegendPosition::After,
            year_labels: Some(true),
            day_radius: 7.,
            ..calendar_options((2024, 2, 24), (2024, 3, 8))
        },
    )?
    .scene;
    note(
        &mut vertical,
        &cc,
        "Vertical weeks include Feb 29 · actual month boundaries, rounded day cells",
    );
    output.push(("Vertical leap-day calendar", vertical));
    let mut annual = Vec::new();
    for year in [2023, 2024] {
        for (month, len) in [
            31,
            if year == 2024 { 29 } else { 28 },
            31,
            30,
            31,
            30,
            31,
            31,
            30,
            31,
            30,
            31,
        ]
        .into_iter()
        .enumerate()
        {
            for day in 1..=len {
                annual.push(CalendarCell::new(
                    (year, month as u8 + 1, day),
                    if day % 13 == 0 {
                        None
                    } else {
                        Some(f64::from((day + month as u8 * 3) % 16))
                    },
                ));
            }
        }
    }
    let mut years = calendar_layout(
        &cc,
        &annual,
        &CalendarLayoutOptions {
            mode: CalendarLayoutMode::FullYears,
            day_labels: CalendarDayLabel::None,
            day_spacing: 1.,
            day_radius: 1.,
            month_style: CalendarTextStyle {
                size: Some(10.),
                ..Default::default()
            },
            year_gap: 30.,
            ..calendar_options((2023, 1, 1), (2024, 12, 31))
        },
    )?
    .scene;
    note(
        &mut years,
        &cc,
        "Two Gregorian years · Monday-first weeks and concave month outlines",
    );
    output.push(("Styled full-year calendar panels", years));

    let sc = ChartConfig {
        margin: Margin {
            left: 80.,
            right: 60.,
            top: 65.,
            bottom: 50.,
        },
        ..frame.clone()
    };
    let boxes = box_samples();
    let bo = BoxPlotLayoutOptions {
        domain: Some([-14., 50.]),
        groups: Some(vec!["Week 3".into(), "Week 1".into(), "Week 2".into()]),
        fill_opacity: 0.4,
        border: Some(stroke(Color(45, 55, 75, 255), 1.5)),
        value_format: ValueFormat::Fixed(1),
        outlier_fill: Some(Color(220, 65, 85, 255)),
        ..Default::default()
    };
    let mut quantiles = boxplot_with_options(&sc, &boxes, &bo)?;
    note(
        &mut quantiles,
        &sc,
        "R-7 10/25/50/75/90 percentiles · 6 samples per box; None is omitted",
    );
    key(
        &mut quantiles,
        &sc,
        &[("Alpha", sc.theme.color(0)), ("Beta", sc.theme.color(1))],
    );
    output.push(("Grouped sample quantile box plots", quantiles));
    let mut tukey = boxplot_with_options(
        &sc,
        &boxes,
        &BoxPlotLayoutOptions {
            orientation: StatisticsOrientation::Horizontal,
            whiskers: BoxWhiskers::Tukey { factor: 1.5 },
            border_radius: 7.,
            fill_opacity: 0.65,
            outlier_shape: BoxOutlierShape::Diamond,
            cap_size: 0.85,
            ..bo
        },
    )?;
    note(
        &mut tukey,
        &sc,
        "Horizontal Tukey 1.5 IQR · original sample IDs own the diamond outliers",
    );
    key(
        &mut tukey,
        &sc,
        &[("Alpha", sc.theme.color(0)), ("Beta", sc.theme.color(1))],
    );
    output.push(("Horizontal Tukey boxes and outliers", tukey));
    let bc = ChartConfig {
        margin: Margin {
            left: 82.,
            right: 45.,
            top: 65.,
            bottom: 40.,
        },
        ..frame.clone()
    };
    let bu = BulletLayoutOptions {
        min: Some(-10.),
        max: Some(55.),
        spacing: 28.,
        title_align: TextAnchor::End,
        value_format: ValueFormat::Fixed(0),
        axis: AxisOptions {
            tick_count: 4,
            tick_values: Some(vec![-10., 0., 20., 40., 55.]),
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bullet = bullet_with_options(&bc, &bullets(), &bu)?;
    note(
        &mut bullet,
        &bc,
        "One shared −10..55 scale · adjacent ranges/measures; red strokes are targets",
    );
    output.push(("Bullet rows with a shared signed domain", bullet));
    let vc = ChartConfig {
        margin: Margin {
            left: 65.,
            right: 65.,
            top: 75.,
            bottom: 50.,
        },
        ..frame.clone()
    };
    let mut reversed = bullet_with_options(
        &vc,
        &bullets(),
        &BulletLayoutOptions {
            orientation: StatisticsOrientation::Vertical,
            reverse: true,
            title_align: TextAnchor::Middle,
            title_offset: [0., -15.],
            spacing: 65.,
            measure_size: 0.5,
            marker_size: 0.85,
            axis_position: BulletPosition::After,
            ..bu
        },
    )?;
    note(
        &mut reversed,
        &vc,
        "Vertical reverse maps larger values downward · measure and target widths differ",
    );
    output.push(("Vertical reversed bullet rows", reversed));

    let hc = ChartConfig {
        margin: Margin {
            left: 30.,
            right: 30.,
            top: 60.,
            bottom: 25.,
        },
        ..frame.clone()
    };
    let tree = hierarchy();
    let mut map = treemap_layout(
        &hc,
        &tree,
        None,
        &TreemapLayoutOptions {
            inner_padding: 5.,
            outer_padding: 5.,
            parent_label_size: 25.,
            parent_label_padding: 7.,
            style: hierarchy_style(),
            ..Default::default()
        },
    )?
    .scene;
    note(
        &mut map,
        &hc,
        "Reserved parent headers and padding are part of the nested allocation",
    );
    output.push(("Treemap with reserved parent headers", map));
    let mut packing_style = hierarchy_style();
    packing_style.parent.opacity = 0.14;
    packing_style.parent_labels.enabled = false;
    packing_style.leaf_labels.skip_width = 0.;
    packing_style.leaf_labels.skip_height = 0.;
    let mut packed = circle_packing_layout(
        &hc,
        &tree,
        Some("Cloud"),
        &CirclePackingLayoutOptions {
            padding: 5.,
            viewport_padding: 7.,
            style: packing_style,
            ..Default::default()
        },
    )?
    .scene;
    note(
        &mut packed,
        &hc,
        "Focus Cloud: a fresh subtree viewport · Compute / Storage / API keep their IDs",
    );
    output.push(("Circle packing focused subtree", packed));
    let mut right = icicle_layout(
        &hc,
        &tree,
        None,
        &IcicleLayoutOptions {
            direction: IcicleDirection::Right,
            padding: 4.,
            style: hierarchy_style(),
        },
    )?
    .scene;
    note(
        &mut right,
        &hc,
        "Root → groups → leaves · rightward depth with independent node labels",
    );
    output.push(("Icicle rightward hierarchy", right));
    let mut left = icicle_layout(
        &hc,
        &tree,
        Some("Cloud"),
        &IcicleLayoutOptions {
            direction: IcicleDirection::Left,
            padding: 4.,
            style: hierarchy_style(),
        },
    )?
    .scene;
    note(
        &mut left,
        &hc,
        "Focus Cloud · leftward depth and fresh subtree allocations, stable leaf IDs",
    );
    output.push(("Icicle leftward focused subtree", left));
    for (_, scene) in &output {
        scene.validate()?;
    }
    Ok(output)
}
