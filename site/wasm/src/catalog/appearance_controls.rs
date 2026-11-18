//! Nivo-style categorical palettes and sequential color schemes for showcase Scenes.
use std::collections::BTreeMap;

use glory::render::PathCommand;
use kartis::{
    CategoricalColorScheme, Color, ColorScheme, DrawItem, FillPaint, Geometry, Label, Mark, Result,
    Scene, TextAnchor,
};

use super::{Control, Settings};

const ORDINAL: Control = Control {
    key: "palette",
    label: "Series palette",
    choices: &[
        ("default", "As authored"),
        ("tableau10", "Tableau 10"),
        ("dark2", "Dark 2"),
    ],
};
const CONTINUOUS: Control = Control {
    key: "palette",
    label: "Color scale",
    choices: &[
        ("default", "As authored"),
        ("viridis", "Viridis"),
        ("magma", "Magma"),
    ],
};
const ANNOTATE: Control = Control {
    key: "label-mode",
    label: "Data labels",
    choices: &[
        ("default", "As authored"),
        ("featured", "Feature up to three marks"),
    ],
};
const LABELS: Control = Control {
    key: "label-mode",
    label: "Data labels",
    choices: &[
        ("default", "As authored"),
        ("featured", "Feature up to three marks"),
        ("hidden", "Hide data labels"),
    ],
};
const FONT: Control = Control {
    key: "label-size",
    label: "Chart text size",
    choices: &[("default", "100%"), ("large", "125%")],
};

pub(super) fn controls(kind: &str) -> &'static [Control] {
    match kind {
        "heatmap" | "calendar" | "time-range" => &[CONTINUOUS, ANNOTATE, FONT],
        "choropleth" | "contour" | "density" => &[CONTINUOUS, ANNOTATE],
        "pie" | "radial-bar" | "waffle" | "voronoi" | "delaunay" | "geo-map" => {
            &[ORDINAL, ANNOTATE]
        }
        "chord" | "sankey" | "funnel" | "tree" | "treemap" | "sunburst" | "circle-packing"
        | "icicle" | "network" | "bump" | "area-bump" => &[ORDINAL, LABELS, FONT],
        _ => &[ORDINAL, ANNOTATE, FONT],
    }
}

type Rgb = (u8, u8, u8);
fn rgb(c: Color) -> Rgb {
    (c.0, c.1, c.2)
}
fn neutral(c: Color) -> bool {
    c.0.max(c.1).max(c.2) - c.0.min(c.1).min(c.2) < 8 || rgb(c) == (108, 117, 124)
}

