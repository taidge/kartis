//! Shared native/browser fixture: customization follows the accepted packing.
use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use kartis::*;

pub fn build(
    width: f64,
    height: f64,
    vertical: bool,
    phase: u8,
    invalid: u8,
) -> Result<SwarmLayout> {
    if phase > 1 {
        return Err(ChartError("Swarm fixture phase must be 0..1".into()));
    }
    let config = ChartConfig {
        width,
        height,
        margin: Margin {
            left: 24.,
            right: 24.,
            top: 24.,
            bottom: 24.,
        },
        ..Default::default()
    };
    let mut data = vec![
        SwarmPoint::new("round", "A", 2. + phase as f64),
        SwarmPoint::new("offset", "A", 5. + phase as f64),
        SwarmPoint::new("zero", "A", 8.),
        SwarmPoint::new("hidden", "B", 3.),
        SwarmPoint::new("gradient", "C", 4.),
        SwarmPoint::new("dash", "C", 7.),
        SwarmPoint::new("culled", "C", 30.),
    ];
    data[2].size = Some(0.);
    if phase == 1 {
        data.push(SwarmPoint::new("added", "A", 9.));
        data.reverse();
    }
    let options = SwarmLayoutOptions {
        orientation: if vertical {
            SwarmOrientation::Vertical
        } else {
            SwarmOrientation::Horizontal
        },
        groups: Some(["A", "B", "C"].map(SwarmGroup::new).to_vec()),
        hidden: ["B".into()].into(),
        value_axis: None,
        group_axis: None,
        value_fit: SwarmValueFit::Clip,
        value_domain: Some([0., 10.]),
        value_format: ValueFormat::Fixed(1),
        node: SwarmNodeStyle {
            shape: SwarmNodeShape::Circle { diameter: 18. },
            ..Default::default()
        },
        ..Default::default()
    };
    let mut calls = 0;
    swarm_layout_with_node_factory(&config, &data, &options, |ctx| {
        calls += 1;
        if invalid == 1 && calls == 2 {
            return Err(ChartError("Swarm application factory rejection".into()));
        }
        let [x, y] = ctx.position;
        let mut node = ctx.inherit(ctx.geometry.clone());
        node.fill = Some(if phase == 0 {
            Color(32, 100, 190, 255)
        } else {
            Color(175, 40, 85, 255)
        });
        match ctx.datum.id.as_str() {
            "offset" => {
                node.geometry = Geometry::Path(vec![
                    C::Move(P::new(x + 24., y - 8.)),
                    C::Line(P::new(x + 40., y - 8.)),
                    C::Line(P::new(x + 40., y + 8.)),
                    C::Line(P::new(x + 24., y + 8.)),
                    C::Close,
                ])
            }
            "gradient" => {
                node.fill_paint = Some(FillPaint::Linear {
                    start: [0., 0.],
                    end: [640., 0.],
                    stops: vec![
                        GradientStop::new(0., Color(20, 100, 200, 255)),
                        GradientStop::new(1., Color(120, 200, 100, 255)),
                    ],
                })
            }
            "dash" => {
                node.fill = None;
                node.geometry = Geometry::Path(vec![
                    C::Move(P::new(x - 16., y)),
                    C::Line(P::new(x + 16., y)),
                ]);
                node.stroke = Some(GraphicStroke {
                    color: Color(30, 150, 80, 255),
                    width: 6.,
                    cap: StrokeCap::Butt,
                });
                node.stroke_dash = Some(StrokeDash {
                    segments: vec![8., 8.],
                    offset: 0.,
                });
            }
            _ => {}
        }
        if invalid == 2 && calls == 2 {
            node.opacity = f64::NAN;
        }
        Ok(node)
    })
}

pub fn ink_probe(site: &SwarmSite) -> [f64; 2] {
    let [x, y] = site.position;
    match site.node_id.as_str() {
        "offset" => [x + 32., y],
        "dash" => [x - 12., y],
        _ => [x, y],
    }
}
