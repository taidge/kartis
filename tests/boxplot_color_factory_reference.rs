//! Execute the declared typed adapters against the complete original fixture.
//! Native priming, nullable coercion, CSS/typed alpha and arbitrary-property
//! boundaries are tested explicitly; no React or native-outlier parity claim.
#![cfg(feature = "geojson")]
use glory::render::{GraphicStroke, StrokeCap};
use kartis::*;
use serde_json::Value;
use std::cell::Cell;
use std::collections::BTreeSet;

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/box-colors-function/values.json")).unwrap()
}
fn arr(v: &Value) -> &[Value] {
    v.as_array().unwrap()
}
fn text(v: &Value) -> &str {
    v.as_str().unwrap()
}
fn index(v: &Value) -> usize {
    v.as_u64().unwrap() as usize
}
fn number(v: &Value) -> f64 {
    v.as_f64().unwrap()
}
fn strings(v: &Value) -> Vec<String> {
    arr(v).iter().map(|v| text(v).into()).collect()
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() <= 1e-12, "{a} != native {b}");
}
fn five(a: &[f64; 5], b: &Value) {
    for (a, b) in a.iter().zip(arr(b)) {
        near(*a, number(b));
    }
}
fn rgba(v: &Value) -> Color {
    let v = arr(v);
    Color(
        index(&v[0]) as u8,
        index(&v[1]) as u8,
        index(&v[2]) as u8,
        index(&v[3]) as u8,
    )
}
// This is a test application's bounded explicit CSS-to-typed conversion, not
// a new library CSS parser or arbitrary native property accessor.
fn css_color(s: &str) -> Option<Color> {
    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() != 6 {
            return None;
        }
        return Some(Color(
            u8::from_str_radix(&hex[0..2], 16).ok()?,
            u8::from_str_radix(&hex[2..4], 16).ok()?,
            u8::from_str_radix(&hex[4..6], 16).ok()?,
            255,
        ));
    }
    let body = s
        .strip_prefix("rgba(")
        .or_else(|| s.strip_prefix("rgb("))?
        .strip_suffix(')')?;
    let v: Vec<f64> = body
        .split(',')
        .map(|x| x.trim().parse())
        .collect::<std::result::Result<_, _>>()
        .ok()?;
    if !(v.len() == 3 || v.len() == 4) || v.iter().any(|x| !x.is_finite()) {
        return None;
    }
    Some(Color(
        v[0].round() as u8,
        v[1].round() as u8,
        v[2].round() as u8,
        if v.len() == 4 {
            (v[3] * 255.).round() as u8
        } else {
            255
        },
    ))
}
fn native_color(v: &Value) -> Color {
    if v["rawRGBA"].is_array() {
        rgba(&v["rawRGBA"])
    } else {
        css_color(text(&v["css"])).unwrap()
    }
}
fn source(input: &Value, which: &str) -> Vec<BoxObservation> {
    arr(&input[which])
        .iter()
        .map(|r| {
            BoxObservation::new(
                text(&r["id"]),
                text(&r["group"]),
                text(&r["subGroup"]),
                r["v"].as_f64(),
            )
        })
        .collect()
}
fn setup(input: &Value) -> (ChartConfig, BoxPlotLayoutOptions) {
    let mut c = ChartConfig {
        width: 480.,
        height: 300.,
        margin: Margin {
            top: 0.,
            right: 0.,
            bottom: 0.,
            left: 0.,
        },
        ..Default::default()
    };
    c.theme.colors = vec![
        Color(11, 22, 33, 255),
        Color(44, 55, 66, 255),
        Color(77, 88, 99, 255),
    ];
    let mut o = BoxPlotLayoutOptions {
        orientation: if text(&input["layout"]) == "vertical" {
            StatisticsOrientation::Vertical
        } else {
            StatisticsOrientation::Horizontal
        },
        groups: Some(strings(&input["groups"])),
        subgroups: Some(strings(&input["subGroups"])),
        color_by: if text(&input["colorBy"]) == "group" {
            BoxPlotColorBy::Group
        } else {
            BoxPlotColorBy::SubGroup
        },
        legend_mode: BoxPlotLegendMode::ByLabel,
        legend: input["legendEnabled"].as_bool().unwrap(),
        auto_domain_source: BoxPlotAutoDomainSource::SummaryQuantiles,
        padding: 0.1,
        inner_padding: 4.,
        fill_opacity: 1.,
        border: Some(GraphicStroke {
            color: Color(1, 2, 3, 255),
            width: 2.,
            cap: StrokeCap::Square,
        }),
        border_color_profile: Some(InheritedColor::FromContext(vec![])),
        median_color_profile: Some(InheritedColor::FromContext(vec![ColorModifier::Darker(2.)])),
        whisker_color_profile: Some(InheritedColor::FromContext(vec![])),
        value_axis: AxisOptions {
            visible: false,
            grid: false,
            ..Default::default()
        },
        group_axis: CategoryAxisOptions {
            visible: false,
            grid: false,
            ..Default::default()
        },
        ..Default::default()
    };
    // Original Nivo horizontal band axes use [height, 0]. Rust keeps its
    // established horizontal default; select its existing explicit band
    // profile to compare the same original coordinate convention.
    if o.orientation == StatisticsOrientation::Horizontal {
        o.category_scale = Some(kartis::scales::BandScaleOptions {
            padding_inner: 0.1,
            padding_outer: 0.1,
            align: 0.5,
            round: false,
        });
        o.category_reverse = true;
    }
    (c, o)
}
fn declared(ctx: &BoxPlotColorContext<'_>, input: &Value) -> Result<Color> {
    let d = &input["color"];
    Ok(match text(&d["kind"]) {
        "constant" => css_color(text(&d["css"])).ok_or_else(|| {
            ChartError("test application rejects invalid CSS as typed RGBA".into())
        })?,
        "group" => {
            if ctx.computed_box.group == "A" {
                Color(120, 40, 90, 200)
            } else if ctx.computed_box.group == "B" {
                Color(22, 80, 120, 255)
            } else {
                Color(12, 13, 14, 255)
            }
        }
        "subgroup" => {
            if ctx.computed_box.subgroup == "x" {
                Color(50, 140, 80, 210)
            } else if ctx.computed_box.subgroup == "y" {
                Color(100, 60, 190, 255)
            } else {
                Color(12, 13, 14, 255)
            }
        }
        "mean" => Color((ctx.computed_box.mean * 12.).round() as u8, 40, 90, 200),
        "values" => Color(
            (ctx.computed_box.quantiles[2] * 10.).round() as u8,
            (ctx.computed_box.quantiles[4] * 8.).round() as u8,
            30,
            220,
        ),
        "cell" => Color(
            20 + ctx.group_index as u8 * 40,
            30 + ctx.subgroup_index as u8 * 50,
            90,
            200,
        ),
        "unguarded-values" => Color(ctx.computed_box.quantiles[2].round() as u8, 20, 30, 255),
        "datum" => {
            return Err(ChartError(
                "test application does not convert arbitrary JS datum properties to Color".into(),
            ));
        }
        other => panic!("uncovered original color descriptor {other}"),
    })
}
fn label(ctx: &BoxPlotLegendContext<'_>, input: &Value) -> String {
    match text(&input["label"]["kind"]) {
        "pair" => format!("{} / {}", ctx.computed_box.group, ctx.computed_box.subgroup),
        "constant" => "same caption".into(),
        "empty" => String::new(),
        other => panic!("uncovered label {other}"),
    }
}
fn mark<'a>(l: &'a BoxPlotLayout, id: &str) -> &'a Mark {
    l.scene.marks.iter().find(|m| m.id == id).unwrap()
}
fn target(v: &Value) -> LegendTarget {
    let raw = text(&v["rawKey"]);
    if text(&v["kind"]) == "Category" {
        LegendTarget::Category(raw.into())
    } else {
        LegendTarget::Series(boxplot_series_id(raw))
    }
}
fn context(
    ctx: &BoxPlotColorContext<'_>,
    native: &Value,
    data: &[BoxObservation],
    c: &ChartConfig,
    raw_indices: bool,
) {
    assert!(std::ptr::eq(ctx.data.as_ptr(), data.as_ptr()));
    assert_eq!(ctx.summary_index, index(&native["summaryIndex"]));
    assert_eq!(ctx.group_index, index(&native["groupIndex"]));
    assert_eq!(ctx.subgroup_index, index(&native["subGroupIndex"]));
    assert_eq!(ctx.cell_index, index(&native["cellIndex"]));
    assert_eq!(ctx.color_index, index(&native["colorIndex"]));
    assert_eq!(ctx.raw_color_key, text(&native["rawColorKey"]));
    assert_eq!(ctx.default_color, c.theme.color(ctx.color_index));
    let indices = if raw_indices {
        "observationIndices"
    } else {
        "effectiveObservationIndices"
    };
    assert_eq!(
        ctx.observation_indices,
        arr(&native[indices]).iter().map(index).collect::<Vec<_>>()
    );
    let s = &native["sourceSummary"];
    assert_eq!(ctx.computed_box.group, text(&s["group"]));
    assert_eq!(ctx.computed_box.subgroup, text(&s["subGroup"]));
    assert_eq!(ctx.computed_box.count, index(&s["n"]));
    near(ctx.computed_box.mean, number(&s["mean"]));
    five(&ctx.computed_box.quantiles, &s["values"]);
    five(&ctx.quantile_probabilities, &s["quantiles"]);
    for (a, b) in ctx.computed_box.extrema.iter().zip(arr(&s["extrema"])) {
        near(*a, number(b));
    }
    for (a, key) in [
        (ctx.computed_box.rect.x, "x"),
        (ctx.computed_box.rect.y, "y"),
        (ctx.computed_box.rect.width, "width"),
        (ctx.computed_box.rect.height, "height"),
    ] {
        assert!(
            (a - number(&native["position"][key])).abs() <= 1e-9,
            "geometry {key}: actual={a}, native={}, group={}, subgroup={}, groupIndex={}, subIndex={}",
            number(&native["position"][key]),
            ctx.computed_box.group,
            ctx.computed_box.subgroup,
            ctx.group_index,
            ctx.subgroup_index
        );
    }
}
fn rows(l: &BoxPlotLayout, actual: &Value) {
    assert_eq!(l.scene.legend.len(), arr(&actual["expectedLegend"]).len());
    for (row, n) in l.scene.legend.iter().zip(arr(&actual["expectedLegend"])) {
        assert_eq!(row.label, text(&n["label"]));
        assert_eq!(row.color, native_color(&n["color"]));
        assert_eq!(l.scene.legend_target(&row.id), Some(target(&n["target"])));
        let b = &l.boxes[index(&n["representativeInputIndex"])];
        assert!(row.id.ends_with(&b.id));
        assert_eq!(row.color, mark(l, &b.id).fill.unwrap());
        assert_eq!(
            index(&n["fullColorIndex"]),
            index(&n["representative"]["colorIndex"])
        );
    }
}
fn paints(l: &BoxPlotLayout, native: &Value, typed_alpha_zero: bool) {
    for (b, n) in l.boxes.iter().zip(arr(&native["boxes"])) {
        let color = native_color(&n["numericColor"]);
        assert_eq!(mark(l, &b.id).fill, Some(color));
        assert_eq!(mark(l, &b.id).opacity, 1.);
        for (role, field) in [
            ("border", "borderColor"),
            ("whiskers", "whiskerColor"),
            ("caps", "whiskerColor"),
        ] {
            assert_eq!(
                mark(l, &format!("{}:{role}", b.id)).stroke.unwrap().color,
                native_color(&n[field])
            );
        }
        let median = mark(l, &format!("{}:median", b.id)).stroke.unwrap().color;
        if typed_alpha_zero {
            assert_eq!(native_color(&n["medianColor"]), Color(0, 0, 0, 0));
            assert_eq!(median, Color(30, 21, 11, 0));
        } else {
            assert_eq!(median, native_color(&n["medianColor"]));
        }
        for x in &b.outliers {
            let id = format!("{}:outlier:{}:{}", b.id, x.source_id.len(), x.source_id);
            assert_eq!(mark(l, &id).fill, Some(color));
        }
    }
}
fn exact_profile(p: &Value, raw_indices: bool) -> BoxPlotLayout {
    let input = &p["input"];
    let actual = &p["actual"];
    let (c, o) = setup(input);
    let data = source(
        input,
        if raw_indices {
            "rawData"
        } else {
            "effectiveData"
        },
    );
    let color_calls = Cell::new(0);
    let label_calls = Cell::new(0);
    let boxes = arr(&actual["boxes"]);
    let mut color = |ctx: &BoxPlotColorContext<'_>| {
        context(ctx, &boxes[color_calls.get()], &data, &c, raw_indices);
        let result = declared(ctx, input)?;
        assert_eq!(
            result,
            native_color(&boxes[color_calls.get()]["numericColor"])
        );
        color_calls.set(color_calls.get() + 1);
        Ok(result)
    };
    let mut text_factory = |ctx: &BoxPlotLegendContext<'_>| {
        assert_eq!(
            color_calls.get(),
            boxes.len(),
            "all color callbacks precede legend callbacks"
        );
        assert_eq!(
            ctx.color,
            native_color(&boxes[label_calls.get()]["numericColor"])
        );
        label_calls.set(label_calls.get() + 1);
        Ok(label(ctx, input))
    };
    let mut control = BoxPlotInteraction::with_factories(
        &c,
        &data,
        &o,
        &BoxPlotInteractionOptions {
            opacity: 0.8,
            active_opacity: 0.9,
            inactive_opacity: 0.2,
        },
        BoxPlotFactories {
            color: Some(&mut color),
            legend_label: Some(&mut text_factory),
        },
    )
    .unwrap();
    assert_eq!(color_calls.get(), boxes.len());
    assert_eq!(label_calls.get(), if o.legend { boxes.len() } else { 0 });
    assert_eq!(control.layout.boxes.len(), boxes.len());
    rows(&control.layout, actual);
    paints(&control.layout, actual, input["color"]["id"] == "alpha0");
    let before = control.layout.clone();
    if let Some(b) = before.boxes.first() {
        control
            .set_active(Some(BoxPlotKey::new(&b.group, &b.subgroup)))
            .unwrap();
        control.whole_parts_interaction_binding().unwrap();
        control.set_active(None).unwrap();
        assert_eq!(control.layout, before);
    }
    assert_eq!(color_calls.get(), boxes.len());
    assert_eq!(label_calls.get(), if o.legend { boxes.len() } else { 0 });
    control.layout
}

