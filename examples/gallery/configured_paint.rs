use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use kartis::*;

pub fn scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let colors = OrdinalColorOptions {
        palette: OrdinalColorPalette::Categorical(CategoricalColorScheme::Category10),
        domain: Some(
            ["North", "South", "Central"]
                .map(CategoryKey::from)
                .to_vec(),
        ),
        ..Default::default()
    }
    .resolve([])?;
    let slices = [
        RadialDatum::new("South", 28.),
        RadialDatum::new("Central", 32.),
        RadialDatum::new("North", 40.),
    ];
    let mut pie_options = PieLayoutOptions::default();
    pie_options.pie.inner_radius = 0.43;
    pie_options.pie.pad_angle = 0.035;
    pie_options.pie.style.border = Some(GraphicStroke {
        color: Color(0, 0, 0, 255),
        width: 2.,
        cap: StrokeCap::Round,
    });
    pie_options.pie.style.link_labels.enabled = true;
    let pie = pie_layout_with_ordinal_colors(
        c,
        &slices,
        &pie_options,
        &colors,
        &PieColorStyles {
            border: Some(InheritedColor::FromContext(vec![ColorModifier::Darker(
                0.8,
            )])),
            arc_label: Some(InheritedColor::Contrast {
                dark: Color(0, 0, 0, 255),
                light: Color(255, 255, 255, 255),
            }),
            link_line: Some(InheritedColor::FromContext(vec![ColorModifier::Opacity(
                0.7,
            )])),
            ..Default::default()
        },
    )?;

    let bar = bar_layout_with_ordinal_colors(
        c,
        &["Central".into(), "North".into(), "South".into()],
        &[BarSeries::new(
            "Orders",
            vec![Some(32.), Some(40.), Some(28.)],
        )],
        &BarOptions {
            corner_radius: 5.,
            ..Default::default()
        },
        &BarStyleOptions {
            color_by: BarColorBy::Category,
            ..Default::default()
        },
        &colors,
    )?;

    let line = line_layout_with_ordinal_colors(
        c,
        &[
            Series::new(
                "Central",
                vec![
                    Point::new(0., 14.),
                    Point::new(1., 22.),
                    Point::new(2., 32.),
                ],
            ),
            Series::new(
                "South",
                vec![
                    Point::new(0., 15.),
                    Point::new(1., 25.),
                    Point::new(2., 28.),
                ],
            ),
            Series::new(
                "North",
                vec![
                    Point::new(0., 25.),
                    Point::new(1., 30.),
                    Point::new(2., 40.),
                ],
            ),
        ],
        &LineOptions {
            points: true,
            ..Default::default()
        },
        &LineStyleOptions::default(),
        &colors,
    )?;

    let axes = ["Range", "Focus", "Clarity", "Reach"].map(str::to_string);
    let radar = radar_layout_with_point_factory(
        c,
        &axes,
        &[RadarSeries::new("Measured", vec![70., 96., 56., 80.])],
        &RadarLayoutOptions {
            domain: Some([0., 100.]),
            points: RadarPointOptions {
                labels: true,
                label_offset: [0., -16.],
                ..Default::default()
            },
            ..Default::default()
        },
        &RadarInteractionOptions {
            slices: false,
            ..Default::default()
        },
        |point| {
            let [x, y] = point.position;
            let offset = if point.axis_index == 1 { 24. } else { 7. };
            let x = x + offset;
            let mut ink = point.inherit(Geometry::Path(vec![
                C::Move(P::new(x, y - 9.)),
                C::Line(P::new(x + 12., y)),
                C::Line(P::new(x, y + 9.)),
                C::Line(P::new(x - 12., y)),
                C::Close,
            ]));
            ink.fill = None;
            ink.stroke = Some(GraphicStroke {
                color: Color(171, 38, 91, 255),
                width: 2.5,
                cap: StrokeCap::Round,
            });
            Ok(ink)
        },
    )?;

    let bullet = bullet_layout_with_scale_options(
        c,
        &[
            BulletDatum {
                id: "Requests".into(),
                ranges: vec![10., 100.],
                measures: vec![28., 450.],
                markers: vec![100.],
            },
            BulletDatum {
                id: "Sessions".into(),
                ranges: vec![10., 100.],
                measures: vec![50., 700.],
                markers: vec![300.],
            },
        ],
        &BulletLayoutOptions {
            min: Some(3.),
            max: Some(760.),
            axis: AxisOptions {
                tick_values: Some(vec![1., 10., 100., 1000.]),
                ..Default::default()
            },
            ..Default::default()
        },
        &ContinuousScaleOptions {
            kind: ScaleKind::Log,
            nice: ScaleNice::Default,
            ..Default::default()
        },
    )?;
    Ok(vec![
        ("Pie · ordinal colors and contrast", pie.scene),
        ("Bar · fixed ordinal color domain", bar.scene),
        ("Line · ordinal series colors", line.scene),
        ("Radar · offset custom point outlines", radar.scene),
        (
            "Bullet · logarithmic ranges and markers",
            bullet.layout.scene,
        ),
    ])
}
