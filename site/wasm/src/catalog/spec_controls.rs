use super::{Control, Settings};

pub(super) fn controls(kind: &str) -> &'static [Control] {
    match kind {
        "heatmap" => &[
            Control {
                key: "cellLabels",
                label: "Cell values",
                choices: &[("default", "Hidden"), ("show", "Show")],
            },
            Control {
                key: "highColor",
                label: "High-value color",
                choices: &[("default", "Teal"), ("berry", "Berry")],
            },
        ],
        "boxplot" => &[
            Control {
                key: "whiskerCaps",
                label: "Whisker caps",
                choices: &[("default", "60% of box"), ("full", "Full box width")],
            },
            Control {
                key: "medianWidth",
                label: "Median line",
                choices: &[("default", "Standard"), ("bold", "4 px")],
            },
        ],
        "bullet" => &[
            Control {
                key: "measureSize",
                label: "Measure thickness",
                choices: &[("default", "40%"), ("wide", "70%")],
            },
            Control {
                key: "markerWidth",
                label: "Target line",
                choices: &[("default", "2 px"), ("bold", "5 px")],
            },
        ],
        "calendar" => &[
            Control {
                key: "dayBorders",
                label: "Day borders",
                choices: &[("default", "Visible"), ("off", "Hidden")],
            },
            Control {
                key: "monthBorders",
                label: "Month borders",
                choices: &[("default", "Visible"), ("off", "Hidden")],
            },
        ],
        "time-range" => &[
            Control {
                key: "dayCorners",
                label: "Day corners",
                choices: &[("default", "Rounded"), ("square", "Square")],
            },
            Control {
                key: "dayOpacity",
                label: "Day opacity",
                choices: &[("default", "100%"), ("muted", "55%")],
            },
        ],
        "swarm" => &[
            Control {
                key: "nodeBorder",
                label: "Node border",
                choices: &[("default", "None"), ("outline", "Dark outline")],
            },
            Control {
                key: "nodeOpacity",
                label: "Node opacity",
                choices: &[("default", "100%"), ("muted", "55%")],
            },
        ],
        "voronoi" => &[
            Control {
                key: "cellOpacity",
                label: "Cell opacity",
                choices: &[("default", "45%"), ("solid", "85%")],
            },
            Control {
                key: "cellBorder",
                label: "Cell border",
                choices: &[("default", "1.5 px"), ("heavy", "4 px")],
            },
        ],
        "delaunay" => &[
            Control {
                key: "siteRadius",
                label: "Site radius",
                choices: &[("default", "5 px"), ("small", "2 px")],
            },
            Control {
                key: "linkWidth",
                label: "Link width",
                choices: &[("default", "1.5 px"), ("thick", "3.5 px")],
            },
        ],
        "contour" => &[
            Control {
                key: "isolines",
                label: "Boundary interpolation",
                choices: &[("default", "Linear"), ("step", "Grid midpoints")],
            },
            Control {
                key: "outline",
                label: "Contour lines",
                choices: &[("default", "None"), ("ink", "Dark outline")],
            },
        ],
        "density" => &[
            Control {
                key: "cellSize",
                label: "Estimator grid",
                choices: &[("default", "4 px"), ("coarse", "8 px")],
            },
            Control {
                key: "bandOpacity",
                label: "Density band opacity",
                choices: &[("default", "100%"), ("muted", "55%")],
            },
        ],
        _ => &[],
    }
}

pub(super) fn code(kind: &str, settings: &Settings) -> String {
    controls(kind).iter().filter_map(|control| {
        match (kind, control.key, settings.get(control.key)) {
            ("heatmap", "cellLabels", "show") => Some("options.labels = true;"),
            ("heatmap", "highColor", "berry") => Some("options.high = Color(160, 58, 106, 255);"),
            ("boxplot", "whiskerCaps", "full") => Some("options.cap_size = 1.;"),
            ("boxplot", "medianWidth", "bold") => Some("options.median.width = 4.;"),
            ("bullet", "measureSize", "wide") => Some("options.measure_size = 0.7;"),
            ("bullet", "markerWidth", "bold") => Some("options.marker_width = 5.;"),
            ("calendar", "dayBorders", "off") => Some("options.day_border = None;"),
            ("calendar", "monthBorders", "off") => Some("options.month_border = None;"),
            ("time-range", "dayCorners", "square") => Some("options.day_radius = 0.;"),
            ("time-range", "dayOpacity", "muted") => Some("options.day_opacity = 0.55;"),
            ("swarm", "nodeBorder", "outline") => Some("for node in &mut scene.marks { node.stroke = Some(GraphicStroke { color: Color(36, 58, 70, 255), width: 1.5, cap: StrokeCap::Butt }); }"),
            ("swarm", "nodeOpacity", "muted") => Some("for node in &mut scene.marks { node.opacity = 0.55; }"),
            ("voronoi", "cellOpacity", "solid") => Some("mark.opacity = 0.85;"),
            ("voronoi", "cellBorder", "heavy") => Some("stroke.width = 4.;"),
            ("delaunay", "siteRadius", "small") => Some("options.site_radius = 2.;"),
            ("delaunay", "linkWidth", "thick") => Some("options.link_width = 3.5;"),
            ("contour", "isolines", "step") => Some("compute.smooth = false;"),
            ("contour", "outline", "ink") => Some("style.stroke = Some(GraphicStroke { color: Color(36, 58, 70, 255), width: 1.5, cap: StrokeCap::Butt });"),
            ("density", "cellSize", "coarse") => Some("options.cell_size = 8.;"),
            ("density", "bandOpacity", "muted") => Some("style.opacity = 0.55;"),
            _ => None,
        }
    }).collect::<Vec<_>>().join("\n")
}
