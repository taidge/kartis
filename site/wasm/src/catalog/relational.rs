use glory::render::{GraphicStroke, StrokeCap};
use serde_json::json;

use super::rel_controls::active;
use super::{phase, *};

fn hierarchy(seed: u32) -> Vec<HierarchyDatum> {
    let mut data = vec![HierarchyDatum::new("Portfolio", None, 0.)];
    for (j, group) in ["Cloud", "Devices", "Services"].into_iter().enumerate() {
        data.push(HierarchyDatum::new(group, Some("Portfolio".into()), 0.));
        for (i, name) in ["Core", "Growth", "Research"].into_iter().enumerate() {
            data.push(HierarchyDatum::new(
                format!("{group} {name}"),
                Some(group.into()),
                20. + ((j * 3 + i) as f64 * 1.3 + phase(seed)).sin() * 12.,
            ));
        }
    }
    data
}
pub(super) fn sample(
    kind: &str,
    c: &ChartConfig,
    seed: u32,
    variant: &str,
    settings: &Settings,
) -> Result<Sample> {
    let mut source = hierarchy(seed);
    let options = HierarchyOptions {
        padding: if active(settings, "tile-padding") {
            1.
        } else {
            4.
        },
        labels: !active(settings, "node-labels"),
        ..Default::default()
    };
    let (mut scene, data, code) = match kind {
        "tree" | "treemap" | "sunburst" | "circle-packing" | "icicle" => {
            if kind == "tree" {
                // Weight does not change tree spacing; authored sibling order does.
                let rotate = seed as usize % 3;
                for children in [2..5, 6..9, 10..13] {
                    source[children].rotate_left(rotate);
                }
            }
            let data = json!(
                source
                    .iter()
                    .map(|d| json!({"id":d.id,"parent":d.parent,"label":d.label,"value":d.value}))
                    .collect::<Vec<_>>()
            );
            match kind {
                "tree" => (
                    tree_with_options(
                        c,
                        &source,
                        &TreeOptions {
                            hierarchy: options,
                            orientation: match variant {
                                "down" => TreeOrientation::TopToBottom,
                                "left" => TreeOrientation::RightToLeft,
                                "up" => TreeOrientation::BottomToTop,
                                "radial" => TreeOrientation::Radial,
                                _ => TreeOrientation::LeftToRight,
                            },
                            node_radius: if active(settings, "node-size") {
                                9.
                            } else {
                                5.
                            },
                            ..Default::default()
                        },
                    )?,
                    data,
                    match variant {
                        "left" => {
                            "tree_with_options(&config, &hierarchy, &TreeOptions { orientation: TreeOrientation::RightToLeft, node_radius: 5., ..Default::default() })?"
                        }
                        "up" => {
                            "tree_with_options(&config, &hierarchy, &TreeOptions { orientation: TreeOrientation::BottomToTop, node_radius: 5., ..Default::default() })?"
                        }
                        "down" => {
                            "tree_with_options(&config, &hierarchy, &TreeOptions { orientation: TreeOrientation::TopToBottom, node_radius: 5., ..Default::default() })?"
                        }
                        "radial" => {
                            "tree_with_options(&config, &hierarchy, &TreeOptions { orientation: TreeOrientation::Radial, node_radius: 5., ..Default::default() })?"
                        }
                        _ => {
                            "tree_with_options(&config, &hierarchy, &TreeOptions { orientation: TreeOrientation::LeftToRight, node_radius: 5., ..Default::default() })?"
                        }
                    },
                ),
                "treemap" => {
                    let (tile, code) = match variant {
                        "squarify" => (
                            TreemapTile::Squarify,
                            "treemap_with_tile(&config, &hierarchy, &options, TreemapTile::Squarify)?",
                        ),
                        "slice" => (
                            TreemapTile::Slice,
                            "treemap_with_tile(&config, &hierarchy, &options, TreemapTile::Slice)?",
                        ),
                        "dice" => (
                            TreemapTile::Dice,
                            "treemap_with_tile(&config, &hierarchy, &options, TreemapTile::Dice)?",
                        ),
                        "slice-dice" => (
                            TreemapTile::SliceDice,
                            "treemap_with_tile(&config, &hierarchy, &options, TreemapTile::SliceDice)?",
                        ),
                        _ => (
                            TreemapTile::Binary,
                            "treemap_with_tile(&config, &hierarchy, &options, TreemapTile::Binary)?",
                        ),
                    };
                    (treemap_with_tile(c, &source, &options, tile)?, data, code)
                }
                "sunburst" => {
                    let (options, code) = match variant {
                        "root" => (
                            HierarchyOptions {
                                include_root: true,
                                ..options
                            },
                            "sunburst(&config, &hierarchy, &HierarchyOptions { include_root: true, padding: 4., ..Default::default() })?",
                        ),
                        "flush" => (
                            HierarchyOptions {
                                padding: 0.,
                                ..options
                            },
                            "sunburst(&config, &hierarchy, &HierarchyOptions { padding: 0., ..Default::default() })?",
                        ),
                        _ => (
                            options,
                            "sunburst(&config, &hierarchy, &HierarchyOptions { padding: 4., ..Default::default() })?",
                        ),
                    };
                    (sunburst(c, &source, &options)?, data, code)
                }
                "circle-packing" => {
                    let (algorithm, padding, code) = match variant {
                        "shelf" => (
                            CirclePackingAlgorithm::Shelf,
                            4.,
                            "circle_packing_with_algorithm(&config, &hierarchy, &HierarchyOptions { padding: 4., ..Default::default() }, CirclePackingAlgorithm::Shelf)?",
                        ),
                        "touching" => (
                            CirclePackingAlgorithm::FrontChain,
                            0.,
                            "circle_packing_with_algorithm(&config, &hierarchy, &HierarchyOptions { padding: 0., ..Default::default() }, CirclePackingAlgorithm::FrontChain)?",
                        ),
                        _ => (
                            CirclePackingAlgorithm::FrontChain,
                            4.,
                            "circle_packing_with_algorithm(&config, &hierarchy, &HierarchyOptions { padding: 4., ..Default::default() }, CirclePackingAlgorithm::FrontChain)?",
                        ),
                    };
                    (
                        circle_packing_with_algorithm(
                            c,
                            &source,
                            &HierarchyOptions {
                                padding,
                                include_root: active(settings, "root-circle"),
                                ..options
                            },
                            algorithm,
                        )?,
                        data,
                        code,
                    )
                }
                _ => {
                    let (options, code) = match variant {
                        "root" => (
                            HierarchyOptions {
                                include_root: true,
                                ..options
                            },
                            "icicle(&config, &hierarchy, &HierarchyOptions { include_root: true, padding: 4., ..Default::default() })?",
                        ),
                        "flush" => (
                            HierarchyOptions {
                                padding: 0.,
                                ..options
                            },
                            "icicle(&config, &hierarchy, &HierarchyOptions { padding: 0., ..Default::default() })?",
                        ),
                        _ => (
                            options,
                            "icicle(&config, &hierarchy, &HierarchyOptions { padding: 4., ..Default::default() })?",
                        ),
                    };
                    (icicle(c, &source, &options)?, data, code)
                }
            }
        }
        "chord" => {
            let p = phase(seed);
            let ids = ["Cloud", "Devices", "Services", "Support"].map(str::to_owned);
            let matrix = (0..4)
                .map(|i| {
                    (0..4)
                        .map(|j| 8. + (i as f64 * 1.5 + j as f64 * 2.1 + p).sin() * 6.)
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            let data = json!({"ids":ids,"matrix":matrix});
            let (pad_angle, code) = match variant {
                "joined" => (0., "chord(&config, &ids, &matrix, 0.)?"),
                "wide" => (0.24, "chord(&config, &ids, &matrix, 0.24)?"),
                _ => (0.06, "chord(&config, &ids, &matrix, 0.06)?"),
            };
            (chord(c, &ids, &matrix, pad_angle)?, data, code)
        }
        "sankey" => {
            let value = 56. + phase(seed).sin() * 16.;
            let nodes = ["Source", "Process A", "Process B", "Output"]
                .into_iter()
                .map(FlowNode::new)
                .collect::<Vec<_>>();
            let links = [
                ("Source", "Process A", value),
                ("Source", "Process B", 100. - value),
                ("Process A", "Output", value),
                ("Process B", "Output", 100. - value),
            ]
            .into_iter()
            .map(|(source, target, value)| FlowLink {
                source: source.into(),
                target: target.into(),
                value,
            })
            .collect::<Vec<_>>();
            let data = json!({"nodes":nodes.iter().map(|n|json!({"id":n.id})).collect::<Vec<_>>(),"links":links.iter().map(|l|json!({"source":l.source,"target":l.target,"value":l.value})).collect::<Vec<_>>()});
            let (node_width, code) = match variant {
                "thin" => (
                    8.,
                    "sankey(&config, &nodes, &links, &SankeyOptions { node_width: 8., node_padding: 18., ..Default::default() })?",
                ),
                "wide" => (
                    32.,
                    "sankey(&config, &nodes, &links, &SankeyOptions { node_width: 32., node_padding: 18., ..Default::default() })?",
                ),
                _ => (
                    18.,
                    "sankey(&config, &nodes, &links, &SankeyOptions { node_width: 18., node_padding: 18., ..Default::default() })?",
                ),
            };
            (
                sankey(
                    c,
                    &nodes,
                    &links,
                    &SankeyOptions {
                        node_width,
                        node_padding: if active(settings, "node-padding") {
                            5.
                        } else {
                            18.
                        },
                        link_opacity: if active(settings, "link-opacity") {
                            0.8
                        } else {
                            0.45
                        },
                        ..Default::default()
                    },
                )?,
                data,
                code,
            )
        }
        "network" => {
            let nodes = (0..16)
                .map(|i| {
                    let mut n = NetworkNode::new(format!("N{i}"));
                    n.radius = 6. + (phase(seed) + f64::from(i)).sin() * 2.;
                    n
                })
                .collect::<Vec<_>>();
            let links = (1..16)
                .map(|i| {
                    NetworkLink::new(
                        format!("N{}", (i - 1) / 2),
                        format!("N{i}"),
                        2. + (phase(seed) + f64::from(i)).cos(),
                    )
                })
                .collect::<Vec<_>>();
            let data = json!({"nodes":nodes.iter().map(|n|json!({"id":n.id,"radius":n.radius})).collect::<Vec<_>>(),"links":links.iter().map(|l|json!({"source":l.source,"target":l.target,"value":l.value})).collect::<Vec<_>>()});
            let (link_distance, code) = match variant {
                "compact" => (
                    24.,
                    "network(&config, &nodes, &links, &NetworkOptions { iterations: 100, link_distance: 24., ..Default::default() })?",
                ),
                "long" => (
                    100.,
                    "network(&config, &nodes, &links, &NetworkOptions { iterations: 100, link_distance: 100., ..Default::default() })?",
                ),
                _ => (
                    52. + phase(seed).sin() * 12.,
                    "network(&config, &nodes, &links, &NetworkOptions { iterations: 100, link_distance: 52. + phase(seed).sin() * 12., ..Default::default() })?",
                ),
            };
            (
                network(
                    c,
                    &nodes,
                    &links,
                    &NetworkOptions {
                        iterations: 100,
                        link_distance,
                        repulsion: if active(settings, "repulsion") {
                            720.
                        } else {
                            240.
                        },
                        labels: !active(settings, "node-labels"),
                    },
                )?,
                data,
                code,
            )
        }
        _ => return Err(ChartError(format!("Unknown relational chart: {kind}"))),
    };
    if active(settings, "node-outline") {
        for mark in &mut scene.marks {
            outline(mark);
        }
    }
    if kind == "chord" {
        for mark in &mut scene.marks {
            if mark.id.starts_with("ribbon:") && active(settings, "ribbon-opacity") {
                mark.opacity = 0.25;
            } else if mark.id.starts_with("group:") && active(settings, "group-outline") {
                outline(mark);
            }
        }
    }
    Ok(Sample { scene, data, code })
}

fn outline(mark: &mut Mark) {
    mark.stroke = Some(GraphicStroke {
        color: Color(40, 57, 67, 255),
        width: 2.,
        cap: StrokeCap::Butt,
    });
}
