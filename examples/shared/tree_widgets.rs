//! Complete flat Tree source and actual node/link component carriers.
use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::*;

pub struct Source {
    pub frame: SceneInteraction,
    pub composition: PlotWidgetsComposition<TreeWidgetData>,
}
pub fn config(width: f64) -> ChartConfig {
    let mut c = ChartConfig {
        width,
        height: 320.,
        margin: Margin {
            left: 40.,
            right: 40.,
            top: 30.,
            bottom: 30.,
        },
        title: "Tree nodes and links".into(),
        ..Default::default()
    };
    c.theme.colors = vec![
        Color(40, 120, 90, 255),
        Color(120, 90, 40, 255),
        Color(70, 100, 180, 255),
        Color(30, 120, 150, 255),
        Color(90, 50, 130, 255),
    ];
    c
}
pub fn data(phase: u8) -> Vec<HierarchyDatum> {
    let mut data = vec![
        HierarchyDatum::new("a", Some("r".into()), 0.),
        HierarchyDatum::new("c", Some("a".into()), 30.),
        HierarchyDatum::new("r", None, 0.),
        HierarchyDatum::new("b", Some("r".into()), if phase == 1 { 0. } else { -0. }),
        HierarchyDatum::new("d", Some("a".into()), 10.),
    ];
    for d in &mut data {
        d.label = format!("MMMM-{}", d.id);
    }
    if phase == 2 {
        data.reverse();
    }
    if phase == 19 {
        data.iter_mut().find(|d| d.id == "c").unwrap().value = 35.;
    }
    if phase == 20 {
        data.iter_mut().find(|d| d.id == "b").unwrap().label.clear();
    }
    if phase == 11 {
        data.clear();
    }
    if phase == 12 {
        data = vec![HierarchyDatum::new("r", None, 0.)];
        data[0].label = "MMMM-r".into();
    }
    data
}
pub fn source(phase: u8, width: f64, revision: u64, profile: u8, failure: u8) -> Result<Source> {
    let c = config(width);
    let raw = data(phase);
    let mut o = TreeOptions {
        orientation: match phase {
            4 => TreeOrientation::RightToLeft,
            5 => TreeOrientation::TopToBottom,
            6 => TreeOrientation::BottomToTop,
            7 => TreeOrientation::Radial,
            _ => TreeOrientation::LeftToRight,
        },
        mode: if phase == 8 {
            TreeMode::Cluster
        } else {
            TreeMode::Tidy
        },
        node_radius: if phase == 13 { 0. } else { 16. },
        ..Default::default()
    };
    o.hierarchy.include_root = phase == 17;
    o.style.nodes.shape = TreeNodeShape::Square;
    if phase == 14 {
        o.style.nodes.fill = TreeNodeFill::None;
    }
    if phase == 15 {
        o.style.nodes.opacity = 0.;
    }
    o.style.links.enabled = phase != 9;
    o.style.links.curve = if phase == 18 {
        TreeLinkCurve::Step
    } else {
        TreeLinkCurve::Straight
    };
    o.style.links.color = TreeLinkColor::Fixed(Color(230, 110, 20, 255));
    o.style.links.width = 4.;
    o.style.labels.size = Some(20.);
    o.style.labels.color = Some(Color(100, 20, 180, 255));
    o.style.labels.offset = 8.;
    if phase == 10 {
        o.style.labels.visibility = TreeLabels::None;
    }
    let mut layout = tree_layout(&c, &raw, &o)?;
    if phase == 3 {
        for mark in &mut layout.scene.marks {
            let center = if mark.id == "node:a" {
                layout
                    .nodes
                    .iter()
                    .find(|n| raw[n.source_index].id == "a")
                    .map(|n| n.position)
            } else if mark.id == "link:c" {
                layout
                    .links
                    .iter()
                    .find(|l| raw[l.target_index].id == "c")
                    .map(|l| l.position)
            } else {
                None
            };
            if let Some([x, y]) = center {
                mark.geometry = Geometry::Path(vec![
                    C::Move(P::new(x + 45., y - 8.)),
                    C::Line(P::new(x + 65., y - 8.)),
                    C::Line(P::new(x + 65., y + 8.)),
                    C::Line(P::new(x + 45., y + 8.)),
                    C::Close,
                ]);
            }
        }
    }
    if failure == 8 {
        let b = layout
            .nodes
            .iter_mut()
            .find(|n| raw[n.source_index].id == "b")
            .unwrap();
        b.raw_value = -0.;
    }
    if failure == 9 {
        layout.links[0].source_index = usize::MAX;
    }
    let widgets = tree_widgets(&layout, revision, |ctx| {
        let (id, offset) = match ctx {
            TreeWidgetContext::Node(ctx) => {
                let id = ctx.datum.id.as_str();
                if failure == 7 && id == "b" {
                    return Err(ChartError("Tree placement rejected".into()));
                }
                let offset = match id {
                    "a" if phase != 16 => [54., -11.],
                    "b" => [-16., -55.],
                    _ => return Ok(None),
                };
                (id, offset)
            }
            TreeWidgetContext::Link(ctx) => {
                let id = ctx.source[ctx.link.target_index].id.as_str();
                if id != "c" {
                    return Ok(None);
                }
                let offset = if phase == 21 {
                    [
                        ctx.link.target_position[0] - ctx.link.position[0] - 16.,
                        ctx.link.target_position[1] - ctx.link.position[1] - 11.,
                    ]
                } else if phase == 7 {
                    [-16., 39.]
                } else {
                    [-16., 5.]
                };
                (id, offset)
            }
        };
        let _ = id;
        Ok(Some(PlotWidgetPlacement::AtDatum {
            offset,
            measured: TextSize {
                width: 32.,
                height: 22.,
            },
        }))
    })?;
    let replace_ink = if profile.is_multiple_of(2) {
        widgets
            .layout
            .slots
            .iter()
            .map(|s| s.mark_id.clone())
            .collect()
    } else {
        Default::default()
    };
    Ok(Source {
        frame: tree_widget_frame(&layout)?,
        composition: PlotWidgetsComposition {
            widgets,
            replace_ink,
        },
    })
}
pub fn color(data: &TreeWidgetData) -> Color {
    let source = match data {
        TreeWidgetData::Node(d) => &d.source,
        TreeWidgetData::Link(d) => &d.source,
    };
    if source.iter().any(|d| d.id == "b" && d.value.to_bits() == 0) {
        Color(0, 200, 0, 255)
    } else {
        Color(0, 100, 255, 255)
    }
}
