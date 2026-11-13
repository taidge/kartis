use super::{Coordinates, SwarmGroupLayout};
use crate::{
    CategoryAxisOptions, ChartConfig, Color, Geometry, Label, Mark, Result, Scene, TextAnchor,
    scene::ensure,
};
use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use std::collections::BTreeSet;

fn line(scene: &mut Scene, id: impl Into<String>, color: Color, from: [f64; 2], to: [f64; 2]) {
    let mut mark = Mark::new(
        id,
        Geometry::Path(vec![
            C::Move(P::new(from[0], from[1])),
            C::Line(P::new(to[0], to[1])),
        ]),
        color,
    );
    mark.fill = None;
    mark.label.clear();
    mark.stroke = Some(GraphicStroke {
        color,
        width: 1.,
        cap: StrokeCap::Butt,
    });
    scene.marks.push(mark);
}

pub(super) fn draw(
    scene: &mut Scene,
    c: &ChartConfig,
    coords: Coordinates,
    groups: &[SwarmGroupLayout],
    options: &CategoryAxisOptions,
) -> Result<()> {
    let explicit = options
        .tick_values
        .as_ref()
        .map(|keys| keys.iter().map(String::as_str).collect::<BTreeSet<_>>());
    if let Some(keys) = &explicit {
        let known: BTreeSet<_> = groups.iter().map(|group| group.id.as_str()).collect();
        ensure(keys.is_subset(&known), "unknown swarm group axis tick")?;
    }
    let stride = groups.len().div_ceil(options.tick_count.max(1)).max(1);
    let p = if coords.horizontal {
        coords.primary[0]
    } else {
        coords.primary[1]
    };
    let direction = if coords.horizontal { -1. } else { 1. };
    for (i, group) in groups.iter().enumerate() {
        if !explicit
            .as_ref()
            .map_or(options.tick_count > 0 && i % stride == 0, |keys| {
                keys.contains(group.id.as_str())
            })
        {
            continue;
        }
        let coord = |primary| coords.position(primary, group.center);
        if options.grid {
            line(
                scene,
                format!("swarm-group-grid:{}:{}", group.id.len(), group.id),
                c.theme.grid,
                coord(coords.primary[0]),
                coord(coords.primary[1]),
            );
        }
        if options.visible {
            line(
                scene,
                format!("swarm-group-tick:{}:{}", group.id.len(), group.id),
                c.theme.text,
                coord(p),
                coord(p + direction * 5.),
            );
            let [x, y] = coord(p + direction * (5. + options.tick_padding));
            scene.labels.push(Label {
                x,
                y,
                text: group.label.clone(),
                color: c.theme.text,
                size: c.theme.font_size,
                anchor: if coords.horizontal {
                    TextAnchor::End
                } else {
                    TextAnchor::Middle
                },
                rotation: options.tick_rotation,
            });
        }
    }
    if options.visible && !groups.is_empty() {
        line(
            scene,
            "swarm-group-axis",
            c.theme.text,
            coords.position(p, coords.cross[0]),
            coords.position(p, coords.cross[1]),
        );
        if let Some(text) = &options.legend {
            let [x, y] = coords.position(
                p + direction * (24. + options.tick_padding),
                (coords.cross[0] + coords.cross[1]) / 2.,
            );
            scene.labels.push(Label {
                x,
                y,
                text: text.clone(),
                color: c.theme.text,
                size: c.theme.font_size,
                anchor: TextAnchor::Middle,
                rotation: if coords.horizontal { -90. } else { 0. },
            });
        }
    }
    Ok(())
}
