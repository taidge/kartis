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
            font_size: 11.,
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
/// Actual layout options available for the selected chart family.
pub fn variants(kind: &str) -> &'static [(&'static str, &'static str)] {
    match kind {
        "line" | "area" => &[
            ("monotone", "Monotone"),
            ("linear", "Linear"),
            ("step", "Step"),
        ],
        "bar" => &[
            ("grouped", "Grouped"),
            ("stacked", "Stacked"),
            ("horizontal", "Horizontal"),
        ],
        "pie" => &[
            ("pie", "Pie"),
            ("donut", "Donut"),
            ("wide-ring", "Wide ring"),
        ],
        "tree" => &[
            ("right", "Rightward"),
            ("down", "Downward"),
            ("radial", "Radial"),
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
    let c = config(kind, width, height);
    let variants = variants(kind);
    let variant = if variant.is_empty() {
        variants.first().map_or("", |v| v.0)
    } else {
        variant
    };
    if !variants.is_empty() && !variants.iter().any(|v| v.0 == variant) {
        return Err(ChartError(format!("Unknown {kind} variant: {variant}")));
    }
    let sample = match kind {
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
        | "swarm" => cartesian::sample(kind, &c, seed, variant)?,
        "tree" | "treemap" | "sunburst" | "circle-packing" | "icicle" | "chord" | "sankey"
        | "network" => relational::sample(kind, &c, seed, variant)?,
        "choropleth" | "geo-map" | "voronoi" | "delaunay" | "contour" | "density" => {
            spatial::sample(kind, &c, seed)?
        }
        "pie" | "radial-bar" | "polar-bar" | "waffle" | "funnel" => {
            let source = radial(seed);
            let data = json!(
                source
                    .iter()
                    .map(|d| json!({"id":d.id,"label":d.label,"value":d.value}))
                    .collect::<Vec<_>>()
            );
            let (scene, data, code) = match kind {
                "pie" => (
                    pie(
                        &c,
                        &source,
                        &PieOptions {
                            inner_radius: match variant {
                                "donut" => 0.65,
                                "wide-ring" => 0.35,
                                _ => 0.,
                            },
                            pad_angle: 0.025,
                            labels: false,
                            ..Default::default()
                        },
                    )?,
                    data,
                    match variant {
                        "donut" => {
                            "pie(&config, &data, &PieOptions { inner_radius: 0.65, ..Default::default() })?"
                        }
                        "wide-ring" => {
                            "pie(&config, &data, &PieOptions { inner_radius: 0.35, ..Default::default() })?"
                        }
                        _ => {
                            "pie(&config, &data, &PieOptions { inner_radius: 0., ..Default::default() })?"
                        }
                    },
                ),
                "radial-bar" => (
                    radial_bar(&c, &source, &RadialBarOptions::default())?,
                    data,
                    "radial_bar(&config, &data, &RadialBarOptions::default())?",
                ),
                "polar-bar" => {
                    let categories = cartesian::categories();
                    let source = cartesian::bars(seed);
                    let data = json!({"categories":categories,"series":source.iter().map(|d|json!({"id":d.id,"values":d.values})).collect::<Vec<_>>()});
                    (
                        polar_bar(&c, &categories, &source, &PolarBarOptions::default())?,
                        data,
                        "polar_bar(&config, &categories, &series, &PolarBarOptions::default())?",
                    )
                }
                "waffle" => (
                    waffle_layout(
                        &c,
                        &source,
                        &WaffleLayoutOptions {
                            rows: 6,
                            columns: 10,
                            total: Some(150.),
                            padding: 4.,
                            corner_radius: 3.,
                            empty_color: Color(235, 239, 237, 255),
                            ..Default::default()
                        },
                    )?
                    .scene,
                    data,
                    "waffle_layout(&config, &data, &WaffleLayoutOptions { rows: 6, columns: 10, total: Some(150.), ..Default::default() })?.scene",
                ),
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
                    (
                        funnel_layout(
                            &c,
                            &source,
                            &FunnelLayoutOptions {
                                spacing: 12.,
                                ..Default::default()
                            },
                        )?
                        .scene,
                        data,
                        "funnel_layout(&config, &stages, &FunnelLayoutOptions { spacing: 12., ..Default::default() })?.scene",
                    )
                }
            };
            Sample { scene, data, code }
        }
        _ => return Err(ChartError(format!("Unknown catalog chart: {kind}"))),
    };
    sample.scene.validate()?;
    Ok(sample)
}
