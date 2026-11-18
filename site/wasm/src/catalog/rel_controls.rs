//! Independent presentation and layout controls for radial, hierarchy and map samples.
use super::{Control, Settings};

pub(super) fn active(settings: &Settings, key: &str) -> bool {
    settings.get(key) == "alternate"
}

const PIE: &[Control] = &[
    Control {
        key: "pad-angle",
        label: "Slice gap",
        choices: &[("default", "Narrow"), ("alternate", "Wide")],
    },
    Control {
        key: "arc-labels",
        label: "Arc labels",
        choices: &[("default", "Hide"), ("alternate", "Show")],
    },
];
const RADIAL_BAR: &[Control] = &[
    Control {
        key: "tracks",
        label: "Background tracks",
        choices: &[("default", "Show"), ("alternate", "Hide")],
    },
    Control {
        key: "maximum",
        label: "Maximum value",
        choices: &[("default", "Automatic"), ("alternate", "60")],
    },
];
const POLAR_BAR: &[Control] = &[
    Control {
        key: "pad-angle",
        label: "Sector gap",
        choices: &[("default", "Narrow"), ("alternate", "Wide")],
    },
    Control {
        key: "inner-radius",
        label: "Inner radius",
        choices: &[("default", "15%"), ("alternate", "40%")],
    },
];
const WAFFLE: &[Control] = &[
    Control {
        key: "grid",
        label: "Grid dimensions",
        choices: &[("default", "6 × 10"), ("alternate", "8 × 12")],
    },
    Control {
        key: "cell-gap",
        label: "Cell gap",
        choices: &[("default", "4 px"), ("alternate", "1 px")],
    },
];
const FUNNEL: &[Control] = &[
    Control {
        key: "stage-gap",
        label: "Stage spacing",
        choices: &[("default", "12 px"), ("alternate", "3 px")],
    },
    Control {
        key: "stage-opacity",
        label: "Stage opacity",
        choices: &[("default", "100%"), ("alternate", "55%")],
    },
];
const TREE: &[Control] = &[
    Control {
        key: "node-size",
        label: "Node radius",
        choices: &[("default", "5 px"), ("alternate", "9 px")],
    },
    Control {
        key: "node-labels",
        label: "Node labels",
        choices: &[("default", "Show"), ("alternate", "Hide")],
    },
];
const TREEMAP: &[Control] = &[
    Control {
        key: "tile-padding",
        label: "Tile padding",
        choices: &[("default", "4 px"), ("alternate", "1 px")],
    },
    Control {
        key: "node-outline",
        label: "Tile outline",
        choices: &[("default", "Default"), ("alternate", "Dark stroke")],
    },
];
const SUNBURST: &[Control] = &[
    Control {
        key: "node-labels",
        label: "Sector labels",
        choices: &[("default", "Show"), ("alternate", "Hide")],
    },
    Control {
        key: "node-outline",
        label: "Sector outline",
        choices: &[("default", "Default"), ("alternate", "Dark stroke")],
    },
];
const CIRCLE_PACKING: &[Control] = &[
    Control {
        key: "root-circle",
        label: "Root circle",
        choices: &[("default", "Hide"), ("alternate", "Show")],
    },
    Control {
        key: "node-outline",
        label: "Circle outline",
        choices: &[("default", "Default"), ("alternate", "Dark stroke")],
    },
];
const ICICLE: &[Control] = &[
    Control {
        key: "node-labels",
        label: "Block labels",
        choices: &[("default", "Show"), ("alternate", "Hide")],
    },
    Control {
        key: "node-outline",
        label: "Block outline",
        choices: &[("default", "Default"), ("alternate", "Dark stroke")],
    },
];
const CHORD: &[Control] = &[
    Control {
        key: "ribbon-opacity",
        label: "Ribbon opacity",
        choices: &[("default", "65%"), ("alternate", "25%")],
    },
    Control {
        key: "group-outline",
        label: "Arc outline",
        choices: &[("default", "None"), ("alternate", "Dark stroke")],
    },
];
const SANKEY: &[Control] = &[
    Control {
        key: "node-padding",
        label: "Node spacing",
        choices: &[("default", "18 px"), ("alternate", "5 px")],
    },
    Control {
        key: "link-opacity",
        label: "Link opacity",
        choices: &[("default", "45%"), ("alternate", "80%")],
    },
];
const NETWORK: &[Control] = &[
    Control {
        key: "repulsion",
        label: "Node repulsion",
        choices: &[("default", "240"), ("alternate", "720")],
    },
    Control {
        key: "node-labels",
        label: "Node labels",
        choices: &[("default", "Show"), ("alternate", "Hide")],
    },
];
const CHOROPLETH: &[Control] = &[
    Control {
        key: "border-width",
        label: "Region borders",
        choices: &[("default", "0.75 px"), ("alternate", "2 px")],
    },
    Control {
        key: "map-padding",
        label: "Projection inset",
        choices: &[("default", "0 px"), ("alternate", "16 px")],
    },
];
const GEO_MAP: &[Control] = &[
    Control {
        key: "graticule",
        label: "Graticule spacing",
        choices: &[("default", "30°"), ("alternate", "15°")],
    },
    Control {
        key: "symbols",
        label: "Symbol radii",
        choices: &[("default", "4–14 px"), ("alternate", "8–22 px")],
    },
];

