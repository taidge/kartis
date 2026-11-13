//! Typed scatter snapshots; no event or nearest-point behavior is fabricated.
use glory::render::{GraphicStroke, StrokeCap};
use kartis::*;
fn caption(scene: &mut Scene, c: &ChartConfig, text: &str) {
    if !scene.draw_order.is_empty() {
        scene.draw_order.push(DrawItem::Label(scene.labels.len()));
    }
    scene.labels.push(Label {
        x: c.width / 2.,
        y: c.height - 9.,
        text: text.into(),
        color: Color(70, 90, 110, 255),
        size: 11.,
        anchor: TextAnchor::Middle,
        rotation: 0.,
    });
}
fn fixed(domain: [f64; 2]) -> ScatterAxisScale {
    ScatterAxisScale::Continuous {
        options: ContinuousScaleOptions::default(),
        min: ScatterBound::Fixed(domain[0]),
        max: ScatterBound::Fixed(domain[1]),
    }
}
fn numeric() -> Vec<ScatterSeries> {
    vec![
        ScatterSeries::new(
            "North",
            vec![
                ScatterDatum::new("a", 2., 4.),
                ScatterDatum::new("b", 5., 8.),
                ScatterDatum::new("c", 9., 3.),
            ],
        ),
        ScatterSeries::new(
            "South",
            vec![
                ScatterDatum::new("d", 4., 6.),
                ScatterDatum::new("e", 7., 1.),
            ],
        ),
    ]
}
pub fn scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let data = numeric();
    let mut default = scatter_layout(c, &data, &ScatterLayoutOptions::default())?.scene;
    caption(
        &mut default,
        c,
        "Snapshot: actual Nivo defaults, circle diameter 9; min 0, max auto, nice enabled",
    );
    let mut sized = data.clone();
    for (point, value) in sized
        .iter_mut()
        .flat_map(|s| s.data.iter_mut())
        .zip([-10., 0., 50., 100., 150.])
    {
        point.size_value = Some(value);
    }
    let mut options = ScatterLayoutOptions {
        scales: ScatterScaleOptions {
            x: fixed([0., 10.]),
            y: fixed([0., 10.]),
        },
        labels: ScatterLabels {
            enabled: true,
            offset: [0., -28.],
            ..Default::default()
        },
        ..Default::default()
    };
    options.node.shape = ScatterNodeShape::Circle {
        diameter: ScatterSize::Linear {
            values: [0., 100.],
            sizes: [10., 30.],
            clamp: false,
            missing: ScatterMissingSize::Error,
        },
    };
    let mut dynamic = scatter_layout(c, &sized, &options)?.scene;
    caption(
        &mut dynamic,
        c,
        "Snapshot: values −10/0/50/100/150 → diameters 8/10/20/30/40; unclamped extrapolation",
    );
    let glyph_data = [ScatterSeries::new(
        "Glyphs",
        vec![
            ScatterDatum::new("left", 2., 5.),
            ScatterDatum::new("center", 5., 5.),
            ScatterDatum::new("right", 8., 5.),
        ],
    )];
    options.labels.offset = [0., -38.];
    options.node.shape = ScatterNodeShape::FilledGlyph {
        shape: FilledGlyph::Star,
        area: ScatterSize::Fixed(900.),
        rotation: 0.37,
    };
    let mut stars = scatter_layout(c, &glyph_data, &options)?.scene;
    caption(
        &mut stars,
        c,
        "Snapshot: each Star has area 900; D3 origin follows actual Path supports, not its bbox center",
    );
    options.node.shape = ScatterNodeShape::StrokedGlyph {
        shape: StrokedGlyph::Triangle,
        size: ScatterSize::Fixed(900.),
        rotation: 0.31,
    };
    options.node.stroke = Some(GraphicStroke {
        color: Color(30, 85, 155, 255),
        width: 2.,
        cap: StrokeCap::Round,
    });
    let mut hollow = scatter_layout(c, &glyph_data, &options)?.scene;
    caption(
        &mut hollow,
        c,
        "Snapshot: stroked Triangle nominal size 900; empty centers, original origins and real plot clipping",
    );
    const FEB28: f64 = 1_709_078_400_000.;
    const DAY: f64 = 86_400_000.;
    let dates = [ScatterSeries::new(
        "Dates",
        vec![
            ScatterDatum::new("Feb 28", FEB28, 2.),
            ScatterDatum::new("Leap day", FEB28 + DAY, 6.),
            ScatterDatum::new("Mar 1", FEB28 + 2. * DAY, 4.),
        ],
    )];
    let mut utc_options = ScatterLayoutOptions {
        scales: ScatterScaleOptions {
            x: ScatterAxisScale::Continuous {
                options: ContinuousScaleOptions {
                    kind: ScaleKind::Utc,
                    ..Default::default()
                },
                min: ScatterBound::Auto,
                max: ScatterBound::Auto,
            },
            y: ScatterAxisScale::Continuous {
                options: ContinuousScaleOptions {
                    reverse: true,
                    ..Default::default()
                },
                min: ScatterBound::Fixed(0.),
                max: ScatterBound::Fixed(10.),
            },
        },
        x_format: ValueFormat::UtcDate,
        labels: ScatterLabels {
            enabled: true,
            ..Default::default()
        },
        ..Default::default()
    };
    utc_options.axes.bottom.as_mut().unwrap().format = ValueFormat::UtcDate;
    utc_options.axes.bottom.as_mut().unwrap().tick_values =
        Some(vec![FEB28, FEB28 + DAY, FEB28 + 2. * DAY]);
    let mut utc_frame = c.clone();
    utc_frame.margin.right = 70.;
    let mut utc = scatter_layout(&utc_frame, &dates, &utc_options)?.scene;
    caption(
        &mut utc,
        c,
        "Snapshot: UTC leap-day instants on X; reversed Y; raw values remain epoch milliseconds",
    );
    let categories = [ScatterSeries::new(
        "Typed",
        vec![
            ScatterDatum::new("text one", "1", "up"),
            ScatterDatum::new("number one", 1., "down"),
            ScatterDatum::new("last", "last", "up"),
        ],
    )];
    let point = ScatterAxisScale::Point {
        options: Default::default(),
        domain: None,
        window: None,
    };
    let mut point_xy = scatter_layout(
        c,
        &categories,
        &ScatterLayoutOptions {
            scales: ScatterScaleOptions {
                x: point.clone(),
                y: point,
            },
            labels: ScatterLabels {
                enabled: true,
                offset: [0., -13.],
                ..Default::default()
            },
            ..Default::default()
        },
    )?
    .scene;
    caption(
        &mut point_xy,
        c,
        "Snapshot: string ‘1’ and number 1 are different point positions; Point Y follows Nivo top→bottom",
    );
    let scenes = vec![
        ("Scatter: default nine-pixel circles", default),
        ("Scatter: dynamic diameter extrapolation", dynamic),
        ("Scatter: filled Star area and origins", stars),
        ("Scatter: hollow Triangle centers", hollow),
        ("Scatter: UTC and reversed value scale", utc),
        ("Scatter: typed point X and Y", point_xy),
    ];
    for (_, scene) in &scenes {
        scene.validate()?;
    }
    Ok(scenes)
}