#[test]
fn every_32_original_profiles_match_complete_context_paints_first_representative_and_cached_hover()
{
    let f = fixture();
    assert_eq!(arr(&f["profiles"]).len(), 32);
    for p in arr(&f["profiles"]) {
        eprintln!("ORIGINAL PROFILE {}", text(&p["id"]));
        exact_profile(p, false);
        assert_eq!(arr(&p["actual"]["primingCalls"]).len(), 3);
        assert_eq!(arr(&p["actual"]["summaryColorCalls"]).len(), 4);
        assert_eq!(arr(&p["actual"]["colorCalls"]).len(), 7);
    }
}

#[test]
fn native_partial_priming_throws_but_typed_summary_context_has_values_and_empty_has_no_callbacks() {
    let f = fixture();
    for p in arr(&f["qualifications"])
        .iter()
        .filter(|p| text(&p["id"]).contains("real-partial"))
    {
        assert_eq!(p["actual"]["error"]["name"], "TypeError");
        assert_eq!(arr(&p["actual"]["primingCalls"]).len(), 1);
        assert!(arr(&p["actual"]["summaryColorCalls"]).is_empty());
        let (c, o) = setup(&p["input"]);
        let d = source(&p["input"], "rawData");
        let calls = Cell::new(0);
        let mut color = |ctx: &BoxPlotColorContext<'_>| {
            calls.set(calls.get() + 1);
            declared(ctx, &p["input"])
        };
        let l = boxplot_layout_with_factories(
            &c,
            &d,
            &o,
            BoxPlotFactories {
                color: Some(&mut color),
                legend_label: None,
            },
        )
        .unwrap();
        assert_eq!(calls.get(), 4);
        assert_eq!(mark(&l, &l.boxes[0].id).fill, Some(Color(5, 20, 30, 255)));
    }
    for p in arr(&f["qualifications"])
        .iter()
        .filter(|p| text(&p["id"]).contains("empty-reserved"))
    {
        let l = exact_profile(p, false);
        assert!(l.boxes.is_empty());
        assert_eq!(arr(&p["actual"]["primingCalls"]).len(), 3);
        assert!(arr(&p["actual"]["summaryColorCalls"]).is_empty());
    }
}

