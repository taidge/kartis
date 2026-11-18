//! Independent display options for Cartesian showcase examples.
use glory::render::{GraphicStroke, StrokeCap};
use kartis::{Color, ScaleKind, Scene};

use super::{Control, Settings};

const VISIBLE: &[(&str, &str)] = &[("on", "Show"), ("off", "Hide")];
const OPTIONAL: &[(&str, &str)] = &[("off", "Hide"), ("on", "Show")];
const SCALES: &[(&str, &str)] = &[("linear", "Linear"), ("sqrt", "Square root")];
macro_rules! control {
    ($key:literal, $label:literal, $choices:expr) => {
        Control {
            key: $key,
            label: $label,
            choices: $choices,
        }
    };
}

pub(super) fn controls(kind: &str) -> &'static [Control] {
    match kind {
        "line" | "area" => &[
            control!("points", "Data points", VISIBLE),
            control!("y-scale", "Y scale", SCALES),
        ],
        "bar" => &[
            control!(
                "corners",
                "Bar corners",
                &[("rounded", "Rounded"), ("square", "Square")]
            ),
            control!(
                "padding",
                "Band padding",
                &[("normal", "20%"), ("wide", "50%")]
            ),
        ],
        "scatter" => &[
            control!("x-scale", "X scale", SCALES),
            control!("y-scale", "Y scale", SCALES),
        ],
        "radar" => &[
            control!(
                "maximum",
                "Radius domain",
                &[("auto", "Fit data"), ("fixed", "0–120")]
            ),
            control!(
                "fill-opacity",
                "Fill opacity",
                &[("normal", "20%"), ("strong", "65%")]
            ),
        ],
        "stream" => &[
            control!(
                "curve",
                "Layer curve",
                &[("basis", "B-spline"), ("linear", "Linear")]
            ),
            control!("dots", "Layer dots", OPTIONAL),
        ],
        "marimekko" => &[
            control!("labels", "Value labels", OPTIONAL),
            control!(
                "border",
                "Cell border",
                &[("none", "None"), ("ink", "2 px")]
            ),
        ],
        "area-bump" => &[
            control!("start-labels", "Start labels", VISIBLE),
            control!(
                "fill-opacity",
                "Band opacity",
                &[("normal", "80%"), ("light", "35%")]
            ),
        ],
        "parallel-coordinates" => &[
            control!(
                "line-width",
                "Record line width",
                &[("normal", "Layout default"), ("wide", "4 px")]
            ),
            control!(
                "line-opacity",
                "Record opacity",
                &[("normal", "Layout default"), ("light", "30%")]
            ),
        ],
        _ => &[],
    }
}

pub(super) fn scale(value: &str) -> ScaleKind {
    if value == "sqrt" {
        ScaleKind::Sqrt
    } else {
        ScaleKind::Linear
    }
}

/// Operate on data marks shared by the site's basic and styled layouts.
pub(super) fn style_marks(kind: &str, scene: &mut Scene, settings: &Settings) {
    match kind {
        "marimekko" if settings.get("border") == "ink" => {
            for mark in scene
                .marks
                .iter_mut()
                .filter(|mark| !mark.series.is_empty())
            {
                mark.stroke = Some(GraphicStroke {
                    color: Color(35, 56, 75, 255),
                    width: 2.,
                    cap: StrokeCap::Round,
                });
            }
        }
        "parallel-coordinates" => {
            for mark in scene
                .marks
                .iter_mut()
                .filter(|mark| !mark.series.is_empty())
            {
                if settings.get("line-width") == "wide"
                    && let Some(stroke) = &mut mark.stroke
                {
                    stroke.width = 4.;
                }
                if settings.get("line-opacity") == "light" {
                    mark.opacity = 0.3;
                }
            }
        }
        _ => {}
    }
}

pub(super) fn code(kind: &str, settings: &Settings) -> String {
    let mut lines = Vec::new();
    for control in controls(kind) {
        let value = settings.get(control.key);
        let line = match (kind, control.key, value) {
            ("line" | "area", "points", "off") => "options.points = false;",
            (_, "x-scale", "sqrt") => "options.x_scale = ScaleKind::Sqrt;",
            (_, "y-scale", "sqrt") => "options.y_scale = ScaleKind::Sqrt;",
            ("bar", "corners", "square") => "options.corner_radius = 0.;",
            ("bar", "padding", "wide") => "options.padding = 0.5;",
            ("radar", "maximum", "fixed") => "options.max_value = Some(120.);",
            ("radar", "fill-opacity", "strong") => "options.fill_opacity = 0.65;",
            ("stream", "curve", "linear") => "options.curve = Curve::Linear;",
            ("stream", "dots", "on") => "options.dots = Some(StreamDotOptions::default());",
            ("marimekko", "labels", "on") => "options.labels = true;",
            ("marimekko", "border", "ink") => {
                "// After layout, for each cell mark:\nmark.stroke = Some(GraphicStroke { color: Color(35, 56, 75, 255), width: 2., cap: StrokeCap::Round });"
            }
            ("area-bump", "start-labels", "off") => "options.start_labels.enabled = false;",
            ("area-bump", "fill-opacity", "light") => "options.style.fill_opacity = 0.35;",
            ("parallel-coordinates", "line-width", "wide") => {
                "// After layout, for each record mark:\nif let Some(stroke) = &mut mark.stroke { stroke.width = 4.; }"
            }
            ("parallel-coordinates", "line-opacity", "light") => {
                "// After layout, for each record mark:\nmark.opacity = 0.3;"
            }
            _ => continue,
        };
        lines.push(line);
    }
    lines.join("\n")
}
