//! Real Kartis layouts used by the public SVG/Canvas catalog.
//!
//! Dataset changes retain business IDs. Every entry calls its own chart layout;
//! the owner decides whether a particular pair of Scenes admits interpolation.
use kartis::*;
use serde_json::{Value, json};

#[path = "catalog/cartesian.rs"]
mod cartesian;
#[path = "catalog/relational.rs"]
mod relational;
#[path = "catalog/spatial.rs"]
mod spatial;

pub struct ChartKind {
    pub slug: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
}
macro_rules! kinds {
    ($(($slug:literal, $name:literal, $category:literal, $description:literal)),* $(,)?) => {
        pub const KINDS: &[ChartKind] = &[$(ChartKind {
            slug: $slug, name: $name, category: $category, description: $description,
        }),*];
    };
}
kinds![
    (
        "line",
        "Line",
        "Cartesian",
        "Compare two time series with smooth curves and individual observations."
    ),
    (
        "area",
        "Area",
        "Cartesian",
        "Filled time series expose magnitude against a shared zero baseline."
    ),
    (
        "bar",
        "Bar",
        "Cartesian",
        "Grouped category values with rounded bars and quantitative axes."
    ),
    (
        "scatter",
        "Scatter plot",
        "Cartesian",
        "Three populations of observations on independent numeric axes."
    ),
    (
        "radar",
        "Radar",
        "Cartesian",
        "Compare product profiles across five radial dimensions."
    ),
    (
        "stream",
        "Stream",
        "Cartesian",
        "A centered stack of smooth bands shows changing contributions."
    ),
    (
        "bump",
        "Bump",
        "Cartesian",
        "Stable series trace their ranks through six periods."
    ),
    (
        "area-bump",
        "Area bump",
        "Cartesian",
        "Ranked ribbons retain their changing quantitative thickness."
    ),
    (
        "marimekko",
        "Marimekko",
        "Cartesian",
        "Category width and stacked composition encode two quantities."
    ),
    (
        "parallel-coordinates",
        "Parallel coordinates",
        "Cartesian",
        "Trace individual records across four independent dimensions."
    ),
    (
        "pie",
        "Pie",
        "Part to whole",
        "Angular sectors encode four contributions to a whole."
    ),
    (
        "radial-bar",
        "Radial bar",
        "Part to whole",
        "Concentric quantitative bars share a radial origin."
    ),
    (
        "polar-bar",
        "Polar bar",
        "Part to whole",
        "Category sectors contain stacked radial contributions."
    ),
    (
        "waffle",
        "Waffle",
        "Part to whole",
        "A cell grid shows occupied and remaining capacity."
    ),
    (
        "funnel",
        "Funnel",
        "Part to whole",
        "Connected stage profiles show a conversion journey."
    ),
    (
        "tree",
        "Tree",
        "Hierarchy",
        "A rooted hierarchy uses depth for position and explicit parent links."
    ),
    (
        "treemap",
        "Treemap",
        "Hierarchy",
        "Nested rectangles allocate area to a hierarchy of values."
    ),
    (
        "sunburst",
        "Sunburst",
        "Hierarchy",
        "Concentric partitions reveal hierarchy depth and leaf weight."
    ),
    (
        "circle-packing",
        "Circle packing",
        "Hierarchy",
        "Nested enclosing circles display weighted hierarchy leaves."
    ),
    (
        "icicle",
        "Icicle",
        "Hierarchy",
        "Rectangular partitions display hierarchy depth and relative weight."
    ),
    (
        "chord",
        "Chord",
        "Network",
        "Directed matrix values form weighted ribbons between groups."
    ),
    (
        "sankey",
        "Sankey",
        "Network",
        "Conserved flows split and join between processing stages."
    ),
    (
        "network",
        "Network",
        "Network",
        "A deterministic force layout connects a small application graph."
    ),
    (
        "choropleth",
        "Choropleth",
        "Geography",
        "175 non-Antarctic country regions from Nivo's world dataset, with synthetic demonstration values."
    ),
    (
        "geo-map",
        "Geographic map",
        "Geography",
        "175 non-Antarctic country outlines, a graticule and sized city symbols share one projection."
    ),
    (
        "heatmap",
        "Heat map",
        "Specialized",
        "A day and hour matrix uses a continuous color scale."
    ),
    (
        "boxplot",
        "Box plot",
        "Specialized",
        "Sample distributions show quartiles, whiskers and outliers."
    ),
    (
        "bullet",
        "Bullet",
        "Specialized",
        "Measures and targets are compared against qualitative ranges."
    ),
    (
        "calendar",
        "Calendar",
        "Specialized",
        "Daily observations use real Gregorian calendar positions."
    ),
    (
        "time-range",
        "Time range",
        "Specialized",
        "A compact civil date interval shows daily activity over a quarter."
    ),
    (
        "swarm",
        "Swarm plot",
        "Specialized",
        "Collision-aware observations retain their quantitative values."
    ),
    (
        "voronoi",
        "Voronoi",
        "Specialized",
        "Nearest-site cells partition a bounded observation plane."
    ),
    (
        "delaunay",
        "Delaunay",
        "Specialized",
        "Robust triangulation exposes neighborhood relationships."
    ),
    (
        "contour",
        "Contour",
        "Specialized",
        "Threshold isolines are extracted from a sampled scalar field."
    ),
    (
        "density",
        "Density",
        "Specialized",
        "A kernel density estimate turns point observations into contour bands."
    ),
];

