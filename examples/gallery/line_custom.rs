use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use kartis::*;

pub fn scene(c: &ChartConfig) -> Result<(&'static str, Scene)> {
    let data = [Series::new(
        "Signal",
        vec![
            Point::new(0., 2.),
            Point::new(1., 5.),
            Point::new(2., 3.),
            Point::new(3., 8.),
        ],
    )];
    let line = line_layout_with_point_factory(
        c,
        &data,
        &LineOptions {
            x_domain: Some([0., 3.]),
            y_domain: Some([0., 10.]),
            points: true,
            ..Default::default()
        },
        &LineStyleOptions {
            series: LineSeriesStyle {
                point_labels: Some(LinePointLabels {
                    offset: [0., -17.],
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        },
        None,
        |point| {
            let [x, y] = point.position;
            let x = x + if point.source_index == 3 { 6. } else { 16. };
            let mut dot = point.inherit(Geometry::Path(vec![
                C::Move(P::new(x, y - 9.)),
                C::Line(P::new(x + 12., y)),
                C::Line(P::new(x, y + 9.)),
                C::Line(P::new(x - 12., y)),
                C::Close,
            ]));
            dot.fill = None;
            dot.stroke = Some(GraphicStroke {
                color: Color(171, 38, 91, 255),
                width: 3.,
                cap: StrokeCap::Round,
            });
            Ok(dot)
        },
    )?;
    Ok(("Line · offset hollow custom points", line.scene))
}
