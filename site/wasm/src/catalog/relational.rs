use serde_json::json;

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
pub(super) fn sample(kind: &str, c: &ChartConfig, seed: u32, variant: &str) -> Result<Sample> {
    let mut source = hierarchy(seed);
    let options = HierarchyOptions {
        padding: 4.,
        labels: true,
        ..Default::default()
    };
    let (scene, data, code) = match kind {
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
                                "radial" => TreeOrientation::Radial,
                                _ => TreeOrientation::LeftToRight,
                            },
                            node_radius: 5.,
                            ..Default::default()
                        },
                    )?,
                    data,
                    match variant {
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
                "treemap" => (
                    treemap(c, &source, &options)?,
                    data,
                    "treemap(&config, &hierarchy, &HierarchyOptions::default())?",
                ),
                "sunburst" => (
                    sunburst(c, &source, &options)?,
                    data,
                    "sunburst(&config, &hierarchy, &HierarchyOptions::default())?",
                ),
                "circle-packing" => (
                    circle_packing(c, &source, &options)?,
                    data,
                    "circle_packing(&config, &hierarchy, &HierarchyOptions::default())?",
                ),
                _ => (
                    icicle(c, &source, &options)?,
                    data,
                    "icicle(&config, &hierarchy, &HierarchyOptions::default())?",
                ),
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
            (
                chord(c, &ids, &matrix, 0.06)?,
                data,
                "chord(&config, &ids, &matrix, 0.06)?",
            )
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
            (
                sankey(
                    c,
                    &nodes,
                    &links,
                    &SankeyOptions {
                        node_width: 18.,
                        node_padding: 18.,
                        ..Default::default()
                    },
                )?,
                data,
                "sankey(&config, &nodes, &links, &SankeyOptions::default())?",
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
            (
                network(
                    c,
                    &nodes,
                    &links,
                    &NetworkOptions {
                        iterations: 100,
                        link_distance: 52. + phase(seed).sin() * 12.,
                        ..Default::default()
                    },
                )?,
                data,
                "network(&config, &nodes, &links, &NetworkOptions { iterations: 100, ..Default::default() })?",
            )
        }
        _ => return Err(ChartError(format!("Unknown relational chart: {kind}"))),
    };
    Ok(Sample { scene, data, code })
}