pub(super) fn controls(kind: &str) -> &'static [Control] {
    match kind {
        "pie" => PIE,
        "radial-bar" => RADIAL_BAR,
        "polar-bar" => POLAR_BAR,
        "waffle" => WAFFLE,
        "funnel" => FUNNEL,
        "tree" => TREE,
        "treemap" => TREEMAP,
        "sunburst" => SUNBURST,
        "circle-packing" => CIRCLE_PACKING,
        "icicle" => ICICLE,
        "chord" => CHORD,
        "sankey" => SANKEY,
        "network" => NETWORK,
        "choropleth" => CHOROPLETH,
        "geo-map" => GEO_MAP,
        _ => &[],
    }
}

const NODE_OUTLINE_CODE: &str = "for mark in &mut scene.marks {\n    mark.stroke = Some(GraphicStroke { color: Color(40, 57, 67, 255), width: 2., cap: StrokeCap::Butt });\n}";

pub(super) fn code(kind: &str, settings: &Settings) -> String {
    let overrides: &[(&str, &str)] = match kind {
        "pie" => &[
            ("pad-angle", "options.pad_angle = 0.10;"),
            ("arc-labels", "options.labels = true;"),
        ],
        "radial-bar" => &[
            ("tracks", "options.tracks = false;"),
            ("maximum", "options.max_value = Some(60.);"),
        ],
        "polar-bar" => &[
            ("pad-angle", "options.pad_angle = 0.12;"),
            ("inner-radius", "options.inner_radius = 0.4;"),
        ],
        "waffle" => &[
            ("grid", "options.rows = 8;\noptions.columns = 12;"),
            ("cell-gap", "options.padding = 1.;"),
        ],
        "funnel" => &[
            ("stage-gap", "options.spacing = 3.;"),
            ("stage-opacity", "options.fill_opacity = 0.55;"),
        ],
        "tree" => &[
            ("node-size", "options.node_radius = 9.;"),
            ("node-labels", "options.hierarchy.labels = false;"),
        ],
        "treemap" => &[
            ("tile-padding", "options.padding = 1.;"),
            ("node-outline", NODE_OUTLINE_CODE),
        ],
        "sunburst" => &[
            ("node-labels", "options.labels = false;"),
            ("node-outline", NODE_OUTLINE_CODE),
        ],
        "circle-packing" => &[
            ("root-circle", "options.include_root = true;"),
            ("node-outline", NODE_OUTLINE_CODE),
        ],
        "icicle" => &[
            ("node-labels", "options.labels = false;"),
            ("node-outline", NODE_OUTLINE_CODE),
        ],
        "chord" => &[
            (
                "ribbon-opacity",
                "for mark in scene.marks.iter_mut().filter(|m| m.id.starts_with(\"ribbon:\")) { mark.opacity = 0.25; }",
            ),
            (
                "group-outline",
                "for mark in scene.marks.iter_mut().filter(|m| m.id.starts_with(\"group:\")) {\n    mark.stroke = Some(GraphicStroke { color: Color(40, 57, 67, 255), width: 2., cap: StrokeCap::Butt });\n}",
            ),
        ],
        "sankey" => &[
            ("node-padding", "options.node_padding = 5.;"),
            ("link-opacity", "options.link_opacity = 0.8;"),
        ],
        "network" => &[
            ("repulsion", "options.repulsion = 720.;"),
            ("node-labels", "options.labels = false;"),
        ],
        "choropleth" => &[
            ("border-width", "options.border_width = 2.;"),
            ("map-padding", "options.padding = 16.;"),
        ],
        "geo-map" => &[
            ("graticule", "graticule.step_minor = [15., 15.];"),
            ("symbols", "symbols.radius_range = [8., 22.];"),
        ],
        _ => &[],
    };
    overrides
        .iter()
        .filter(|(key, _)| active(settings, key))
        .map(|(_, assignment)| *assignment)
        .collect::<Vec<_>>()
        .join("\n")
}
