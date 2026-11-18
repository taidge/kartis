use super::{Control, Settings};

const TOOLTIP: Control = Control {
    key: "tooltip-content",
    label: "Tooltip content",
    choices: &[
        ("detail", "Label and value"),
        ("value", "Value only"),
        ("hidden", "Hidden"),
    ],
};
const FORMAT: Control = Control {
    key: "tooltip-format",
    label: "Tooltip number format",
    choices: &[("auto", "Automatic"), ("fixed", "Two decimals")],
};
const STYLE: Control = Control {
    key: "tooltip-style",
    label: "Tooltip appearance",
    choices: &[("default", "Dark"), ("light", "Light")],
};
const LEGEND: Control = Control {
    key: "legend-display",
    label: "Legend",
    choices: &[
        ("inline", "Inline"),
        ("stacked", "Stacked"),
        ("hidden", "Hidden"),
    ],
};

pub(super) fn controls(kind: &str) -> &'static [Control] {
    match kind {
        "line" | "area" | "bar" | "scatter" | "radar" | "stream" | "bump" | "area-bump"
        | "marimekko" | "pie" | "radial-bar" | "polar-bar" | "waffle" | "funnel" | "chord"
        | "boxplot" | "bullet" | "swarm" | "delaunay" => &[LEGEND, TOOLTIP, FORMAT, STYLE],
        _ => &[TOOLTIP, FORMAT, STYLE],
    }
}
pub(super) fn code(_kind: &str, settings: &Settings) -> String {
    let mut lines = Vec::new();
    match settings.get("tooltip-content") {
        "hidden" => lines.push("browser_options.tooltip = false;"),
        "value" => {
            lines.push("browser_options.tooltip_options.lines = vec![vec![TooltipPart::Value]];")
        }
        _ => {}
    }
    if settings.get("tooltip-format") == "fixed" {
        lines.push("browser_options.tooltip_options.value_format = ValueFormat::Fixed(2);");
    }
    if settings.get("tooltip-style") == "light" {
        lines.push("// The host styles the native tooltip with the light surface palette.");
    }
    match settings.get("legend-display") {
        "hidden" => lines.push("// Hide the host legend while retaining series visibility."),
        "stacked" => lines.push("// Arrange the native legend items in a column."),
        _ => {}
    }
    lines.join("\n")
}