fn phase(seed: u32) -> f64 {
    f64::from(seed % 17) * 0.47
}

/// Start an entrance from the same authored marks and geometry topology.
/// The browser can interrupt this spring with the next data set or layout.
pub(crate) fn entrance_scene(target: &Scene) -> Scene {
    let mut start = target.clone();
    for mark in &mut start.marks {
        if mark.series.is_empty() {
            // Keep decorative axes and grids visible. Some chart families
            // identify individual observations by label instead of series.
            if !mark.label.is_empty() {
                mark.opacity = 0.;
            }
            continue;
        }
        match &mut mark.geometry {
            Geometry::Circle { radius, .. } => *radius = 0.,
            Geometry::Rect {
                x,
                y,
                width,
                height,
            } => {
                *x += *width / 2.;
                *y += *height / 2.;
                *width = 0.;
                *height = 0.;
            }
            Geometry::Path(_) => {
                if let Some(clip) = &mut mark.clip {
                    clip.width = 0.;
                } else {
                    mark.opacity = 0.;
                }
            }
        }
    }
    start
}
fn config(kind: &str, width: f64, height: f64) -> ChartConfig {
    ChartConfig {
        width: width.clamp(220., 2400.),
        height: height.clamp(220., 1200.),
        margin: Margin {
            top: 24.,
            right: 28.,
            bottom: 42.,
            left: 48.,
        },
        title: format!("Kartis {kind}"),
        theme: Theme {
            colors: vec![
                Color(34, 126, 119, 255),
                Color(232, 137, 86, 255),
                Color(113, 127, 187, 255),
                Color(211, 173, 78, 255),
                Color(90, 165, 186, 255),
            ],
            text: Color(108, 117, 124, 255),
            grid: Color(233, 236, 235, 255),
            font_size: if width < 400. { 13. } else { 15. },
            ..Default::default()
        },
    }
}
fn key_json(key: &CategoryKey) -> Value {
    match key {
        CategoryKey::String(s) => json!(s),
        CategoryKey::Number(n) => json!(n),
    }
}
fn radial(seed: u32) -> Vec<RadialDatum> {
    ["Direct", "Search", "Community", "Other"]
        .into_iter()
        .enumerate()
        .map(|(i, id)| {
            RadialDatum::new(
                id,
                18. + i as f64 * 6. + (phase(seed) + i as f64).sin() * 8.,
            )
        })
        .collect()
}

