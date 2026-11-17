//! Current force states and visual glyphs for live Network Widgets.
use kartis::*;
pub fn config(width: f64, height: f64) -> ChartConfig {
    ChartConfig {
        width,
        height,
        margin: Margin {
            left: 40.,
            right: 40.,
            top: 30.,
            bottom: 30.,
        },
        title: "Network live node Widgets".into(),
        ..Default::default()
    }
}
pub fn source(phase: u8) -> Result<(ForceSimulation, NetworkGlyphOptions)> {
    let mut nodes = vec![
        NetworkNode::new("a"),
        NetworkNode::new("b"),
        NetworkNode::new("c"),
        NetworkNode::new("z"),
    ];
    nodes[0].label = if phase == 1 { "Alpha updated" } else { "Alpha" }.into();
    nodes[0].radius = 8.;
    nodes[1].label = "Beta".into();
    nodes[1].radius = 12.;
    nodes[2].label = "Drag".into();
    nodes[2].radius = 10.;
    nodes[3].label.clear();
    nodes[3].radius = 6.;
    if phase == 2 {
        nodes.reverse();
    }
    if phase == 3 {
        nodes.retain(|n| n.id != "b");
    }
    if phase == 4 {
        nodes.clear();
    }
    let links = if nodes.len() > 1 {
        vec![NetworkLink::new("a", "c", 4.)]
    } else {
        vec![]
    };
    let mut sim = ForceSimulation::new(
        &nodes,
        &links,
        ForceOptions {
            repulsion: 0.,
            link_strength: 0.,
            center_strength: 0.,
            collision_strength: 0.,
            collision_iterations: 0,
            velocity_decay: 0.5,
            alpha_decay: 0.2,
            labels: false,
            ..Default::default()
        },
    )?;
    for (id, p, v) in [
        (
            "a",
            [-100., if phase == 1 { -15. } else { -25. }],
            [2., -0.],
        ),
        ("b", [20., 35.], [-0., 1.]),
        ("c", [115., -35.], [0., 0.]),
        ("z", [-15., -75.], [0., 0.]),
    ] {
        if sim.node(id).is_some() {
            sim.set_position(id, p)?;
            sim.set_velocity(id, v)?;
            if phase == 5 {
                sim.set_fixed(id, Some(p))?;
            }
        }
    }
    let mut glyphs = NetworkGlyphOptions::default();
    if sim.node("b").is_some() {
        glyphs.nodes.insert(
            "b".into(),
            NetworkNodeGlyph {
                glyph: GlyphOptions {
                    shape: GlyphShape::Filled(FilledGlyph::Triangle),
                    size: 180.,
                    rotation: 0.2,
                },
                fill: Some(Color(230, 150, 40, 255)),
                stroke: None,
            },
        );
    }
    if sim.node("c").is_some() {
        glyphs.nodes.insert(
            "c".into(),
            NetworkNodeGlyph {
                glyph: GlyphOptions {
                    shape: GlyphShape::Filled(FilledGlyph::Circle),
                    size: std::f64::consts::PI * 100.,
                    rotation: 0.,
                },
                fill: Some(Color(180, 70, 90, 255)),
                stroke: None,
            },
        );
    }
    if sim.node("z").is_some() {
        glyphs.nodes.insert(
            "z".into(),
            NetworkNodeGlyph {
                glyph: GlyphOptions {
                    shape: GlyphShape::Filled(FilledGlyph::Diamond),
                    size: 0.,
                    rotation: 0.,
                },
                fill: Some(Color(0, 0, 0, 0)),
                stroke: None,
            },
        );
    }
    Ok((sim, glyphs))
}
pub fn placement(ctx: &NetworkNodeWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>> {
    Ok(
        (ctx.node.id != "c").then_some(PlotWidgetPlacement::AtDatum {
            offset: [-16., -11.],
            measured: TextSize {
                width: 32.,
                height: 22.,
            },
        }),
    )
}