#[test]
fn upstream_hidden_profiles_match_effective_source_but_none_and_native_null_are_explicitly_different()
 {
    let f = fixture();
    for p in arr(&f["qualifications"])
        .iter()
        .filter(|p| text(&p["id"]).contains("upstream-hidden"))
    {
        let l = exact_profile(p, false);
        assert_eq!(l.boxes.len(), 2);
        assert_eq!(arr(&p["actual"]["primingCalls"]).len(), 3);
    }
    for p in arr(&f["qualifications"])
        .iter()
        .filter(|p| text(&p["id"]).ends_with("-nullable"))
    {
        let a = &p["actual"];
        assert_eq!(arr(&a["boxes"]).len(), 2);
        let native_b = arr(&a["boxes"]).iter().find(|b| b["group"] == "B").unwrap();
        assert_eq!(native_b["sourceSummary"]["n"], 2);
        assert_eq!(native_b["sourceSummary"]["mean"], 2.);
        let (c, o) = setup(&p["input"]);
        let d = source(&p["input"], "rawData");
        let calls = Cell::new(0);
        let mut color = |ctx: &BoxPlotColorContext<'_>| {
            assert_eq!(ctx.computed_box.group, "B");
            assert_eq!(ctx.computed_box.count, 1);
            assert_eq!(ctx.computed_box.mean, 4.);
            assert_eq!(ctx.observation_indices, &[0, 1]);
            assert_eq!(ctx.data[0].value, None);
            calls.set(calls.get() + 1);
            declared(ctx, &p["input"])
        };
        let l = boxplot_layout_with_factories(
            &c,
            &d,
            &o,
            BoxPlotFactories {
                color: Some(&mut color),
                legend_label: None,
            },
        )
        .unwrap();
        assert_eq!(calls.get(), 1);
        assert_eq!(l.boxes.len(), 1);
        assert_eq!(mark(&l, &l.boxes[0].id).fill, Some(Color(48, 40, 90, 200)));
    }
}

