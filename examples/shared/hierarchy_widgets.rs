//! Shared accepted hierarchy frames for live node components.
use kartis::*;
#[derive(Clone, Debug)]
pub struct Source {
    pub frame: SceneInteraction,
    pub composition: PlotWidgetsComposition<HierarchyNodeWidgetData>,
}
pub fn source(kind: u8, phase: u8, width: f64, revision: u64) -> Result<Source> {
    let c = ChartConfig {
        width,
        height: 260.,
        margin: Margin {
            left: 40.,
            right: 40.,
            top: 30.,
            bottom: 30.,
        },
        title: "Hierarchy live node Widgets".into(),
        ..Default::default()
    };
    let d = |id: &str, parent: Option<&str>, value: f64| HierarchyDatum {
        id: id.into(),
        label: format!("Original {id}"),
        parent: parent.map(String::from),
        value,
    };
    let data = vec![
        d("root", None, 0.),
        d("north", Some("root"), 0.),
        d("south", Some("root"), 0.),
        d("a", Some("north"), if phase == 1 { 3.5 } else { 2. }),
        d(
            "b",
            Some("north"),
            if phase == 1 {
                3.
            } else if phase == 8 {
                0.
            } else {
                -0.
            },
        ),
        d("c", Some("north"), 1.),
        d("d", Some("south"), 3.),
        d("e", Some("south"), 4.),
    ];
    let mut style = HierarchyStyleOptions {
        leaves_only: true,
        include_focus: true,
        leaf_labels: HierarchyLabelStyle {
            enabled: false,
            ..Default::default()
        },
        parent_labels: HierarchyLabelStyle {
            enabled: false,
            ..Default::default()
        },
        value_format: ValueFormat::Fixed(1),
        ..Default::default()
    };
    for (id, color) in [
        ("a", Color(0, 100, 255, 255)),
        ("b", Color(0, 200, 0, 255)),
        ("c", Color(255, 220, 0, 255)),
        ("d", Color(255, 180, 0, 255)),
        ("e", Color(150, 80, 220, 255)),
    ] {
        style.nodes.insert(
            id.into(),
            HierarchyNodeStyle {
                fill: HierarchyFill::Fixed(color),
                opacity: if phase == 5 && id == "c" { 0. } else { 1. },
                ..Default::default()
            },
        );
    }
    if phase == 2 {
        style.hidden.insert("d".into());
    }
    let focus = match phase {
        3 => Some("north"),
        4 => Some("a"),
        _ => None,
    };
    let layout = match kind {
        0 => treemap_layout(
            &c,
            &data,
            focus,
            &TreemapLayoutOptions {
                tile: HierarchyTreemapTile::Static(if phase == 7 {
                    TreemapTile::Slice
                } else {
                    TreemapTile::Squarify
                }),
                parent_label_size: 0.,
                inner_padding: 4.,
                outer_padding: 4.,
                style,
                ..Default::default()
            },
        )?,
        1 => circle_packing_layout(
            &c,
            &data,
            focus,
            &CirclePackingLayoutOptions {
                padding: if phase == 7 { 6. } else { 2. },
                style,
                ..Default::default()
            },
        )?,
        2 => icicle_layout(
            &c,
            &data,
            focus,
            &IcicleLayoutOptions {
                direction: if phase == 7 {
                    IcicleDirection::Right
                } else {
                    IcicleDirection::Down
                },
                padding: 4.,
                style,
            },
        )?,
        _ => return Err(ChartError("unknown hierarchy fixture kind".into())),
    };
    let widgets = hierarchy_node_widgets(&layout, revision, |ctx| {
        if phase == 6 && ctx.node.id == "c" {
            return Ok(None);
        }
        Ok(Some(PlotWidgetPlacement::AtDatum {
            offset: [-16., -11.],
            measured: TextSize {
                width: 32.,
                height: 22.,
            },
        }))
    })?;
    let replace_ink = widgets
        .layout
        .slots
        .iter()
        .map(|s| s.mark_id.clone())
        .collect();
    Ok(Source {
        frame: hierarchy_widget_frame(&layout)?,
        composition: PlotWidgetsComposition {
            widgets,
            replace_ink,
        },
    })
}
