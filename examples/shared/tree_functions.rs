//! Actual borrowed Tree functions with raw and computed source sidecars.
use std::cell::RefCell;

use kartis::*;
pub struct Source {
    pub layout: TreeLayout,
    pub log: Vec<String>,
}
pub fn config(width: f64) -> ChartConfig {
    ChartConfig {
        width,
        height: 320.,
        margin: Margin {
            left: 40.,
            right: 40.,
            top: 30.,
            bottom: 30.,
        },
        ..Default::default()
    }
}
pub fn source(phase: u8, width: f64, failure: u8) -> Result<Source> {
    let c = config(width);
    let mut raw = vec![
        HierarchyDatum::new("a", Some("r".into()), if phase == 1 { 24. } else { 12. }),
        HierarchyDatum::new("r", None, 0.),
        HierarchyDatum::new("b", Some("r".into()), if phase == 1 { 4. } else { -0. }),
    ];
    for d in &mut raw {
        d.label = format!("source-{}", d.id);
    }
    if phase == 2 {
        raw.reverse();
    }
    if phase == 14 {
        raw.clear();
    }
    if phase == 15 {
        raw.retain(|d| d.parent.is_none());
    }
    let mut o = TreeOptions {
        node_radius: 16.,
        orientation: match phase {
            8 => TreeOrientation::RightToLeft,
            9 => TreeOrientation::TopToBottom,
            10 => TreeOrientation::BottomToTop,
            11 => TreeOrientation::Radial,
            _ => TreeOrientation::LeftToRight,
        },
        ..Default::default()
    };
    o.style.nodes.shape = if phase == 12 {
        TreeNodeShape::Square
    } else {
        TreeNodeShape::Circle
    };
    o.style.nodes.fill = TreeNodeFill::None; // Explicit function color overrides this declaration.
    o.style.links.curve = TreeLinkCurve::Straight;
    o.style.links.enabled = phase != 3;
    o.style.links.color = TreeLinkColor::Source;
    o.style.labels.position = TreeLabelPosition::Below;
    o.style.labels.color = Some(Color(100, 20, 180, 255));
    o.style.labels.size = Some(18.);
    o.hierarchy.labels = phase != 6;
    if phase == 7 {
        o.style.labels.visibility = TreeLabels::None;
    }
    let log = RefCell::new(vec![]);
    let mut size = |ctx: &TreeNodeFunctionContext<'_>| {
        log.borrow_mut().push(format!(
            "size:{}:{}:{}:{}:{}:{}:{}",
            ctx.datum.id,
            ctx.source_index,
            ctx.depth,
            ctx.height,
            ctx.value.to_bits(),
            ctx.datum.value.to_bits(),
            ctx.source.len()
        ));
        if failure == 1 {
            return Err(ChartError("Tree size rejected".into()));
        }
        if failure == 6 {
            return Ok(f64::NAN);
        }
        Ok(if phase == 4 {
            0.
        } else {
            24. + 4. * ctx.depth as f64 + if phase == 1 { 8. } else { 0. }
        })
    };
    let mut color = |ctx: &TreeNodeFunctionContext<'_>| {
        log.borrow_mut().push(format!(
            "color:{}:{}:{}",
            ctx.datum.id,
            ctx.parent_index.is_some(),
            ctx.children.len()
        ));
        if failure == 2 {
            return Err(ChartError("Tree color rejected".into()));
        }
        let mut color = match ctx.datum.id.as_str() {
            "a" => Color(0, 100, 255, 255),
            "b" => Color(0, 200, 0, 255),
            _ => Color(160, 80, 30, 255),
        };
        if phase == 13 {
            color.3 = 0;
        }
        Ok(color)
    };
    let mut thickness = |ctx: &TreeLinkFunctionContext<'_>| {
        log.borrow_mut().push(format!(
            "thickness:{}:{}:{}:{}:{}",
            ctx.link.link_index,
            ctx.source_node.radius,
            ctx.target_node.radius,
            ctx.source_node.value.to_bits(),
            ctx.target_node.value.to_bits()
        ));
        if failure == 3 {
            return Err(ChartError("Tree thickness rejected".into()));
        }
        if failure == 7 {
            return Ok(-1.);
        }
        Ok(if phase == 5 {
            0.
        } else if phase == 1 {
            10.
        } else {
            6.
        })
    };
    let mut link_color = |ctx: &TreeLinkFunctionContext<'_>| {
        log.borrow_mut().push(format!(
            "link-color:{}:{}:{}",
            ctx.link.link_index,
            ctx.source[ctx.source_node.source_index].id,
            ctx.source[ctx.target_node.source_index].id
        ));
        if failure == 4 {
            return Err(ChartError("Tree link color rejected".into()));
        }
        Ok(Color(230, 110, 20, 255))
    };
    let mut label = |ctx: &TreeLabelFunctionContext<'_>| {
        log.borrow_mut().push(format!(
            "label:{}:{}:{}",
            ctx.datum.id,
            ctx.node.radius,
            ctx.node.value.to_bits()
        ));
        if failure == 5 {
            return Err(ChartError("Tree label rejected".into()));
        }
        if failure == 8 {
            return Ok("x".repeat(1025));
        }
        Ok(if phase == 16 && ctx.datum.id == "b" {
            String::new()
        } else {
            format!("function-{}", ctx.datum.id)
        })
    };
    let layout = tree_layout_with_factories(
        &c,
        &raw,
        &o,
        &mut TreeFactories {
            node_size: Some(&mut size),
            node_color: Some(&mut color),
            link_thickness: Some(&mut thickness),
            link_color: Some(&mut link_color),
            label: Some(&mut label),
        },
    )?;
    Ok(Source {
        layout,
        log: log.into_inner(),
    })
}