#[test]
fn fixed_roles_and_typed_alpha_zero_keep_their_declared_numeric_contracts() {
    let f = fixture();
    let fixed = arr(&f["qualifications"])
        .iter()
        .find(|p| p["id"] == "fixed-three-strokes")
        .unwrap();
    let (c, mut o) = setup(&fixed["input"]);
    o.border_color_profile = None;
    o.median_color_profile = None;
    o.whisker_color_profile = None;
    o.border.as_mut().unwrap().color = Color(17, 34, 51, 255);
    o.median.color = Color(44, 55, 66, 128);
    o.whisker.color = Color(119, 136, 153, 255);
    o.outlier_fill = Some(Color(7, 8, 9, 90));
    let d = source(&fixed["input"], "rawData");
    let mut color = |ctx: &BoxPlotColorContext<'_>| declared(ctx, &fixed["input"]);
    let mut caption = |ctx: &BoxPlotLegendContext<'_>| Ok(label(ctx, &fixed["input"]));
    let l = boxplot_layout_with_factories(
        &c,
        &d,
        &o,
        BoxPlotFactories {
            color: Some(&mut color),
            legend_label: Some(&mut caption),
        },
    )
    .unwrap();
    rows(&l, &fixed["actual"]);
    for (b, n) in l.boxes.iter().zip(arr(&fixed["actual"]["boxes"])) {
        assert_eq!(
            mark(&l, &format!("{}:border", b.id)).stroke.unwrap().color,
            native_color(&n["borderColor"])
        );
        assert_eq!(
            mark(&l, &format!("{}:median", b.id)).stroke.unwrap().color,
            native_color(&n["medianColor"])
        );
        assert_eq!(
            mark(&l, &format!("{}:caps", b.id)).stroke.unwrap().color,
            native_color(&n["whiskerColor"])
        );
        assert!(!b.outliers.is_empty());
        for x in &b.outliers {
            assert_eq!(
                mark(
                    &l,
                    &format!("{}:outlier:{}:{}", b.id, x.source_id.len(), x.source_id)
                )
                .fill,
                Some(Color(7, 8, 9, 90))
            );
        }
    }
    let p = arr(&f["qualifications"])
        .iter()
        .find(|p| p["id"] == "alpha-zero-modifier-restores-black")
        .unwrap();
    let (c, mut o) = setup(&p["input"]);
    o.border_color_profile = Some(InheritedColor::FromContext(vec![ColorModifier::Opacity(
        1.,
    )]));
    o.median_color_profile = Some(InheritedColor::FromContext(vec![]));
    let d = source(&p["input"], "rawData");
    let mut color = |ctx: &BoxPlotColorContext<'_>| declared(ctx, &p["input"]);
    let mut caption = |ctx: &BoxPlotLegendContext<'_>| Ok(label(ctx, &p["input"]));
    let l = boxplot_layout_with_factories(
        &c,
        &d,
        &o,
        BoxPlotFactories {
            color: Some(&mut color),
            legend_label: Some(&mut caption),
        },
    )
    .unwrap();
    rows(&l, &p["actual"]);
    for (b, n) in l.boxes.iter().zip(arr(&p["actual"]["boxes"])) {
        assert_eq!(native_color(&n["borderColor"]), Color(0, 0, 0, 255));
        assert_eq!(
            mark(&l, &format!("{}:border", b.id)).stroke.unwrap().color,
            Color(61, 42, 23, 255)
        );
        assert_eq!(mark(&l, &b.id).fill, Some(Color(61, 42, 23, 0)));
    }
}

