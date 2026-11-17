//! Independent full Line/Scatter consumers of original d3-zoom rescaleX/Y.
//! The companion original producer retains every native true turn and all IEEE
//! values. Domains alone are insufficient: actual sites, point marks and Line
//! stroke vertices are checked against original D3 forward probes after reflow.
#![cfg(feature = "geojson")]

use glory::render::PathCommand;
use kartis::*;
use serde_json::Value;

type Audit<T> = std::result::Result<T, String>;
fn arr(v: &Value) -> &[Value] {
    v.as_array().unwrap()
}
fn num(v: &Value) -> f64 {
    v.as_f64()
        .unwrap_or_else(|| v.as_str().unwrap().parse().unwrap())
}
fn vals(v: &Value) -> Vec<f64> {
    arr(v).iter().map(num).collect()
}
fn require(ok: bool, message: impl Into<String>) -> Audit<()> {
    if ok { Ok(()) } else { Err(message.into()) }
}
fn near(a: f64, b: f64) -> Audit<()> {
    require(
        a == b || (a - b).abs() <= 2e-12 * a.abs().max(b.abs()).max(1.),
        format!("native knot {a:?} != {b:?}"),
    )
}
fn knots(a: &[f64], b: &[f64]) -> Audit<()> {
    require(a.len() == b.len(), "full-knot length changed")?;
    for (&a, &b) in a.iter().zip(b) {
        near(a, b)?;
    }
    Ok(())
}
fn cfg() -> ChartConfig {
    ChartConfig {
        width: 420.5,
        height: 300.5,
        margin: Margin {
            left: 60.,
            right: 20.,
            top: 20.,
            bottom: 40.,
        },
        ..Default::default()
    }
}
fn kind(c: &Value) -> ScaleKind {
    match c["kind"].as_str().unwrap() {
        "linear" => ScaleKind::Linear,
        "log" => ScaleKind::Log,
        "symlog" => ScaleKind::SymLog {
            constant: num(&c["parameter"]),
        },
        "pow" => ScaleKind::Pow {
            exponent: num(&c["parameter"]),
        },
        "sqrt" => ScaleKind::Sqrt,
        "utc" => ScaleKind::Utc,
        _ => unreachable!(),
    }
}
fn profile(c: &Value) -> PiecewiseScaleOptions {
    PiecewiseScaleOptions {
        domain: vals(&c["source"]),
        range: vals(&c["rangeFractions"]),
        continuous: ContinuousScaleOptions {
            kind: kind(c),
            log_base: if c["kind"] == "log" {
                num(&c["parameter"])
            } else {
                10.
            },
            nice: match c["nice"].as_i64().unwrap() {
                -1 => ScaleNice::Off,
                0 => ScaleNice::Default,
                n => ScaleNice::Count(n as usize),
            },
            reverse: c["reverse"].as_bool().unwrap(),
            round: c["round"].as_bool().unwrap(),
            ..Default::default()
        },
    }
}
fn axes() -> CartesianAxes {
    let mut a = CartesianAxes::default();
    a.x.visible = false;
    a.x.grid = false;
    a.y.visible = false;
    a.y.grid = false;
    a
}
fn ordinary(n: usize) -> ScatterAxisScale {
    ScatterAxisScale::Continuous {
        options: ContinuousScaleOptions::default(),
        min: ScatterBound::Fixed(0.),
        max: ScatterBound::Fixed(n as f64 + 1.),
    }
}
#[derive(Clone, Copy, Debug)]
enum Consumer {
    Line,
    Scatter,
}
enum Frame {
    Line(Box<PiecewiseLineLayout>),
    Scatter(Box<ScatterLayout>),
}
struct Data {
    line: Vec<Series>,
    scatter: Vec<ScatterSeries>,
    probes: Vec<Value>,
}
fn data(c: &Value) -> Data {
    let vertical = c["axis"] == "y";
    let probes = arr(&c["native"]["probes"])
        .iter()
        .filter(|p| p["checkedInput"].as_bool().unwrap())
        .cloned()
        .collect::<Vec<_>>();
    let pairs = probes
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let raw = num(&p["input"]);
            let other = i as f64 + 1.;
            if vertical { [other, raw] } else { [raw, other] }
        })
        .collect::<Vec<_>>();
    Data {
        line: vec![Series::new(
            "source",
            pairs.iter().map(|p| Point::new(p[0], p[1])).collect(),
        )],
        scatter: vec![ScatterSeries::new(
            "source",
            pairs
                .iter()
                .enumerate()
                .map(|(i, p)| ScatterDatum::new(format!("datum-{i}"), p[0], p[1]))
                .collect(),
        )],
        probes,
    }
}
fn build(mode: Consumer, c: &Value, d: &Data) -> Audit<Frame> {
    let vertical = c["axis"] == "y";
    let p = profile(c);
    let n = d.probes.len();
    match mode {
        Consumer::Line => line_layout_with_piecewise_scale_options(
            &cfg(),
            &d.line,
            &LineOptions {
                x_domain: vertical.then_some([0., n as f64 + 1.]),
                y_domain: (!vertical).then_some([0., n as f64 + 1.]),
                axes: axes(),
                ..Default::default()
            },
            &LineStyleOptions::default(),
            &PiecewiseLineScaleOptions {
                x: (!vertical).then(|| p.clone()),
                y: vertical.then_some(p),
                ..Default::default()
            },
        )
        .map(|l| Frame::Line(Box::new(l)))
        .map_err(|e| format!("baseline Line: {e}")),
        Consumer::Scatter => scatter_layout(
            &cfg(),
            &d.scatter,
            &ScatterLayoutOptions {
                scales: ScatterScaleOptions {
                    x: if vertical {
                        ordinary(n)
                    } else {
                        ScatterAxisScale::Piecewise { options: p.clone() }
                    },
                    y: if vertical {
                        ScatterAxisScale::Piecewise { options: p }
                    } else {
                        ordinary(n)
                    },
                },
                axes: ScatterAxes {
                    top: None,
                    right: None,
                    bottom: None,
                    left: None,
                },
                ..Default::default()
            },
        )
        .map(|l| Frame::Scatter(Box::new(l)))
        .map_err(|e| format!("baseline Scatter: {e}")),
    }
}
impl Frame {
    fn scene(&self) -> &Scene {
        match self {
            Self::Line(l) => &l.layout.scene,
            Self::Scatter(l) => &l.scene,
        }
    }
    fn primary(&self, vertical: bool) -> &ChartScale {
        match self {
            Self::Line(l) => {
                if vertical {
                    l.y_scale.chart_scale()
                } else {
                    l.x_scale.chart_scale()
                }
            }
            Self::Scatter(l) => {
                if vertical {
                    l.y_scale.chart_scale().unwrap()
                } else {
                    l.x_scale.chart_scale().unwrap()
                }
            }
        }
    }
    fn controller(&self) -> kartis::Result<DomainViewport> {
        match self {
            Self::Line(l) => l.viewport_controller(Default::default()),
            Self::Scatter(l) => l.viewport_controller(Default::default()),
        }
    }
    fn reflow(
        &self,
        c: &Value,
        d: &Data,
        domains: ResolvedDomains,
        stops: &PiecewiseViewportDomains,
    ) -> kartis::Result<Self> {
        match self {
            Self::Line(l) => {
                let scales = l.viewport_scale_options(stops)?;
                line_layout_with_piecewise_scale_options(
                    &cfg(),
                    &d.line,
                    &LineOptions {
                        x_domain: Some(domains.x),
                        y_domain: domains.left,
                        axes: axes(),
                        ..Default::default()
                    },
                    &LineStyleOptions::default(),
                    &scales,
                )
                .map(|v| Self::Line(Box::new(v)))
            }
            Self::Scatter(l) => {
                let scales = l.viewport_scale_options_with_piecewise(domains, stops)?;
                scatter_layout(
                    &cfg(),
                    &d.scatter,
                    &ScatterLayoutOptions {
                        scales,
                        axes: ScatterAxes {
                            top: None,
                            right: None,
                            bottom: None,
                            left: None,
                        },
                        ..Default::default()
                    },
                )
                .map(|v| Self::Scatter(Box::new(v)))
            }
        }
        .map_err(|e| ChartError(format!("{} reflow: {e}", c["name"])))
    }
    fn sites(&self) -> Vec<(usize, [f64; 2], &Mark)> {
        match self {
            Self::Line(l) => l
                .layout
                .sites
                .iter()
                .map(|s| {
                    (
                        s.source_index,
                        s.position,
                        l.layout
                            .scene
                            .marks
                            .iter()
                            .find(|m| m.id == s.mark_id)
                            .unwrap(),
                    )
                })
                .collect(),
            Self::Scatter(l) => l
                .sites
                .iter()
                .map(|s| (s.source_index, s.position, &l.scene.marks[s.mark_index]))
                .collect(),
        }
    }
    fn validate(&self) -> Audit<()> {
        self.scene().validate().map_err(|e| e.to_string())?;
        match self {
            Self::Line(l) => l.interaction_binding().validate(self.scene()),
            Self::Scatter(l) => l.interaction_binding().validate(self.scene()),
        }
        .map_err(|e| e.to_string())
    }
}
#[derive(Default, Debug)]
struct Counts {
    baseline_constant: usize,
    accepted: usize,
    native_reject: usize,
    geometry_reject: usize,
    collapsed: usize,
    sites: usize,
    line_boundary: usize,
    rounded_ulp: usize,
    resets: usize,
}
fn coordinate(
    actual: f64,
    expected: f64,
    raw: f64,
    native_raw: f64,
    round: bool,
    counts: &mut Counts,
) -> Audit<()> {
    if (actual - expected).abs() <= 1e-8 {
        return Ok(());
    }
    let half = native_raw.floor() + 0.5;
    let ulp = f64::from_bits(half.abs().to_bits() + 1) - half.abs();
    if round
        && (actual - expected).abs() == 1.
        && (raw - half).abs() <= ulp
        && (native_raw - half).abs() <= ulp
        && actual == (raw + 0.5).floor()
        && expected == (native_raw + 0.5).floor()
    {
        counts.rounded_ulp += 1;
        return Ok(());
    }
    Err(format!(
        "native pixel {actual:?} != {expected:?}; actual/native raw {raw:?}/{native_raw:?}"
    ))
}
fn geometry(frame: &Frame, c: &Value, d: &Data, viewport: bool, counts: &mut Counts) -> Audit<()> {
    let vertical = c["axis"] == "y";
    let scale = frame.primary(vertical);
    let offset = if vertical {
        cfg().margin.top
    } else {
        cfg().margin.left
    };
    let extent = num(&c["extent"]);
    let raw = ResolvedContinuousScale::piecewise(&PiecewiseScaleOptions {
        domain: scale.domain_stops().to_vec(),
        range: scale.range_stops().to_vec(),
        continuous: ContinuousScaleOptions {
            nice: ScaleNice::Off,
            reverse: false,
            round: false,
            ..profile(c).continuous
        },
    })
    .map_err(|e| format!("raw geometry scale: {e}"))?;
    let sites = frame.sites();
    let mut seen = std::collections::BTreeSet::new();
    for (index, p, mark) in &sites {
        require(
            *index < d.probes.len() && seen.insert(*index),
            "invented/repeated source index",
        )?;
        let probe = &d.probes[*index];
        let expected = num(&probe[if viewport {
            "viewportPixel"
        } else {
            "baselinePixel"
        }]) + offset;
        let native_raw = num(&probe[if viewport {
            "viewportUnrounded"
        } else {
            "baselineUnrounded"
        }]) + offset;
        let main = usize::from(vertical);
        coordinate(
            p[main],
            expected,
            raw.map(num(&probe["input"])),
            native_raw,
            c["round"].as_bool().unwrap(),
            counts,
        )
        .map_err(|e| format!("source {index}: {e}"))?;
        let fraction = (*index as f64 + 1.) / (d.probes.len() as f64 + 1.);
        let peer = if vertical {
            cfg().margin.left + cfg().inner_width() * fraction
        } else {
            cfg().margin.top + cfg().inner_height() * (1. - fraction)
        };
        require(
            (p[1 - main] - peer).abs() <= 1e-8,
            format!("ordinary peer axis drift: {p:?} vs {peer}"),
        )?;
        match mark.geometry {
            Geometry::Circle { x, y, .. } => {
                require(
                    (x - p[0]).abs() <= 1e-8 && (y - p[1]).abs() <= 1e-8,
                    "point mark differs from its native-checked site",
                )?;
            }
            _ => return Err("default point is not actual circle geometry".into()),
        }
        counts.sites += 1;
    }
    if let Frame::Line(l) = frame {
        let native_stops = vals(&c["native"][if viewport { "rescaled" } else { "display" }]);
        let repeated = native_stops.windows(2).any(|p| p[0] == p[1]);
        let low = native_stops[0].min(*native_stops.last().unwrap());
        let high = native_stops[0].max(*native_stops.last().unwrap());
        for (index, probe) in d.probes.iter().enumerate() {
            let p = num(&probe[if viewport {
                "viewportPixel"
            } else {
                "baselinePixel"
            }]);
            let boundary = p.abs().min((p - extent).abs());
            if repeated && boundary <= 1e-8 {
                counts.line_boundary += 1;
                continue;
            }
            // Ordinary strictly monotonic displayed domains keep their legacy
            // domain clipping, including an endpoint rounded half a pixel out
            // of the plot. The new repeated-domain branch clips projected ink.
            let retained = if repeated {
                p > 0. && p < extent
            } else {
                let input = num(&probe["input"]);
                input >= low && input <= high
            };
            require(
                seen.contains(&index) == retained,
                format!("Line source {index} retention differs from native projected pixel {p}"),
            )?;
        }
        let vertices = l
            .layout
            .scene
            .marks
            .iter()
            .filter(|m| m.id.starts_with("line-stroke:"))
            .flat_map(|m| match &m.geometry {
                Geometry::Path(path) => path
                    .iter()
                    .filter_map(|c| match c {
                        PathCommand::Move(p) | PathCommand::Line(p) => Some([p.x, p.y]),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
                _ => vec![],
            })
            .collect::<Vec<_>>();
        for (_, p, _) in &sites {
            require(
                vertices
                    .iter()
                    .any(|q| (p[0] - q[0]).hypot(p[1] - q[1]) <= 1e-8),
                format!("native-checked point missing from real linear stroke vertices: {p:?}"),
            )?;
        }
    } else {
        require(
            sites.len() == d.probes.len(),
            "Scatter must keep every checked finite source center",
        )?;
    }
    frame.validate()
}
fn request(c: &Value) -> ViewportRequest {
    let vertical = c["axis"] == "y";
    let k = 1. / num(&c["window"]["span"]);
    let offset = -num(&c["window"]["start"]) * num(&c["extent"]) * k;
    ViewportRequest::TransformXY {
        transform: ViewportTransform {
            scale_x: if vertical { 1. } else { k },
            scale_y: if vertical { k } else { 1. },
            translate_x: if vertical { 0. } else { offset },
            translate_y: if vertical { offset } else { 0. },
        },
        axes: if vertical {
            ViewportSelection::Y
        } else {
            ViewportSelection::X
        },
    }
}
fn primary_stops(stops: &PiecewiseViewportDomains, vertical: bool) -> &[f64] {
    if vertical {
        stops.left.as_ref().unwrap()
    } else {
        stops.x.as_ref().unwrap()
    }
}
fn run(mode: Consumer, c: &Value, counts: &mut Counts) -> Audit<()> {
    let d = data(c);
    let original_line = d.line.clone();
    let original_scatter = d.scatter.clone();
    let frame = build(mode, c, &d)?;
    let scene = frame.scene().clone();
    let vertical = c["axis"] == "y";
    let display = vals(&c["native"]["display"]);
    knots(frame.primary(vertical).domain_stops(), &display)?;
    geometry(&frame, c, &d, false, counts).map_err(|e| format!("baseline geometry: {e}"))?;
    if display.first() == display.last() {
        require(
            frame.controller().is_err(),
            "constant display baseline must not fabricate a navigable original axis",
        )?;
        counts.baseline_constant += 1;
        return Ok(());
    }
    let mut controller = frame
        .controller()
        .map_err(|e| format!("original controller: {e}"))?;
    let original = controller.clone();
    let request = request(c);
    let preview = controller.resolve_piecewise(request);
    require(
        controller == original,
        "resolve mutated original controller",
    )?;
    let native = vals(&c["native"]["rescaled"]);
    let rejected = c["native"]["rescaledCheckedProfile"] != "accept";
    if rejected {
        require(
            preview.is_err(),
            format!(
                "native {} request was accepted",
                c["native"]["rescaledCheckedProfile"]
            ),
        )?;
        require(
            controller.apply(request).is_err(),
            "invalid native request apply was accepted",
        )?;
        require(
            controller == original,
            "rejected native request changed controller",
        )?;
        counts.native_reject += 1;
    } else {
        let preview = preview.map_err(|e| {
            format!("native checked-valid resolve rejected: {e}; native {native:?}")
        })?;
        let actual = primary_stops(&preview, vertical);
        knots(actual, &native).map_err(|e| format!("viewport full stops actual {actual:?}, original {native:?}, request {request:?}: {e}"))?;
        if c["native"]["rescaledConstant"].as_bool().unwrap() {
            require(
                actual.iter().all(|v| *v == actual[0]),
                format!(
                    "native constant window {native:?} became an ULP span: {actual:?}; request {request:?}"
                ),
            )?;
        }
        let domains = controller
            .resolve(request)
            .map_err(|e| format!("endpoint preview: {e}"))?;
        let reflow = frame.reflow(c, &d, domains, &preview);
        let scatter_budget = matches!(mode, Consumer::Scatter)
            && d.probes.iter().any(|p| {
                let v = num(&p["viewportPixel"])
                    + if vertical {
                        cfg().margin.top
                    } else {
                        cfg().margin.left
                    };
                !v.is_finite() || v.abs() > 1_000_000.
            });
        if scatter_budget {
            require(
                reflow.is_err(),
                "native Scatter geometry exceeds Scene coordinate budget yet reflow accepted",
            )?;
            require(
                controller == original,
                "failed proposal reflow committed controller",
            )?;
            counts.geometry_reject += 1;
        } else {
            let reflow = reflow.map_err(|e| format!("native checked-valid full reflow: {e}"))?;
            knots(reflow.primary(vertical).domain_stops(), &native)?;
            geometry(&reflow, c, &d, true, counts)
                .map_err(|e| format!("viewport geometry, actual full stops {actual:?}, original {native:?}, request {request:?}: {e}"))?;
            controller
                .apply(request)
                .map_err(|e| format!("accepted proposal apply: {e}"))?;
            require(
                controller.piecewise_domains().map_err(|e| e.to_string())? == preview,
                "committed full knots differ from preview",
            )?;
            counts.accepted += 1;
            if c["native"]["rescaledConstant"].as_bool().unwrap() {
                counts.collapsed += 1;
                require(
                    reflow.controller().is_err(),
                    "collapsed reflow fabricated a replacement original controller",
                )?;
            }
            controller
                .apply(ViewportRequest::Reset)
                .map_err(|e| format!("same original controller Reset: {e}"))?;
            require(
                controller.piecewise_domains().map_err(|e| e.to_string())?
                    == original.piecewise_domains().map_err(|e| e.to_string())?,
                "Reset lost original full knots",
            )?;
            let reset = frame
                .reflow(
                    c,
                    &d,
                    controller.domains(),
                    &controller.piecewise_domains().map_err(|e| e.to_string())?,
                )
                .map_err(|e| format!("Reset full reflow: {e}"))?;
            geometry(&reset, c, &d, false, counts).map_err(|e| format!("Reset geometry: {e}"))?;
            counts.resets += 1;
        }
    }
    require(frame.scene() == &scene, "proposal changed baseline Scene")?;
    require(
        d.line == original_line && d.scatter == original_scatter,
        "proposal mutated source observations",
    )?;
    Ok(())
}

#[test]
fn original_native_all_knots_flow_through_real_line_and_scatter_viewport_proposals() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/piecewise-repeated/values.json")).unwrap();
    let cases = arr(&fixture["viewportCases"]);
    assert_eq!(cases.len(), 4128);
    let mut failures = vec![];
    for mode in [Consumer::Line, Consumer::Scatter] {
        let mut counts = Counts::default();
        for case in cases {
            if let Err(error) = run(mode, case, &mut counts) {
                let message = format!("{mode:?} {}: {error}", case["name"]);
                eprintln!("NATIVE VIEWPORT DIFFERENCE {message}");
                failures.push(message);
            }
        }
        eprintln!("Original repeated viewport {mode:?}: {counts:?}");
        if failures.is_empty() {
            assert_eq!(counts.baseline_constant, 1216);
            assert_eq!(counts.native_reject, 8);
            assert_eq!(counts.accepted + counts.geometry_reject, 2904);
            assert_eq!(counts.resets, counts.accepted);
            assert_eq!(counts.collapsed, 872);
            match mode {
                Consumer::Line => assert_eq!((counts.accepted, counts.geometry_reject), (2904, 0)),
                Consumer::Scatter => {
                    assert_eq!((counts.accepted, counts.geometry_reject), (2848, 56))
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} native viewport consumer differences; all case diagnostics retained in test output; first: {}",
        failures.len(),
        failures.first().map_or("", String::as_str)
    );
}
