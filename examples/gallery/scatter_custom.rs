use glory::render::{GraphicPoint, GraphicStroke, PathCommand, StrokeCap};
use kartis::*;

pub fn scene(config: &ChartConfig) -> Result<Scene> {
    let data = [
        ScatterSeries::new(
            "Measured",
            vec![
                ScatterDatum::new("a", 1.8, 2.8),
                ScatterDatum::new("b", 3.4, 5.8),
                ScatterDatum::new("c", 5.2, 3.6),
                ScatterDatum::new("d", 7.6, 7.2),
            ],
        ),
        ScatterSeries::new(
            "Estimated",
            vec![
                ScatterDatum::new("cropped", -0.35, 7.8),
                ScatterDatum::new("e", 2.5, 7.4),
                ScatterDatum::new("f", 5.9, 6.1),
                ScatterDatum::new("g", 8.3, 4.2),
            ],
        ),
    ];
    let axis = ScatterAxisScale::Continuous {
        options: ContinuousScaleOptions::default(),
        min: ScatterBound::Fixed(0.),
        max: ScatterBound::Fixed(10.),
    };
    let mut options = ScatterLayoutOptions {
        scales: ScatterScaleOptions {
            x: axis.clone(),
            y: axis,
        },
        ..Default::default()
    };
    options.node.shape = ScatterNodeShape::Circle {
        diameter: ScatterSize::Fixed(25.),
    };
    let layout = scatter_layout_with_node_factory(config, &data, &options, |node| {
        let [x, y] = node.position;
        let r = node.size / 2.;
        let p = |dx: f64, dy: f64| GraphicPoint::new(x + r * dx, y + r * dy);
        if node.series_id == "Measured" {
            let geometry = Geometry::Path(vec![
                PathCommand::Move(p(-1.45, -0.9)),
                PathCommand::Line(p(0.95, -0.9)),
                PathCommand::Line(p(1.55, 0.)),
                PathCommand::Line(p(0.95, 0.9)),
                PathCommand::Line(p(-1.45, 0.9)),
                PathCommand::Line(p(-0.78, 0.)),
                PathCommand::Close,
            ]);
            let mut shape = ScatterNodeGeometry::inherit(geometry, node);
            shape.fill_paint = Some(FillPaint::Linear {
                start: [x - r, y],
                end: [x + r, y],
                stops: vec![
                    GradientStop::new(0., Color(52, 103, 174, 255)),
                    GradientStop::new(1., Color(39, 154, 164, 255)),
                ],
            });
            Ok(shape)
        } else {
            let geometry = Geometry::Path(vec![
                PathCommand::Move(p(0.95, -1.05)),
                PathCommand::Line(p(2.85, -1.05)),
                PathCommand::Line(p(3.08, 0.62)),
                PathCommand::Line(p(0.95, 0.85)),
                PathCommand::Close,
            ]);
            let mut shape = ScatterNodeGeometry::inherit(geometry, node);
            shape.fill = None;
            shape.stroke = Some(GraphicStroke {
                color: node.color,
                width: 3.,
                cap: StrokeCap::Round,
            });
            shape.stroke_dash = Some(StrokeDash {
                segments: vec![8., 4.],
                offset: 1.,
            });
            Ok(shape)
        }
    })?;
    Ok(layout.scene)
}
