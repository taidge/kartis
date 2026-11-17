//! Complete source and callback traces for Tree current-node presentation.
use std::cell::RefCell;

use kartis::*;
pub struct Source {
    pub layout: TreeActivityLayout,
    pub log: Vec<String>,
}
pub fn config(width: f64) -> ChartConfig {
    ChartConfig {
        width,
        height: 340.,
        margin: Margin {
            left: 40.,
            right: 40.,
            top: 40.,
            bottom: 40.,
        },
        ..Default::default()
    }
}
pub fn data(phase: u8) -> Vec<HierarchyDatum> {
    let mut raw = vec![
        HierarchyDatum::new("a", Some("r".into()), 0.),
        HierarchyDatum::new("c", Some("a".into()), if phase == 1 { 35. } else { 30. }),
        HierarchyDatum::new("r", None, 0.),
        HierarchyDatum::new(
            "b",
            Some("r".into()),
            if phase == 1 {
                5.
            } else if phase == 9 {
                0.
            } else {
                -0.
            },
        ),
        HierarchyDatum::new("d", Some("a".into()), 10.),
    ];
    for d in &mut raw {
        d.label = format!("source-{}", d.id);
    }
    if phase == 2 {
        raw.reverse();
    }
    if phase == 8 {
        raw.clear();
    }
    raw
}
pub fn tree(phase: u8) -> TreeOptions {
    let mut options = TreeOptions {
        node_radius: 24.,
        ..Default::default()
    };
    options.style.links.curve = TreeLinkCurve::Straight;
    options.style.links.enabled = phase != 3;
    options.hierarchy.labels = phase != 4;
    options.style.labels.position = TreeLabelPosition::Below;
    options.style.labels.size = Some(16.);
    options.style.labels.color = Some(Color(100, 20, 180, 255));
    options
}
pub fn source(
    phase: u8,
    config: &ChartConfig,
    raw: &[HierarchyDatum],
    tree: &TreeOptions,
    activity: &TreeActivityOptions,
    failure: u8,
) -> Result<Source> {
    let log = RefCell::new(vec![]);
    let mut size = |ctx: &TreeNodeFunctionContext<'_>| {
        log.borrow_mut().push(format!(
            "size:{}:{}:{}:{}:{}",
            ctx.datum.id,
            ctx.source_index,
            ctx.depth,
            ctx.height,
            ctx.datum.value.to_bits()
        ));
        if failure == 1 {
            return Err(ChartError("Tree base size rejected".into()));
        }
        Ok(if phase == 5 {
            0.
        } else {
            24. + 4. * ctx.depth as f64
        })
    };
    let mut color = |ctx: &TreeNodeFunctionContext<'_>| {
        log.borrow_mut().push(format!("color:{}", ctx.datum.id));
        if failure == 2 {
            return Err(ChartError("Tree base color rejected".into()));
        }
        Ok(match ctx.datum.id.as_str() {
            "a" => Color(0, 100, 255, 255),
            "b" => Color(0, 200, 0, 255),
            "c" => Color(230, 110, 20, 255),
            "d" => Color(100, 20, 180, 255),
            _ => Color(160, 80, 30, 255),
        })
    };
    let mut thickness = |ctx: &TreeLinkFunctionContext<'_>| {
        log.borrow_mut().push(format!(
            "thickness:{}:{}:{}:{:?}:{:?}",
            ctx.link.link_index,
            ctx.source_node.radius,
            ctx.target_node.radius,
            ctx.activity.and_then(|a| a.nodes[ctx.link.source_index]),
            ctx.activity.and_then(|a| a.nodes[ctx.link.target_index])
        ));
        if failure == 3 {
            return Err(ChartError("Tree base thickness rejected".into()));
        }
        Ok(if phase == 5 {
            0.
        } else {
            6. + 2. * ctx.target_node.depth as f64
        })
    };
    let mut link_color = |ctx: &TreeLinkFunctionContext<'_>| {
        log.borrow_mut()
            .push(format!("link-color:{}", ctx.link.link_index));
        if failure == 4 {
            return Err(ChartError("Tree base link color rejected".into()));
        }
        Ok(Color(230, 110, 20, 255))
    };
    let mut label = |ctx: &TreeLabelFunctionContext<'_>| {
        let active = ctx.activity.and_then(|a| a.nodes[ctx.source_index]);
        log.borrow_mut().push(format!(
            "label:{}:{}:{active:?}",
            ctx.datum.id, ctx.node.radius
        ));
        if failure == 5 {
            return Err(ChartError("Tree activity label rejected".into()));
        }
        Ok(format!("{}:{active:?}", ctx.datum.id))
    };
    let mut active_size = |ctx: &TreeNodeModifierContext<'_>| {
        log.borrow_mut().push(format!(
            "active-size:{}:{}:{}",
            ctx.datum.id, ctx.size, ctx.is_active
        ));
        if failure == 6 {
            return Err(ChartError("Tree active size rejected".into()));
        }
        if failure == 10 {
            return Ok(f64::NAN);
        }
        Ok(if phase == 6 { 0. } else { ctx.size + 12. })
    };
    let mut inactive_size = |ctx: &TreeNodeModifierContext<'_>| {
        log.borrow_mut().push(format!(
            "inactive-size:{}:{}:{}",
            ctx.datum.id, ctx.size, ctx.is_active
        ));
        if failure == 7 {
            return Err(ChartError("Tree inactive size rejected".into()));
        }
        Ok(if phase == 6 { 0. } else { 8. })
    };
    let mut active_width = |ctx: &TreeLinkModifierContext<'_>| {
        log.borrow_mut().push(format!(
            "active-width:{}:{}:{}",
            ctx.link.link_index, ctx.thickness, ctx.is_active
        ));
        if failure == 8 {
            return Err(ChartError("Tree active width rejected".into()));
        }
        if failure == 11 {
            return Ok(-1.);
        }
        Ok(if phase == 7 { 0. } else { ctx.thickness + 6. })
    };
    let mut inactive_width = |ctx: &TreeLinkModifierContext<'_>| {
        log.borrow_mut().push(format!(
            "inactive-width:{}:{}:{}",
            ctx.link.link_index, ctx.thickness, ctx.is_active
        ));
        if failure == 9 {
            return Err(ChartError("Tree inactive width rejected".into()));
        }
        Ok(if phase == 7 { 0. } else { 4. })
    };
    let layout = tree_layout_with_activity(
        config,
        raw,
        tree,
        &mut TreeFactories {
            node_size: Some(&mut size),
            node_color: Some(&mut color),
            link_thickness: Some(&mut thickness),
            link_color: Some(&mut link_color),
            label: Some(&mut label),
        },
        activity,
        &mut TreeActivityFactories {
            active_node_size: Some(&mut active_size),
            inactive_node_size: Some(&mut inactive_size),
            active_link_thickness: Some(&mut active_width),
            inactive_link_thickness: Some(&mut inactive_width),
        },
    )?;
    Ok(Source {
        layout,
        log: log.into_inner(),
    })
}
