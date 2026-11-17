use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use kartis::*;

pub fn config(width: f64, height: f64) -> ChartConfig {
    ChartConfig {
        width,
        height,
        margin: Margin {
            left: 40.,
            right: 30.,
            top: 30.,
            bottom: 30.,
        },
        ..Default::default()
    }
}
pub fn source(phase: u8) -> Result<(ForceSimulation, NetworkGlyphOptions)> {
    let mut nodes = [
        NetworkNode::new("a:wing"),
        NetworkNode::new("b"),
        NetworkNode::new("c"),
    ];
    for node in &mut nodes {
        node.label = format!(
            "{} {}",
            node.id,
            if phase == 0 { "original" } else { "updated" }
        );
    }
    let mut s = ForceSimulation::new(
        &nodes,
        &[
            NetworkLink::new("a:wing", "b", if phase == 0 { 9. } else { 16. }),
            NetworkLink::new("a:wing", "b", 0.),
            NetworkLink::new("b", "c", 4.),
            NetworkLink::new("c", "c", 1.),
        ],
        ForceOptions {
            labels: false,
            alpha: 0.,
            repulsion: 0.,
            link_strength: 0.,
            center_strength: 0.,
            collision_strength: 0.,
            alpha_decay: 1.,
            ..Default::default()
        },
    )?;
    s.set_position("a:wing", [-120., 0.])?;
    s.set_position("b", [30., -25.])?;
    s.set_position("c", [115., 65.])?;
    if phase != 0 {
        s.set_position("a:wing", [-105., 8.])?;
        s.set_position("b", [55., -12.])?;
    }
    s.set_velocity("b", [3., -2.])?;
    let mut glyphs = NetworkGlyphOptions::default();
    glyphs.nodes.insert(
        "a:wing".into(),
        NetworkNodeGlyph {
            glyph: GlyphOptions {
                shape: GlyphShape::Filled(FilledGlyph::Star),
                size: 900.,
                rotation: 0.,
            },
            fill: Some(Color(40, 103, 190, 255)),
            stroke: None,
        },
    );
    Ok((s, glyphs))
}
pub fn ink(ctx: &NetworkLinkContext<'_>) -> Result<NetworkLinkGeometry> {
    let phase = ctx.source.label.ends_with("updated");
    let [x0, y0] = ctx.source_position;
    let [x1, y1] = ctx.target_position;
    let bend = if ctx.link_index.is_multiple_of(2) {
        -64.
    } else {
        56.
    };
    let geometry = if ctx.edge.source == ctx.edge.target {
        Geometry::Path(vec![
            C::Move(P::new(x0, y0)),
            C::Cubic {
                first: P::new(x0 - 48., y0 - 72.),
                second: P::new(x0 + 48., y0 - 72.),
                end: P::new(x0, y0),
            },
        ])
    } else {
        Geometry::Path(vec![
            C::Move(P::new(x0, y0)),
            C::Cubic {
                first: P::new(x0, y0 + bend),
                second: P::new(x1, y1 + bend),
                end: P::new(x1, y1),
            },
        ])
    };
    let mut g = ctx.inherit(geometry);
    g.stroke = Some(GraphicStroke {
        color: if phase {
            Color(20, 151, 164, 255)
        } else {
            Color(210, 37, 96, 255)
        },
        width: if phase { 8. } else { 6. },
        cap: StrokeCap::Round,
    });
    Ok(g)
}
