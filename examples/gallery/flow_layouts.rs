use glory::render::{GraphicPoint, PathCommand};
use kartis::*;

pub fn scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let ids = ["Cloud", "Devices", "Services"].map(str::to_owned);
    let matrix = [vec![3., 8., 4.], vec![14., 2., 7.], vec![6., 11., 1.]];
    let chord_options = ChordLayoutOptions {
        pad_angle: 0.06,
        inner_radius_ratio: 0.85,
        inner_radius_offset: 0.05,
        labels: Some(ChordLabelOptions {
            orientation: ChordLabelOrientation::Horizontal,
            ..Default::default()
        }),
        ..Default::default()
    };
    let chord_initial = chord_layout(c, &ids, &matrix, &chord_options)?.scene;
    let chord_changed = chord_layout(
        c,
        &ids,
        &[vec![3., 20., 4.], vec![14., 2., 7.], vec![6., 11., 1.]],
        &chord_options,
    )?
    .scene;

    let data = [
        BumpSeries::new(
            "North",
            vec![
                BumpDatum::new("Q1", 1.),
                BumpDatum::new("Q2", 4.),
                BumpDatum::new("Q3", 10.),
                BumpDatum::new("Q4", 4.),
            ],
        ),
        BumpSeries::new(
            "South",
            vec![
                BumpDatum::new("Q1", 10.),
                BumpDatum::new("Q2", 1.),
                BumpDatum::gap("Q3"),
                BumpDatum::new("Q4", 10.),
            ],
        ),
        BumpSeries::new(
            "West",
            vec![
                BumpDatum::new("Q1", 4.),
                BumpDatum::new("Q2", 10.),
                BumpDatum::new("Q3", 4.),
                BumpDatum::new("Q4", 1.),
            ],
        ),
    ];
    let bump = bump_layout(c, &data, &BumpLayoutOptions::default())?.scene;
    let badges =
        bump_layout_with_point_factory(c, &data, &BumpLayoutOptions::default(), |context| {
            let [x, y] = context.position;
            let p = GraphicPoint::new;
            Ok(context.inherit(Geometry::Path(vec![
                PathCommand::Move(p(x + 10., y - 8.)),
                PathCommand::Line(p(x + 18., y)),
                PathCommand::Line(p(x + 10., y + 8.)),
                PathCommand::Line(p(x + 2., y)),
                PathCommand::Close,
            ])))
        })?
        .scene;

    let categories = ["Q1", "Q2", "Q3", "Q4", "Q5"].map(CategoryKey::from);
    let stream_data = [
        BarSeries::new(
            "Search",
            vec![Some(12.), Some(30.), Some(18.), Some(28.), Some(16.)],
        ),
        BarSeries::new(
            "Direct",
            vec![Some(24.), Some(18.), Some(28.), Some(16.), Some(26.)],
        ),
        BarSeries::new(
            "Partners",
            vec![Some(8.), Some(14.), Some(10.), Some(18.), Some(12.)],
        ),
    ];
    let options = StreamLayoutOptions {
        stack: stack::StackOptions {
            order: stack::StackOrder::InsideOut,
            offset: stack::StackOffset::Wiggle,
        },
        curve: Curve::Basis,
        legend: true,
        ..Default::default()
    };
    let stream = stream_layout(c, &categories, &stream_data, &options)?.scene;
    let dots = stream_layout(
        c,
        &categories,
        &stream_data,
        &StreamLayoutOptions {
            curve: Curve::CatmullRom { alpha: 0.5 },
            dots: Some(StreamDotOptions {
                position: StreamDotPosition::End,
                size: 8.,
                ..Default::default()
            }),
            ..options.clone()
        },
    )?
    .scene;
    let stars = stream_layout(
        c,
        &categories,
        &stream_data,
        &StreamLayoutOptions {
            dots: Some(StreamDotOptions {
                glyph: Some(GlyphOptions {
                    shape: GlyphShape::Filled(FilledGlyph::Star),
                    size: 120.,
                    rotation: 0.2,
                }),
                ..Default::default()
            }),
            ..options.clone()
        },
    )?
    .scene;
    let custom = stream_layout_with_dot_factory(
        c,
        &categories,
        &stream_data,
        &StreamLayoutOptions {
            dots: Some(StreamDotOptions::default()),
            ..options
        },
        |context| {
            let [x, y] = context.position;
            let p = GraphicPoint::new;
            Ok(context.inherit(Geometry::Path(vec![
                PathCommand::Move(p(x + 7., y - 6.)),
                PathCommand::Line(p(x + 13., y)),
                PathCommand::Line(p(x + 7., y + 6.)),
                PathCommand::Line(p(x + 1., y)),
                PathCommand::Close,
            ])))
        },
    )?
    .scene;
    let diverging = stream_layout(
        c,
        &categories,
        &[
            BarSeries::new(
                "North",
                vec![Some(12.), Some(-18.), Some(16.), Some(-24.), Some(12.)],
            ),
            BarSeries::new(
                "South",
                vec![Some(-8.), Some(12.), None, Some(16.), Some(-10.)],
            ),
        ],
        &StreamLayoutOptions {
            stack: stack::StackOptions {
                offset: stack::StackOffset::Diverging,
                ..Default::default()
            },
            curve: Curve::Linear,
            legend: true,
            ..Default::default()
        },
    )?
    .scene;
    Ok(vec![
        ("Chord · directional ribbon values", chord_initial),
        ("Chord · changed dominant source", chord_changed),
        ("Bump · distinct rank steps and gaps", bump),
        ("Bump · offset datum diamonds", badges),
        ("Stream · InsideOut and Wiggle", stream),
        ("Stream · end-boundary dots", dots),
        ("Stream · signed diverging layers", diverging),
        ("Stream · shared star dots", stars),
        ("Stream · offset custom dots", custom),
    ])
}