#[test]
fn datum_and_invalid_css_qualifications_are_not_silently_cast_into_typed_colors() {
    let f = fixture();
    for id in [
        "datum-raw-extra-property-lost",
        "datum-summary-mean-is-number-not-css",
        "invalid-css-original-not-prevalidated",
    ] {
        let p = arr(&f["qualifications"])
            .iter()
            .find(|p| p["id"] == id)
            .unwrap();
        assert!(p["actual"]["completedBoxHook"].as_bool().unwrap());
        let expected = if id.contains("raw-extra") {
            "undefined"
        } else if id.contains("mean") {
            "number"
        } else {
            "css"
        };
        assert!(
            arr(&p["actual"]["boxes"])
                .iter()
                .all(|b| b["numericColor"]["kind"] == expected)
        );
        let (c, o) = setup(&p["input"]);
        let d = source(&p["input"], "rawData");
        let mut control =
            BoxPlotInteraction::new(&c, &d, &o, &BoxPlotInteractionOptions::default()).unwrap();
        let before = control.layout.clone();
        let calls = Cell::new(0);
        let mut color = |ctx: &BoxPlotColorContext<'_>| {
            calls.set(calls.get() + 1);
            declared(ctx, &p["input"])
        };
        assert!(
            control
                .replace_source_with_factories(
                    &c,
                    &d,
                    &o,
                    BoxPlotFactories {
                        color: Some(&mut color),
                        legend_label: None
                    }
                )
                .is_err()
        );
        assert_eq!(calls.get(), 1);
        assert_eq!(control.layout, before);
    }
    let p = arr(&f["qualifications"])
        .iter()
        .find(|p| p["id"] == "datum-precomputed-spread-keeps-extra-property")
        .unwrap();
    assert_eq!(
        p["actual"]["boxes"][0]["numericColor"]["css"],
        "rgba(99,77,55,0.5)"
    );
    let (c, o) = setup(&p["input"]);
    let d = source(&p["input"], "rawData");
    assert_eq!(
        d[0].value, None,
        "fixed Rust observation has no precomputed/meta representation"
    );
    let calls = Cell::new(0);
    let mut color = |_: &BoxPlotColorContext<'_>| {
        calls.set(calls.get() + 1);
        Ok(Color(99, 77, 55, 128))
    };
    let l = boxplot_layout_with_factories(
        &c,
        &d,
        &o,
        BoxPlotFactories {
            color: Some(&mut color),
            legend_label: None,
        },
    )
    .unwrap();
    assert!(l.boxes.is_empty());
    assert_eq!(calls.get(), 0);
}