struct Ordinal {
    mapping: BTreeMap<Rgb, Color>,
    colors: &'static [Color],
    next: usize,
}
impl Ordinal {
    fn new(scheme: CategoricalColorScheme) -> Self {
        let colors = scheme.palette();
        let mut mapping = BTreeMap::new();
        // These are the authored theme colors in config(). Both source palettes assign
        // the first series slot zero, regardless of sample order or visibility.
        for (i, color) in [
            (34, 126, 119),
            (232, 137, 86),
            (113, 127, 187),
            (211, 173, 78),
            (90, 165, 186),
        ]
        .into_iter()
        .enumerate()
        {
            mapping.insert(color, colors[i]);
        }
        for (i, c) in CategoricalColorScheme::Nivo.palette().iter().enumerate() {
            mapping.insert(rgb(*c), colors[i]);
        }
        Self {
            mapping,
            colors,
            next: 0,
        }
    }
    fn map(&mut self, c: Color) -> Color {
        if neutral(c) {
            return c;
        }
        let mapped = self.mapping.entry(rgb(c)).or_insert_with(|| {
            let choice = self.colors[self.next % self.colors.len()];
            self.next += 1;
            choice
        });
        Color(mapped.0, mapped.1, mapped.2, c.3)
    }
}
/// Recover the normalized parameter of an authored two-stop RGB ramp. Missing
/// fills and independent ink (axis, outline, marker) fall outside that ramp.
fn ramp(kind: &str, settings: &Settings) -> ([f64; 3], [f64; 3]) {
    match kind {
        "choropleth" => ([230., 240., 250.], [40., 120., 190.]),
        "contour" => ([217., 239., 226.], [19., 119., 122.]),
        "density" => ([240., 229., 214.], [147., 106., 170.]),
        "heatmap" if settings.get("highColor") == "berry" => {
            ([234., 244., 236.], [160., 58., 106.])
        }
        "heatmap" | "time-range" => ([234., 244., 236.], [34., 126., 119.]),
        _ => ([235., 244., 251.], [31., 99., 171.]),
    }
}
fn transfer(c: Color, from: [f64; 3], to: [f64; 3], scheme: ColorScheme) -> Color {
    let sample = [f64::from(c.0), f64::from(c.1), f64::from(c.2)];
    let delta = std::array::from_fn::<_, 3, _>(|i| to[i] - from[i]);
    let denom: f64 = delta.iter().map(|x| x * x).sum();
    let t = ((0..3)
        .map(|i| (sample[i] - from[i]) * delta[i])
        .sum::<f64>()
        / denom)
        .clamp(0., 1.);
    if (0..3).any(|i| (sample[i] - from[i] - t * delta[i]).abs() > 2.) {
        return c;
    }
    let mapped = scheme.sample(t).expect("finite RGB projection");
    Color(mapped.0, mapped.1, mapped.2, c.3)
}
fn recolor(scene: &mut Scene, mut map: impl FnMut(Color) -> Color) {
    // Register legend colors before marks, retaining their correspondence in every backend.
    for row in &mut scene.legend {
        row.color = map(row.color);
    }
    for mark in &mut scene.marks {
        mark.fill = mark.fill.map(&mut map);
        if let Some(stroke) = &mut mark.stroke {
            stroke.color = map(stroke.color);
        }
        match &mut mark.fill_paint {
            Some(FillPaint::Linear { stops, .. } | FillPaint::Radial { stops, .. }) => {
                for stop in stops {
                    stop.color = map(stop.color);
                }
            }
            Some(FillPaint::Pattern(pattern)) => {
                pattern.foreground = map(pattern.foreground);
                pattern.background = map(pattern.background);
            }
            None => {}
        }
    }
    for label in &mut scene.labels {
        label.color = map(label.color);
    }
    for stroke in scene.typography.label_strokes.values_mut() {
        stroke.color = map(stroke.color);
    }
}
fn anchor(mark: &Mark) -> Option<(f64, f64)> {
    match &mark.geometry {
        Geometry::Circle { x, y, .. } => Some((*x, *y)),
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => Some((*x + width / 2., *y + height / 2.)),
        Geometry::Path(commands) => {
            let mut bounds = [
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ];
            for command in commands {
                let p = match command {
                    PathCommand::Move(p)
                    | PathCommand::Line(p)
                    | PathCommand::Cubic { end: p, .. } => p,
                    PathCommand::Close => continue,
                };
                bounds = [
                    bounds[0].min(p.x),
                    bounds[1].min(p.y),
                    bounds[2].max(p.x),
                    bounds[3].max(p.y),
                ];
            }
            bounds[0]
                .is_finite()
                .then(|| ((bounds[0] + bounds[2]) / 2., (bounds[1] + bounds[3]) / 2.))
        }
    }
}
fn feature(scene: &mut Scene) {
    let mut marks: Vec<_> = scene
        .marks
        .iter()
        .filter(|m| {
            !m.series.is_empty()
                && !m.label.is_empty()
                && m.opacity > 0.
                && (m.has_fill() || m.stroke.is_some())
                && !m.label.ends_with(": missing")
        })
        .collect();
    marks.sort_by(|a, b| b.value.total_cmp(&a.value).then_with(|| a.id.cmp(&b.id)));
    let mut placed: Vec<(f64, f64, f64)> = Vec::new();
    let mut added = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for mark in marks {
        let Some((x, y)) = anchor(mark) else { continue };
        let text = mark.label.split(": ").next().unwrap_or(&mark.label);
        let half = (text.chars().count() as f64 * 3.4 + 4.).min(scene.width / 2. - 4.);
        let x = x.clamp(half + 2., scene.width - half - 2.);
        let y = (y - 7.).clamp(14., scene.height - 4.);
        if placed
            .iter()
            .any(|&(a, b, w)| (a - x).abs() < w + half && (b - y).abs() < 15.)
        {
            continue;
        }
        if !seen.insert(text) {
            continue;
        }
        placed.push((x, y, half));
        added.push((
            mark.series.clone(),
            Label {
                x,
                y,
                text: text.to_owned(),
                size: 12.,
                color: Color(46, 56, 62, 255),
                anchor: TextAnchor::Middle,
                rotation: 0.,
            },
        ));
        if added.len() == 3 {
            break;
        }
    }
    for (series, label) in added {
        let i = scene.labels.len();
        scene.label_series.insert(i, series);
        scene.labels.push(label);
        if !scene.draw_order.is_empty() {
            scene.draw_order.push(DrawItem::Label(i));
        }
    }
}
pub(super) fn apply(kind: &str, settings: &Settings, scene: &mut Scene) -> Result<()> {
    match settings.get("palette") {
        "tableau10" | "dark2" => {
            let scheme = if settings.get("palette") == "tableau10" {
                CategoricalColorScheme::Tableau10
            } else {
                CategoricalColorScheme::Dark2
            };
            let mut ordinal = Ordinal::new(scheme);
            recolor(scene, |c| ordinal.map(c));
        }
        "viridis" | "magma" => {
            let scheme = if settings.get("palette") == "viridis" {
                ColorScheme::Viridis
            } else {
                ColorScheme::Magma
            };
            let (from, to) = ramp(kind, settings);
            recolor(scene, |c| transfer(c, from, to, scheme));
        }
        _ => {}
    }
    match settings.get("label-mode") {
        "featured" => feature(scene),
        "hidden" => {
            let owned = scene.label_series.clone();
            super::axes_controls::retain_labels(scene, |i, _| !owned.contains_key(&i));
        }
        _ => {}
    }
    if settings.get("label-size") == "large" {
        for label in &mut scene.labels {
            label.size *= 1.25;
        }
    }
    Ok(())
}
pub(super) fn code(_kind: &str, settings: &Settings) -> String {
    let mut lines = Vec::new();
    match settings.get("palette") {
        "tableau10" => lines.push("let colors = CategoricalColorScheme::Tableau10.palette();"),
        "dark2" => lines.push("let colors = CategoricalColorScheme::Dark2.palette();"),
        "viridis" => lines.push("let color = ColorScheme::Viridis.sample(t)?;"),
        "magma" => lines.push("let color = ColorScheme::Magma.sample(t)?;"),
        _ => {}
    }
    match settings.get("label-mode") {
        "featured" => {
            lines.push("// Place up to three data labels, retaining their series ownership.")
        }
        "hidden" => lines.push("// Omit series-owned labels, preserving axis text."),
        _ => {}
    }
    if settings.get("label-size") == "large" {
        lines.push("for label in &mut scene.labels { label.size *= 1.25; }");
    }
    lines.join("\n")
}
