//! Static public-API profiles; captions describe authored state, not live input.
use std::f64::consts::{FRAC_PI_4, PI};

use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use kartis::*;
fn text(scene: &mut Scene, x: f64, y: f64, value: &str, size: f64) {
    if !scene.draw_order.is_empty() {
        scene.draw_order.push(DrawItem::Label(scene.labels.len()));
    }
    scene.labels.push(Label {
        x,
        y,
        text: value.into(),
        size,
        color: Color(65, 85, 105, 255),
        anchor: TextAnchor::Middle,
        rotation: 0.,
    });
}
fn caption(scene: &mut Scene, c: &ChartConfig, value: &str) {
    text(scene, c.width / 2., c.height - 10., value, 11.);
}
fn continuous(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let data = [
        Series::new(
            "North",
            vec![
                Point::new(-3., 12.5),
                Point::new(3.3, 4.25),
                Point::new(7.2, 7.4),
            ],
        ),
        Series::new(
            "South",
            vec![
                Point::new(1.4, 2.),
                Point::new(4.6, 3.),
                Point::new(8.2, 5.),
            ],
        ),
    ];
    let style = LineStyleOptions {
        series: LineSeriesStyle {
            point_radius: 5.,
            width: 3.,
            ..Default::default()
        },
        ..Default::default()
    };
    let line = LineOptions {
        x_domain: Some([0.13, 9.2]),
        y_domain: Some([0.3, 8.1]),
        ..Default::default()
    };
    let scale = ContinuousScaleOptions {
        nice: ScaleNice::Count(5),
        reverse: true,
        round: true,
        clamp: true,
        ..Default::default()
    };
    let mut linear = line_layout_with_scale_options(
        c,
        &data,
        &line,
        &style,
        &LineScaleOptions { x: scale, y: scale },
    )?
    .layout
    .scene;
    caption(
        &mut linear,
        c,
        "Profile snapshot: nice [0,10], reversed rounded pixels; clipped raw datum = 12.5",
    );
    let log_data = [
        Series::new(
            "North",
            vec![
                Point::new(1.3, -8.),
                Point::new(9., 6.),
                Point::new(63., 31.),
            ],
        ),
        Series::new(
            "South",
            vec![
                Point::new(2.5, 13.),
                Point::new(17., -3.),
                Point::new(80., 23.),
            ],
        ),
    ];
    let mut log = line_layout_with_scale_options(
        c,
        &log_data,
        &LineOptions {
            x_domain: Some([0.7, 90.]),
            y_domain: Some([-13., 41.]),
            ..Default::default()
        },
        &style,
        &LineScaleOptions {
            x: ContinuousScaleOptions {
                kind: ScaleKind::Log,
                log_base: 2.,
                nice: ScaleNice::Default,
                ..Default::default()
            },
            y: ContinuousScaleOptions {
                kind: ScaleKind::SymLog { constant: 2. },
                nice: ScaleNice::Count(5),
                ..Default::default()
            },
        },
    )?
    .layout
    .scene;
    caption(
        &mut log,
        c,
        "Profile snapshot: base-2 powers on X; SymLog geometry with original-value Y ticks",
    );
    const FEB27: f64 = 1_708_992_000_000.;
    const DAY: f64 = 86_400_000.;
    let dates = [
        Series::new(
            "North",
            vec![
                Point::new(FEB27 + DAY * 1.5, 3.),
                Point::new(FEB27 + DAY * 3.25, 6.75),
                Point::new(FEB27 + DAY * 4.5, 8.),
            ],
        ),
        Series::new(
            "South",
            vec![
                Point::new(FEB27 + DAY * 0.75, 7.),
                Point::new(FEB27 + DAY * 2.75, 2.),
                Point::new(FEB27 + DAY * 4.8, 5.),
            ],
        ),
    ];
    let mut opts = LineOptions {
        x_domain: Some([FEB27 + DAY * 0.5, FEB27 + DAY * 5.125]),
        y_domain: Some([0., 10.]),
        ..Default::default()
    };
    opts.axes.x.format = ValueFormat::UtcDate;
    opts.axes.x.tick_count = 4;
    opts.axes.x.tick_rotation = -15.;
    let utc_frame = ChartConfig {
        margin: Margin {
            right: 70.,
            ..c.margin
        },
        ..c.clone()
    };
    let mut utc = line_layout_with_scale_options(
        &utc_frame,
        &dates,
        &opts,
        &style,
        &LineScaleOptions {
            x: ContinuousScaleOptions {
                kind: ScaleKind::Utc,
                nice: ScaleNice::Count(4),
                reverse: true,
                round: true,
                ..Default::default()
            },
            ..Default::default()
        },
    )?
    .layout
    .scene;
    caption(
        &mut utc,
        c,
        "Profile snapshot: real UTC calendar nice; reversed dates include leap day 2024-02-29",
    );
    Ok(vec![
        ("Scales · reversed rounded clamped line", linear),
        ("Scales · base-2 log and value-domain SymLog", log),
        ("Scales · UTC calendar nice and reverse", utc),
    ])
}
fn point_scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let keys = vec![
        CategoryKey::from("1"),
        1_f64.into(),
        "March".into(),
        "April".into(),
        "May".into(),
    ];
    let data = [
        PointLineSeries::new(
            "North",
            vec![
                PointLineDatum::new("1", 3.),
                PointLineDatum::new(1_f64, 7.),
                PointLineDatum::gap("March"),
                PointLineDatum::new("April", 4.),
                PointLineDatum::new("May", 8.),
            ],
        ),
        PointLineSeries::new(
            "South",
            keys.iter()
                .cloned()
                .zip([7., 3., 2., 8., 5.])
                .map(|(x, y)| PointLineDatum::new(x, y))
                .collect(),
        ),
    ];
    let number = ValueFormat::Affix {
        prefix: "#".into(),
        suffix: String::new(),
        precision: 0,
    };
    let style = LineStyleOptions {
        series: LineSeriesStyle {
            point_radius: 5.,
            point_labels: Some(LinePointLabels {
                text: LinePointLabelText::Coordinates {
                    x: number.clone(),
                    y: ValueFormat::Fixed(0),
                },
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    let mut opts = PointLineOptions {
        domain: Some(keys),
        line: LineOptions {
            y_domain: Some([0., 10.]),
            ..Default::default()
        },
        ..Default::default()
    };
    opts.line.axes.x.format = number;
    let mut full = point_line_layout(c, &data, &opts, &style)?.scene;
    caption(
        &mut full,
        c,
        "Profile snapshot: text \"1\" and numeric #1 are distinct; March remains a genuine gap",
    );
    opts.window = Some([0.5, 3.5]);
    opts.point.round = true;
    let mut window = point_line_layout(c, &data, &opts, &style)?.scene;
    caption(
        &mut window,
        c,
        "Profile snapshot: ordinal window [0.5,3.5], rounded slots; labels keep original keys",
    );
    Ok(vec![
        ("Point · typed categories and gaps", full),
        ("Point · ordinal affine window", window),
    ])
}
fn glyph_scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let filled = [
        ("Circle", FilledGlyph::Circle),
        ("Cross", FilledGlyph::Cross),
        ("Diamond", FilledGlyph::Diamond),
        ("Square", FilledGlyph::Square),
        ("Star", FilledGlyph::Star),
        ("Triangle", FilledGlyph::Triangle),
        ("Wye", FilledGlyph::Wye),
    ];
    let stroked = [
        ("Circle", StrokedGlyph::Circle),
        ("Plus", StrokedGlyph::Plus),
        ("Times", StrokedGlyph::Times),
        ("Triangle", StrokedGlyph::Triangle),
        ("Asterisk", StrokedGlyph::Asterisk),
        ("Square", StrokedGlyph::Square),
        ("Diamond", StrokedGlyph::Diamond),
    ];
    let mut fill = c.scene()?;
    let mut stroke = c.scene()?;
    text(
        &mut fill,
        c.width / 2.,
        75.,
        "Same mathematical area: 900 logical px²",
        16.,
    );
    text(
        &mut stroke,
        c.width / 2.,
        75.,
        "D3 stroke-size parameter: 400",
        16.,
    );
    for (i, (name, shape)) in filled.into_iter().enumerate() {
        let p = [75. + i as f64 * 95., 190.];
        let geometry = glyph_geometry(
            p,
            &GlyphOptions {
                shape: GlyphShape::Filled(shape),
                size: 900.,
                rotation: 0.,
            },
        )?;
        let mut mark = glyph_mark(
            format!("gallery-glyph-fill:{name}"),
            &geometry,
            Some(c.theme.color(i)),
            None,
        )?;
        mark.series = name.into();
        mark.label = name.into();
        mark.value = 900.;
        fill.marks.push(mark);
        text(&mut fill, p[0], 250., name, 13.);
    }
    for (i, (name, shape)) in stroked.into_iter().enumerate() {
        let p = [75. + i as f64 * 95., 190.];
        let geometry = glyph_geometry(
            p,
            &GlyphOptions {
                shape: GlyphShape::Stroked(shape),
                size: 400.,
                rotation: 0.,
            },
        )?;
        let mut mark = glyph_mark(
            format!("gallery-glyph-stroke:{name}"),
            &geometry,
            None,
            Some(GraphicStroke {
                color: c.theme.color(i),
                width: 1.5,
                cap: StrokeCap::Round,
            }),
        )?;
        mark.series = name.into();
        mark.label = name.into();
        mark.value = 400.;
        stroke.marks.push(mark);
        text(&mut stroke, p[0], 250., name, 13.);
    }
    caption(
        &mut fill,
        c,
        "Profile snapshot: seven D3 filled symbols; size is geometric area, not bounding-box area",
    );
    caption(
        &mut stroke,
        c,
        "Profile snapshot: true centerlines, fill none, 1.5 px round caps; nominal size is not ink area",
    );
    let mut rotated = c.scene()?;
    for (i, (name, shape, rotation)) in [
        ("Triangle", FilledGlyph::Triangle, FRAC_PI_4),
        ("Star", FilledGlyph::Star, PI / 5.),
        ("Wye", FilledGlyph::Wye, -PI / 6.),
    ]
    .into_iter()
    .enumerate()
    {
        let center = [180. + i as f64 * 180., 180.];
        let geometry = glyph_geometry(
            center,
            &GlyphOptions {
                shape: GlyphShape::Filled(shape),
                size: 1600.,
                rotation,
            },
        )?;
        let mut shape = glyph_mark(
            format!("gallery-glyph-rotated:{name}"),
            &geometry,
            Some(c.theme.color(i)),
            None,
        )?;
        shape.series = name.into();
        shape.label = format!("{name}: true anchor {}, {}", center[0], center[1]);
        shape.value = 1600.;
        rotated.marks.push(shape);
        let mut guide = Mark::new(
            format!("gallery-glyph-guide:{name}"),
            Geometry::Path(vec![
                C::Move(P::new(center[0] - 45., center[1])),
                C::Line(P::new(center[0] + 45., center[1])),
                C::Move(P::new(center[0], center[1] - 45.)),
                C::Line(P::new(center[0], center[1] + 45.)),
            ]),
            Color(75, 90, 105, 255),
        );
        guide.label.clear();
        guide.fill = None;
        guide.stroke = Some(GraphicStroke {
            color: Color(75, 90, 105, 255),
            width: 0.8,
            cap: StrokeCap::Butt,
        });
        guide.stroke_dash = Some(StrokeDash {
            segments: vec![3., 3.],
            offset: 0.,
        });
        rotated.marks.push(guide);
        let mut anchor = Mark::new(
            format!("gallery-glyph-anchor:{name}"),
            Geometry::Circle {
                x: geometry.anchor[0],
                y: geometry.anchor[1],
                radius: 2.5,
            },
            Color(15, 30, 50, 255),
        );
        anchor.label.clear();
        rotated.marks.push(anchor);
        text(&mut rotated, center[0], 270., name, 14.);
        text(
            &mut rotated,
            center[0],
            295.,
            &format!("anchor ({}, {})", center[0], center[1]),
            11.,
        );
    }
    caption(
        &mut rotated,
        c,
        "Profile snapshot: rotated D3 origin markers; asymmetric glyph bounds do not define the anchor",
    );
    Ok(vec![
        ("Glyphs · seven equal-area filled symbols", fill),
        ("Glyphs · seven stroke-only centerlines", stroke),
        ("Glyphs · rotated true anchors", rotated),
    ])
}
fn radar_scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let c = ChartConfig {
        margin: Margin {
            left: 90.,
            right: 90.,
            top: 55.,
            bottom: 55.,
        },
        ..c.clone()
    };
    let axes = ["Speed", "Quality", "Coverage", "Efficiency", "Uptime"].map(str::to_string);
    let data = [
        RadarSeries::new("North", vec![80., 65., 90., 55., 75.]),
        RadarSeries::new("South", vec![50., 85., 65., 80., 60.]),
    ];
    let mut options = RadarLayoutOptions {
        domain: Some([0., 100.]),
        ..Default::default()
    };
    options.series.fill_opacity = 0.14;
    options.points.glyph = Some(GlyphOptions {
        shape: GlyphShape::Filled(FilledGlyph::Wye),
        size: 110.,
        rotation: 0.2,
    });
    let mut wye = radar_with_options(&c, &axes, &data, &options)?;
    caption(
        &mut wye,
        &c,
        "Profile snapshot: explicit filled Wye area 110; series opacity does not dim point ink",
    );
    options.points.glyph = Some(GlyphOptions {
        shape: GlyphShape::Stroked(StrokedGlyph::Triangle),
        size: 160.,
        rotation: 0.35,
    });
    options.points.border = RadarLineStyle {
        enabled: true,
        width: 2.,
        ..Default::default()
    };
    let mut hollow = radar_with_options(&c, &axes, &data, &options)?;
    caption(
        &mut hollow,
        &c,
        "Profile snapshot: explicit hollow Triangle glyph, 2 px border; origin stays at each datum",
    );
    Ok(vec![
        ("Radar · filled Wye glyphs", wye),
        ("Radar · hollow triangle glyphs", hollow),
    ])
}
pub fn scenes(frame: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let c = ChartConfig {
        width: 720.,
        height: 400.,
        margin: Margin {
            left: 70.,
            right: 30.,
            top: 35.,
            bottom: 65.,
        },
        ..frame.clone()
    };
    let mut scenes = continuous(&c)?;
    scenes.extend(point_scenes(&c)?);
    scenes.extend(glyph_scenes(&c)?);
    scenes.extend(radar_scenes(&c)?);
    for (_, scene) in &scenes {
        scene.validate()?;
    }
    Ok(scenes)
}