#[test]
fn legend_false_cold_active_and_keys_reserved_fallback_consume_the_remaining_original_cases() {
    let f = fixture();
    let p = arr(&f["qualifications"])
        .iter()
        .find(|p| p["id"] == "legend-disabled-still-computes-color")
        .unwrap();
    assert!(exact_profile(p, false).scene.legend.is_empty());
    let p = arr(&f["qualifications"])
        .iter()
        .find(|p| p["id"] == "explicit-cold-active-reexecution")
        .unwrap();
    let input = &p["input"];
    let (c, o) = setup(input);
    let d = source(input, "rawData");
    let calls = Cell::new(0);
    let mut color = |ctx: &BoxPlotColorContext<'_>| {
        calls.set(calls.get() + 1);
        declared(ctx, input)
    };
    let caption_calls = Cell::new(0);
    let mut caption = |ctx: &BoxPlotLegendContext<'_>| {
        caption_calls.set(caption_calls.get() + 1);
        Ok(label(ctx, input))
    };
    let mut control = BoxPlotInteraction::with_factories(
        &c,
        &d,
        &o,
        &BoxPlotInteractionOptions {
            opacity: 0.8,
            active_opacity: 0.9,
            inactive_opacity: 0.2,
        },
        BoxPlotFactories {
            color: Some(&mut color),
            legend_label: Some(&mut caption),
        },
    )
    .unwrap();
    assert_eq!(calls.get(), 4);
    assert_eq!(caption_calls.get(), 4);
    rows(&control.layout, &p["actual"]["before"]);
    control.set_active(Some(BoxPlotKey::new("B", "y"))).unwrap();
    for (b, n) in control
        .layout
        .boxes
        .iter()
        .zip(arr(&p["actual"]["after"]["boxes"]))
    {
        near(
            control
                .layout
                .scene
                .effective_mark_opacity(mark(&control.layout, &b.id)),
            number(&n["opacity"]),
        );
        assert_eq!(
            mark(&control.layout, &b.id).fill,
            Some(native_color(&n["numericColor"]))
        );
    }
    assert_eq!(calls.get(), 4);
    assert_eq!(caption_calls.get(), 4);
    rows(&control.layout, &p["actual"]["after"]);
    assert_eq!(arr(&p["actual"]["before"]["summaryColorCalls"]).len(), 4);
    assert_eq!(arr(&p["actual"]["after"]["summaryColorCalls"]).len(), 4);
    let p = arr(&f["profiles"])
        .iter()
        .find(|p| p["id"] == "subGroup-vertical-cell")
        .unwrap();
    let (c, mut o) = setup(&p["input"]);
    o.legend_mode = BoxPlotLegendMode::Keys;
    let d = source(&p["input"], "rawData");
    let count = Cell::new(0);
    let mut color = |ctx: &BoxPlotColorContext<'_>| {
        count.set(count.get() + 1);
        declared(ctx, &p["input"])
    };
    let l = boxplot_layout_with_factories(
        &c,
        &d,
        &o,
        BoxPlotFactories {
            color: Some(&mut color),
            legend_label: None,
        },
    )
    .unwrap();
    assert_eq!(count.get(), 4);
    assert_eq!(l.scene.legend.len(), 3);
    assert_eq!(l.scene.legend[0].color, c.theme.color(0));
    assert_ne!(l.scene.legend[0].color, Color(201, 202, 203, 255));
    assert_eq!(l.scene.legend[1].color, Color(60, 80, 90, 200));
    assert_eq!(l.scene.legend[2].color, Color(60, 130, 90, 200));
    assert_eq!(
        arr(&p["actual"]["expectedLegend"]).len(),
        1,
        "Native ByLabel constant text differs from Rust legacy full Keys rows"
    );
}

#[test]
fn all_16_original_qualification_ids_are_accounted_for_without_silent_subset() {
    let f = fixture();
    let expected: [&str; 16] = [
        "group-real-partial-summary-values-throws",
        "group-upstream-hidden",
        "group-nullable",
        "group-empty-reserved",
        "subGroup-real-partial-summary-values-throws",
        "subGroup-upstream-hidden",
        "subGroup-nullable",
        "subGroup-empty-reserved",
        "fixed-three-strokes",
        "alpha-zero-modifier-restores-black",
        "datum-raw-extra-property-lost",
        "datum-precomputed-spread-keeps-extra-property",
        "datum-summary-mean-is-number-not-css",
        "invalid-css-original-not-prevalidated",
        "legend-disabled-still-computes-color",
        "explicit-cold-active-reexecution",
    ];
    assert_eq!(
        arr(&f["qualifications"])
            .iter()
            .map(|p| text(&p["id"]))
            .collect::<BTreeSet<_>>(),
        expected.into_iter().collect()
    );
}