/// The exact author input and core API call used to build a Scene.
pub struct Sample {
    pub scene: Scene,
    pub data: Value,
    pub code: &'static str,
}
#[path = "catalog/cart_controls.rs"]
mod cart_controls;
#[path = "catalog/rel_controls.rs"]
mod rel_controls;
#[path = "catalog/spec_controls.rs"]
mod spec_controls;

#[path = "catalog/appearance_controls.rs"]
mod appearance_controls;
#[path = "catalog/axes_controls.rs"]
mod axes_controls;
#[path = "catalog/browser_controls.rs"]
mod browser_controls;
pub struct Control {
    pub key: &'static str,
    pub label: &'static str,
    pub choices: &'static [(&'static str, &'static str)],
}
#[derive(Clone, Default)]
pub struct Settings(pub std::collections::BTreeMap<String, String>);
impl Settings {
    pub fn get(&self, key: &str) -> &str {
        self.0.get(key).map_or("", String::as_str)
    }
    pub fn validate(&self, kind: &str) -> Result<()> {
        for (key, value) in &self.0 {
            if !controls(kind).any(|control| {
                control.key == key && control.choices.iter().any(|choice| choice.0 == value)
            }) {
                return Err(ChartError(format!("Unknown {kind} option: {key}={value}")));
            }
        }
        Ok(())
    }
}
fn chart_controls(kind: &str) -> &'static [Control] {
    [
        cart_controls::controls(kind),
        rel_controls::controls(kind),
        spec_controls::controls(kind),
    ]
    .into_iter()
    .find(|c| !c.is_empty())
    .unwrap_or(&[])
}
pub fn control_groups(kind: &str) -> [(&'static str, &'static [Control]); 4] {
    [
        ("Layout", chart_controls(kind)),
        ("Axes and grid", axes_controls::controls(kind)),
        ("Colors and labels", appearance_controls::controls(kind)),
        ("Legend and tooltip", browser_controls::controls(kind)),
    ]
}
pub fn controls(kind: &str) -> impl Iterator<Item = &'static Control> {
    control_groups(kind)
        .into_iter()
        .flat_map(|(_, group)| group.iter())
}
pub fn control_code(kind: &str, settings: &Settings) -> String {
    [
        cart_controls::code(kind, settings),
        rel_controls::code(kind, settings),
        spec_controls::code(kind, settings),
        axes_controls::code(kind, settings),
        appearance_controls::code(kind, settings),
        browser_controls::code(kind, settings),
    ]
    .into_iter()
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join("\n")
}
/// Actual layout options available for the selected chart family.
pub fn variants(kind: &str) -> &'static [(&'static str, &'static str)] {
    match kind {
        "line" | "area" => &[
            ("monotone", "Monotone"),
            ("linear", "Linear"),
            ("step", "Step"),
            ("basis", "B-spline"),
            ("natural", "Natural spline"),
        ],
        "bar" => &[
            ("grouped", "Grouped"),
            ("stacked", "Stacked"),
            ("horizontal", "Horizontal"),
            ("horizontal-stacked", "Horizontal stacked"),
        ],
        "scatter" => &[
            ("medium", "9 px dots"),
            ("small", "5 px dots"),
            ("large", "16 px dots"),
        ],
        "radar" => &[
            ("default", "5 grid levels"),
            ("coarse", "3 grid levels"),
            ("rotated", "Rotate 36°"),
        ],
        "stream" => &[
            ("wiggle", "Wiggle offset"),
            ("silhouette", "Silhouette offset"),
            ("expand", "Normalize to 100%"),
            ("zero", "Zero baseline"),
        ],
        "bump" => &[
            ("smooth", "Smooth ranks"),
            ("linear", "Linear ranks"),
            ("no-plateau", "Smooth without plateaus"),
            ("large-points", "14 px rank points"),
            ("line-only", "Lines only"),
        ],
        "area-bump" => &[
            ("middle", "Centered bands"),
            ("linear", "Linear bands"),
            ("start", "Align to start"),
            ("spaced", "Separated bands"),
        ],
        "marimekko" => &[
            ("vertical", "Vertical"),
            ("horizontal", "Horizontal"),
            ("spaced", "6 px band gap"),
        ],
        "parallel-coordinates" => &[
            ("horizontal", "Horizontal"),
            ("vertical", "Vertical"),
            ("smooth", "Smooth paths"),
        ],
        "radial-bar" => &[
            ("full", "Full circle"),
            ("half", "Half circle"),
            ("ring", "Wide center"),
        ],
        "polar-bar" => &[
            ("stacked", "Stacked"),
            ("grouped", "Grouped"),
            ("area", "Area-scaled radii"),
        ],
        "waffle" => &[
            ("top", "Fill upward"),
            ("right", "Fill rightward"),
            ("bottom", "Fill downward"),
            ("left", "Fill leftward"),
        ],
        "funnel" => &[
            ("vertical", "Vertical · smooth"),
            ("horizontal", "Horizontal · smooth"),
            ("linear", "Vertical · linear"),
        ],
        "treemap" => &[
            ("binary", "Binary"),
            ("squarify", "Squarify"),
            ("slice", "Slice"),
            ("dice", "Dice"),
            ("slice-dice", "Slice and dice"),
        ],
        "sunburst" | "icicle" => &[
            ("partition", "Partition"),
            ("root", "Include root"),
            ("flush", "Zero padding"),
        ],
        "circle-packing" => &[
            ("front-chain", "Front chain"),
            ("shelf", "Shelf"),
            ("touching", "Zero padding"),
        ],
        "chord" => &[
            ("normal", "Standard gap"),
            ("joined", "No gap"),
            ("wide", "Wide gap"),
        ],
        "sankey" => &[
            ("normal", "Standard nodes"),
            ("thin", "Thin nodes"),
            ("wide", "Wide nodes"),
        ],
        "network" => &[
            ("normal", "Balanced links"),
            ("compact", "Short links"),
            ("long", "Long links"),
        ],
        "choropleth" => &[
            ("mercator", "Mercator"),
            ("equirectangular", "Equirectangular"),
            ("equal-earth", "Equal Earth"),
        ],
        "geo-map" => &[
            ("equirectangular", "Equirectangular"),
            ("mercator", "Mercator"),
            ("equal-earth", "Equal Earth"),
        ],
        "pie" => &[
            ("pie", "Pie"),
            ("donut", "Donut"),
            ("wide-ring", "Wide ring"),
            ("half-donut", "Half donut"),
        ],
        "tree" => &[
            ("right", "Rightward"),
            ("down", "Downward"),
            ("left", "Leftward"),
            ("up", "Upward"),
            ("radial", "Radial"),
        ],
        "heatmap" => &[
            ("rect", "Rectangular cells"),
            ("square", "Square cells"),
            ("spaced", "Wide cell gaps"),
        ],
        "boxplot" => &[
            ("vertical", "Vertical"),
            ("horizontal", "Horizontal"),
            ("extrema", "Min–max whiskers"),
        ],
        "bullet" => &[
            ("horizontal", "Horizontal"),
            ("vertical", "Vertical"),
            ("reverse", "Reversed scale"),
        ],
        "calendar" => &[
            ("year", "Full year"),
            ("monday", "Monday-first year"),
            ("vertical", "Vertical year"),
        ],
        "time-range" => &[
            ("monday", "Monday first"),
            ("sunday", "Sunday first"),
            ("vertical", "Vertical weeks"),
        ],
        "swarm" => &[
            ("horizontal", "Horizontal"),
            ("vertical", "Vertical"),
            ("compact", "Touching nodes"),
        ],
        "voronoi" => &[
            ("sites", "Cells and sites"),
            ("cells", "Cells only"),
            ("large", "Large sites"),
        ],
        "delaunay" => &[
            ("filled", "Filled triangles"),
            ("wire", "Wireframe"),
            ("labels", "Site labels"),
        ],
        "contour" => &[
            ("seven", "7 contour levels"),
            ("three", "3 contour levels"),
            ("fine", "14 contour levels"),
        ],
        "density" => &[
            ("medium", "16 px bandwidth"),
            ("local", "8 px bandwidth"),
            ("broad", "30 px bandwidth"),
        ],
        _ => &[],
    }
}
/// Builds and validates one real chart at the requested responsive size.
pub fn scene(kind: &str, width: f64, height: f64, seed: u32) -> Result<Scene> {
    scene_with_variant(kind, width, height, seed, "")
}
pub fn scene_with_variant(
    kind: &str,
    width: f64,
    height: f64,
    seed: u32,
    variant: &str,
) -> Result<Scene> {
    sample(kind, width, height, seed, variant).map(|sample| sample.scene)
}
pub fn sample(kind: &str, width: f64, height: f64, seed: u32, variant: &str) -> Result<Sample> {
    sample_with_spacing(
        kind,
        width,
        height,
        seed,
        variant,
        100,
        BumpControls::default(),
    )
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BumpControls {
    pub line_width: f64,
    pub end_labels: bool,
}
impl Default for BumpControls {
    fn default() -> Self {
        Self {
            line_width: 2.,
            end_labels: true,
        }
    }
}
pub fn sample_with_spacing(
    kind: &str,
    width: f64,
    height: f64,
    seed: u32,
    variant: &str,
    spacing: u32,
    bump: BumpControls,
) -> Result<Sample> {
    configured_sample(
        kind,
        (width, height),
        seed,
        variant,
        spacing,
        bump,
        &Settings::default(),
    )
}
pub fn configured_sample(
    kind: &str,
    size: (f64, f64),
    seed: u32,
    variant: &str,
    spacing: u32,
    bump: BumpControls,
    settings: &Settings,
) -> Result<Sample> {
    let (width, height) = size;
    settings.validate(kind)?;
    if !(50..=150).contains(&spacing) {
        return Err(ChartError(
            "Plot spacing must be between 50 and 150 percent".into(),
        ));
    }
    let mut c = config(kind, width, height);
    let factor = f64::from(spacing) / 100.;
    c.margin.left *= factor;
    c.margin.right *= factor;
    c.margin.top *= factor;
    c.margin.bottom *= factor;
    cartesian::reserve_rank_labels(&mut c, kind, settings, bump);
    let variants = variants(kind);
    let variant = if variant.is_empty() {
        variants.first().map_or("", |v| v.0)
    } else {
        variant
    };
    if !variants.is_empty() && !variants.iter().any(|v| v.0 == variant) {
        return Err(ChartError(format!("Unknown {kind} variant: {variant}")));
    }
    let mut sample = match kind {
        "line"
        | "area"
        | "bar"
        | "scatter"
        | "radar"
        | "stream"
        | "bump"
        | "area-bump"
        | "marimekko"
        | "parallel-coordinates"
        | "heatmap"
        | "boxplot"
        | "bullet"
        | "calendar"
        | "time-range"
        | "swarm" => cartesian::sample(kind, &c, seed, variant, bump, settings)?,
        "tree" | "treemap" | "sunburst" | "circle-packing" | "icicle" | "chord" | "sankey"
        | "network" => relational::sample(kind, &c, seed, variant, settings)?,
        "choropleth" | "geo-map" | "voronoi" | "delaunay" | "contour" | "density" => {
            spatial::sample(kind, &c, seed, variant, settings)?
        }
        "pie" | "radial-bar" | "polar-bar" | "waffle" | "funnel" => {
            let source = radial(seed);
            let data = json!(
                source
                    .iter()
                    .map(|d| json!({"id":d.id,"label":d.label,"value":d.value}))
                    .collect::<Vec<_>>()
            );
            use rel_controls::active;
            let (scene, data, code) = match kind {
                "pie" => (
                    pie(
                        &c,
                        &source,
                        &PieOptions {
                            inner_radius: match variant {
                                "donut" | "half-donut" => 0.65,
                                "wide-ring" => 0.35,
                                _ => 0.,
                            },
                            start_angle: if variant == "half-donut" {
                                -std::f64::consts::FRAC_PI_2
                            } else {
                                0.
                            },
                            end_angle: if variant == "half-donut" {
                                std::f64::consts::FRAC_PI_2
                            } else {
                                std::f64::consts::TAU
                            },
                            pad_angle: if active(settings, "pad-angle") {
                                0.10
                            } else {
                                0.025
                            },
                            labels: active(settings, "arc-labels"),
                            ..Default::default()
                        },
                    )?,
                    data,
                    match variant {
                        "donut" => {
                            "pie(&config, &data, &PieOptions { inner_radius: 0.65, ..Default::default() })?"
                        }
                        "half-donut" => {
                            "pie(&config, &data, &PieOptions { inner_radius: 0.65, start_angle: -std::f64::consts::FRAC_PI_2, end_angle: std::f64::consts::FRAC_PI_2, ..Default::default() })?"
                        }
                        "wide-ring" => {
                            "pie(&config, &data, &PieOptions { inner_radius: 0.35, ..Default::default() })?"
                        }
                        _ => {
                            "pie(&config, &data, &PieOptions { inner_radius: 0., ..Default::default() })?"
                        }
                    },
                ),
                "radial-bar" => {
                    let (options, code) = match variant {
                        "half" => (
                            RadialBarOptions {
                                start_angle: -std::f64::consts::FRAC_PI_2,
                                end_angle: std::f64::consts::FRAC_PI_2,
                                ..Default::default()
                            },
                            "radial_bar(&config, &data, &RadialBarOptions { start_angle: -std::f64::consts::FRAC_PI_2, end_angle: std::f64::consts::FRAC_PI_2, ..Default::default() })?",
                        ),
                        "ring" => (
                            RadialBarOptions {
                                inner_radius: 0.55,
                                padding: 0.5,
                                ..Default::default()
                            },
                            "radial_bar(&config, &data, &RadialBarOptions { inner_radius: 0.55, padding: 0.5, ..Default::default() })?",
                        ),
                        _ => (
                            RadialBarOptions::default(),
                            "radial_bar(&config, &data, &RadialBarOptions::default())?",
                        ),
                    };
                    let options = RadialBarOptions {
                        tracks: !active(settings, "tracks"),
                        max_value: active(settings, "maximum").then_some(60.),
                        ..options
                    };
                    (radial_bar(&c, &source, &options)?, data, code)
                }
                "polar-bar" => {
                    let categories = cartesian::categories();
                    let source = cartesian::bars(seed);
                    let data = json!({"categories":categories,"series":source.iter().map(|d|json!({"id":d.id,"values":d.values})).collect::<Vec<_>>()});
                    let (options, code) = match variant {
                        "grouped" => (
                            PolarBarOptions {
                                layout: BarLayout::Grouped,
                                ..Default::default()
                            },
                            "polar_bar(&config, &categories, &series, &PolarBarOptions { layout: BarLayout::Grouped, ..Default::default() })?",
                        ),
                        "area" => (
                            PolarBarOptions {
                                area_scale: true,
                                ..Default::default()
                            },
                            "polar_bar(&config, &categories, &series, &PolarBarOptions { area_scale: true, ..Default::default() })?",
                        ),
                        _ => (
                            PolarBarOptions::default(),
                            "polar_bar(&config, &categories, &series, &PolarBarOptions::default())?",
                        ),
                    };
                    let options = PolarBarOptions {
                        pad_angle: if active(settings, "pad-angle") {
                            0.12
                        } else {
                            options.pad_angle
                        },
                        inner_radius: if active(settings, "inner-radius") {
                            0.4
                        } else {
                            options.inner_radius
                        },
                        ..options
                    };
                    (polar_bar(&c, &categories, &source, &options)?, data, code)
                }
                "waffle" => {
                    let (direction, code) = match variant {
                        "right" => (
                            WaffleFillDirection::Right,
                            "waffle_layout(&config, &data, &WaffleLayoutOptions { rows: 6, columns: 10, total: Some(150.), direction: WaffleFillDirection::Right, ..Default::default() })?.scene",
                        ),
                        "bottom" => (
                            WaffleFillDirection::Bottom,
                            "waffle_layout(&config, &data, &WaffleLayoutOptions { rows: 6, columns: 10, total: Some(150.), direction: WaffleFillDirection::Bottom, ..Default::default() })?.scene",
                        ),
                        "left" => (
                            WaffleFillDirection::Left,
                            "waffle_layout(&config, &data, &WaffleLayoutOptions { rows: 6, columns: 10, total: Some(150.), direction: WaffleFillDirection::Left, ..Default::default() })?.scene",
                        ),
                        _ => (
                            WaffleFillDirection::Top,
                            "waffle_layout(&config, &data, &WaffleLayoutOptions { rows: 6, columns: 10, total: Some(150.), direction: WaffleFillDirection::Top, ..Default::default() })?.scene",
                        ),
                    };
                    (
                        waffle_layout(
                            &c,
                            &source,
                            &WaffleLayoutOptions {
                                rows: if active(settings, "grid") { 8 } else { 6 },
                                columns: if active(settings, "grid") { 12 } else { 10 },
                                direction,
                                total: Some(150.),
                                padding: if active(settings, "cell-gap") { 1. } else { 4. },
                                corner_radius: 3.,
                                empty_color: Color(235, 239, 237, 255),
                                ..Default::default()
                            },
                        )?
                        .scene,
                        data,
                        code,
                    )
                }
                _ => {
                    let source = [
                        FunnelDatum::new("Reached", 100.),
                        FunnelDatum::new("Qualified", 74. + phase(seed).sin() * 9.),
                        FunnelDatum::new("Evaluated", 46. + phase(seed).cos() * 8.),
                        FunnelDatum::new("Converted", 22. + phase(seed).sin() * 5.),
                    ];
                    let data = json!(
                        source
                            .iter()
                            .map(|d| json!({"id":key_json(&d.id),"label":d.label,"value":d.value}))
                            .collect::<Vec<_>>()
                    );
                    let (direction, interpolation, code) = match variant {
                        "horizontal" => (
                            FunnelDirection::Horizontal,
                            FunnelInterpolation::Smooth,
                            "funnel_layout(&config, &stages, &FunnelLayoutOptions { direction: FunnelDirection::Horizontal, interpolation: FunnelInterpolation::Smooth, spacing: 12., ..Default::default() })?.scene",
                        ),
                        "linear" => (
                            FunnelDirection::Vertical,
                            FunnelInterpolation::Linear,
                            "funnel_layout(&config, &stages, &FunnelLayoutOptions { direction: FunnelDirection::Vertical, interpolation: FunnelInterpolation::Linear, spacing: 12., ..Default::default() })?.scene",
                        ),
                        _ => (
                            FunnelDirection::Vertical,
                            FunnelInterpolation::Smooth,
                            "funnel_layout(&config, &stages, &FunnelLayoutOptions { direction: FunnelDirection::Vertical, interpolation: FunnelInterpolation::Smooth, spacing: 12., ..Default::default() })?.scene",
                        ),
                    };
                    (
                        funnel_layout(
                            &c,
                            &source,
                            &FunnelLayoutOptions {
                                direction,
                                interpolation,
                                spacing: if active(settings, "stage-gap") {
                                    3.
                                } else {
                                    12.
                                },
                                fill_opacity: if active(settings, "stage-opacity") {
                                    0.55
                                } else {
                                    1.
                                },
                                ..Default::default()
                            },
                        )?
                        .scene,
                        data,
                        code,
                    )
                }
            };
            Sample { scene, data, code }
        }
        _ => return Err(ChartError(format!("Unknown catalog chart: {kind}"))),
    };
    axes_controls::apply(kind, settings, &mut sample.scene)?;
    if kind == "heatmap" && c.inner_width() < c.theme.font_size * 8. * 3.2 {
        for label in &mut sample.scene.labels {
            if let Some(hour) = label.text.strip_suffix(":00") {
                label.text = hour.to_owned();
            }
        }
    }
    appearance_controls::apply(kind, settings, &mut sample.scene)?;
    sample.scene.validate()?;
    Ok(sample)
}
