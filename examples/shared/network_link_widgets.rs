//! Explicit parallel, self and transparent link-component profiles.
use glory::render::{GraphicPoint, PathCommand};
use kartis::*;

use crate::network_widget_model as nodes;
pub fn config(width: f64, height: f64) -> ChartConfig {
    nodes::config(width, height)
}
pub fn source(phase: u8) -> Result<(ForceSimulation, NetworkGlyphOptions)> {
    let (base, mut glyphs) = nodes::source(phase)?;
    let raw: Vec<_> = base
        .nodes()
        .iter()
        .map(|n| NetworkNode {
            id: n.id.clone(),
            label: n.label.clone(),
            radius: n.radius,
        })
        .collect();
    let mut edges = vec![
        NetworkLink::new("a", "c", if phase == 1 { 9. } else { 4. }),
        NetworkLink::new("a", "c", if phase == 1 { 0. } else { -0. }),
        NetworkLink::new("b", "b", 1.),
        NetworkLink::new("c", "a", 9.),
    ];
    edges.retain(|e| raw.iter().any(|n| n.id == e.source) && raw.iter().any(|n| n.id == e.target));
    if phase == 2 {
        edges.reverse();
    }
    let mut simulation = ForceSimulation::new(&raw, &edges, base.options().clone())?;
    for n in base.nodes() {
        simulation.set_position(&n.id, n.position)?;
        simulation.set_velocity(&n.id, n.velocity)?;
        simulation.set_fixed(&n.id, n.fixed)?;
    }
    for (id, shape, size, fill) in [
        (
            "a",
            FilledGlyph::Circle,
            std::f64::consts::PI * 64.,
            Color(40, 120, 90, 255),
        ),
        ("b", FilledGlyph::Square, 196., Color(230, 150, 40, 255)),
    ] {
        if simulation.node(id).is_some() {
            glyphs.nodes.insert(
                id.into(),
                NetworkNodeGlyph {
                    glyph: GlyphOptions {
                        shape: GlyphShape::Filled(shape),
                        size,
                        rotation: 0.,
                    },
                    fill: Some(fill),
                    stroke: None,
                },
            );
        }
    }
    Ok((simulation, glyphs))
}
pub fn placement(ctx: &NetworkWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>> {
    let offset = match ctx {
        NetworkWidgetContext::Node(n) if n.node.id == "a" => [-16., -11.],
        NetworkWidgetContext::Link(l) if l.link_index == 0 || l.link_index == 2 => [-16., -11.],
        NetworkWidgetContext::Link(l) if l.link_index == 3 => [-16., 30.],
        _ => return Ok(None),
    };
    Ok(Some(PlotWidgetPlacement::AtDatum {
        offset,
        measured: TextSize {
            width: 32.,
            height: 22.,
        },
    }))
}
pub fn ink(ctx: &NetworkLinkContext<'_>) -> Result<NetworkLinkGeometry> {
    let mut ink = ctx.inherit(ctx.geometry.clone());
    if let Some(stroke) = &mut ink.stroke {
        stroke.width = 6.;
    }
    if ctx.link_index == 0 {
        let [x, y] = [ctx.position[0] + 40., ctx.position[1] + 35.];
        ink.geometry = Geometry::Path(vec![
            PathCommand::Move(GraphicPoint::new(x, y)),
            PathCommand::Line(GraphicPoint::new(x + 18., y)),
            PathCommand::Line(GraphicPoint::new(x + 18., y + 16.)),
            PathCommand::Line(GraphicPoint::new(x, y + 16.)),
            PathCommand::Close,
        ]);
        ink.fill = Some(Color(130, 20, 180, 255));
        ink.stroke = None;
    }
    if ctx.link_index == 1 {
        ink.geometry = Geometry::Rect {
            x: ctx.position[0] - 6.,
            y: ctx.position[1] - 8.,
            width: 12.,
            height: 16.,
        };
        ink.fill = Some(Color(220, 30, 40, 255));
        ink.stroke = None;
    }
    if ctx.link_index == 3 {
        let p = GraphicPoint::new(ctx.position[0] + 50., ctx.position[1] + 20.);
        ink.geometry = Geometry::Path(vec![
            PathCommand::Move(p),
            PathCommand::Line(p),
            PathCommand::Close,
        ]);
        ink.fill = Some(Color(0, 0, 0, 0));
        ink.stroke = None;
    }
    Ok(ink)
}
