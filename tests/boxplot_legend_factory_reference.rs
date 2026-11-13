//! Consume actual original Nivo hooks + Lodash rows. Nullable coercion, upstream
//! filtering and numeric property labels have explicit qualification tests.
//! No JS accessor/React lifecycle, geometry or physical-render parity is implied.
#![cfg(feature = "geojson")]
use std::collections::BTreeSet;

use kartis::*;
use serde_json::Value;

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/box-legend-factory/values.json")).unwrap()
}
fn array(v: &Value) -> &[Value] {
    v.as_array().unwrap()
}
fn text(v: &Value) -> &str {
    v.as_str().unwrap()
}
fn index(v: &Value) -> usize {
    v.as_u64().unwrap() as usize
}
fn strings(v: &Value) -> Vec<String> {
    array(v).iter().map(|v| text(v).into()).collect()
}
fn rgba(v: &Value) -> Color {
    let v = array(v);
    Color(
        index(&v[0]) as u8,
        index(&v[1]) as u8,
        index(&v[2]) as u8,
        index(&v[3]) as u8,
    )
}
fn near(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-12,
        "{actual} != original {expected}"
    );
}
fn numbers(actual: &[f64], expected: &Value) {
    assert_eq!(actual.len(), array(expected).len());
    for (a, b) in actual.iter().zip(array(expected)) {
        near(*a, b.as_f64().unwrap());
    }
}
fn source(input: &Value, which: &str) -> Vec<BoxObservation> {
    array(&input[which])
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
    let mut chart = ChartConfig {
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
    chart.theme.colors = array(&input["paletteRGBA"]).iter().map(rgba).collect();
    let mut quantiles = [0.; 5];
    for (q, v) in quantiles
        .iter_mut()
        .zip(array(&input["quantileProbabilities"]))
    {
        *q = v.as_f64().unwrap();
    }
    let options = BoxPlotLayoutOptions {
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
        quantiles,
        padding: 0.1,
        inner_padding: 4.,
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
    (chart, options)
}
fn descriptor_label(ctx: &BoxPlotLegendContext<'_>, input: &Value) -> String {
    match text(&input["label"]["kind"]) {
        "pair" => format!(
            "{} / {} Å <>&",
            ctx.computed_box.group, ctx.computed_box.subgroup
        ),
        "constant" => "same caption".into(),
        "raw-key" => ctx.raw_color_key.into(),
        "empty" => String::new(),
        // Explicit typed Rust conversion for this one native numeric property.
        // This is not a generic JS/Lodash property accessor implementation.
        "property" => {
            assert_eq!(text(&input["label"]["path"]), "values[2]");
            ctx.computed_box.quantiles[2].to_string()
        }
        other => panic!("uncovered original declared descriptor {other}"),
    }
}
fn summary_identity(ctx: &BoxPlotLegendContext<'_>, native: &Value) {
    assert_eq!(ctx.summary_index, index(&native["summaryIndex"]));
    assert_eq!(ctx.computed_box.group, text(&native["group"]));
    assert_eq!(ctx.computed_box.subgroup, text(&native["subGroup"]));
    assert_eq!(ctx.group_index, index(&native["groupIndex"]));
    assert_eq!(ctx.subgroup_index, index(&native["subGroupIndex"]));
    assert_eq!(ctx.cell_index, index(&native["cellIndex"]));
    assert_eq!(ctx.color_index, index(&native["colorIndex"]));
    assert_eq!(ctx.raw_color_key, text(&native["rawColorKey"]));
}
fn check_context(
    ctx: &BoxPlotLegendContext<'_>,
    native: &Value,
    box_native: &Value,
    raw_indices: bool,
) {
    summary_identity(ctx, native);
    let expected = array(
        &native[if raw_indices {
            "observationIndices"
        } else {
            "effectiveObservationIndices"
        }],
    )
    .iter()
    .map(index)
    .collect::<Vec<_>>();
    assert_eq!(ctx.observation_indices, expected);
    let original = &box_native["sourceSummary"];
    assert_eq!(ctx.computed_box.count, index(&original["n"]));
    numbers(&ctx.computed_box.extrema, &original["extrema"]);
    numbers(
        &ctx.computed_box.quantiles,
        &box_native["computedStatistics"],
    );
    near(ctx.computed_box.mean, original["mean"].as_f64().unwrap());
    numbers(
        &ctx.quantile_probabilities,
        &native["quantileProbabilities"],
    );
    assert_eq!(ctx.color, rgba(&box_native["numericRGBA"]));
    // Original data.quantiles are probabilities; data.values are statistics.
    if !native["data"].is_null() {
        numbers(&ctx.quantile_probabilities, &native["data"]["quantiles"]);
        numbers(&ctx.computed_box.quantiles, &native["data"]["values"]);
    }
}
fn target(native: &Value) -> LegendTarget {
    let key = text(&native["target"]["rawKey"]);
    match text(&native["target"]["kind"]) {
        "Series" => LegendTarget::Series(boxplot_series_id(key)),
        "Category" => LegendTarget::Category(key.into()),
        other => panic!("unhandled native target {other}"),
    }
}
fn expected_text(v: &Value, numeric_conversion: bool) -> String {
    if let Some(s) = v.as_str() {
        s.into()
    } else {
        assert!(
            numeric_conversion,
            "numeric native labels require explicit qualified conversion"
        );
        v.as_f64().unwrap().to_string()
    }
}
fn check_rows(layout: &BoxPlotLayout, rows: &[Value], numeric_conversion: bool) {
    assert_eq!(layout.scene.legend.len(), rows.len());
    assert_eq!(layout.scene.legend_targets.len(), rows.len());
    assert_eq!(
        layout
            .scene
            .legend
            .iter()
            .map(|r| &r.id)
            .collect::<BTreeSet<_>>()
            .len(),
        rows.len()
    );
    for (actual, native) in layout.scene.legend.iter().zip(rows) {
        assert_eq!(
            actual.label,
            expected_text(&native["label"], numeric_conversion)
        );
        assert_eq!(actual.color, rgba(&native["rgba"]));
        assert_eq!(layout.scene.legend_target(&actual.id), Some(target(native)));
        assert_eq!(index(&native["nativeId"]), index(&native["fullColorIndex"]));
        let representative = &native["representative"];
        assert_eq!(
            index(&representative["colorIndex"]),
            index(&native["fullColorIndex"])
        );
        let summary = layout
            .boxes
            .iter()
            .find(|b| {
                b.group == text(&representative["group"])
                    && b.subgroup == text(&representative["subGroup"])
            })
            .unwrap();
        // Native numeric IDs may repeat; the Rust transport ID instead carries
        // the exact first representative's collision-free original box ID.
        assert!(actual.id.ends_with(summary.id.as_str()));
    }
}
fn strict_profile(profile: &Value, raw_indices: bool) -> (usize, usize, BoxPlotLayout) {
    let input = &profile["input"];
    let actual = &profile["actual"];
    let (chart, options) = setup(input);
    let data = source(
        input,
        if raw_indices {
            "rawData"
        } else {
            "effectiveData"
        },
    );
    let native_contexts = array(&actual["callbackContexts"]);
    let native_boxes = array(&actual["boxes"]);
    let mut calls = 0;
    let layout = boxplot_layout_with_legend_label_factory(&chart, &data, &options, &mut |ctx| {
        assert!(std::ptr::eq(ctx.data.as_ptr(), data.as_ptr()));
        let n = &native_contexts[calls];
        check_context(ctx, n, &native_boxes[calls], raw_indices);
        let label = descriptor_label(ctx, input);
        assert_eq!(label, text(&n["returnedLabel"]));
        calls += 1;
        Ok(label)
    })
    .unwrap();
    assert_eq!(calls, native_contexts.len());
    assert_eq!(layout.boxes.len(), native_boxes.len());
    assert_eq!(layout.groups, strings(&input["groups"]));
    assert_eq!(layout.subgroups, strings(&input["subGroups"]));
    let selected = if options.color_by == BoxPlotColorBy::Group {
        "groups"
    } else {
        "subGroups"
    };
    assert_eq!(strings(&actual["ordinalDomain"]), strings(&input[selected]));
    for (b, n) in layout.boxes.iter().zip(native_boxes) {
        assert_eq!(b.group, text(&n["group"]));
        assert_eq!(b.subgroup, text(&n["subGroup"]));
        numbers(&b.quantiles, &n["computedStatistics"]);
        assert_eq!(b.count, index(&n["sourceSummary"]["n"]));
    }
    let rows = array(&actual["expectedLegend"]);
    check_rows(&layout, rows, false);
    if options.legend {
        let ops = array(&actual["originalLegendOperations"]);
        assert_eq!(ops.len(), 2);
        assert_eq!(text(&ops[0]["kind"]), "uniqBy");
        assert_eq!(text(&ops[1]["kind"]), "sortBy");
        let unique = array(&ops[0]["selectedInputIndices"]);
        let sorted = array(&ops[1]["selectedInputIndices"]);
        for (row, selected) in rows.iter().zip(sorted) {
            assert_eq!(
                index(&row["representativeInputIndex"]),
                index(&unique[index(selected)])
            );
        }
    }
    (calls, rows.len(), layout)
}

#[test]
fn all_32_original_hook_profiles_match_rows_representatives_targets_and_borrowed_contexts() {
    let f = fixture();
    assert_eq!(f["schema"], "box-legend-factory-original-v1");
    let profiles = array(&f["profiles"]);
    assert_eq!(profiles.len(), 32);
    let (mut callbacks, mut rows) = (0, 0);
    for p in profiles {
        let (c, r, _) = strict_profile(p, false);
        callbacks += c;
        rows += r;
    }
    assert_eq!(callbacks, 88);
    assert_eq!(rows, 46);
    for by in ["group", "subGroup"] {
        let p = profiles
            .iter()
            .find(|p| text(&p["id"]) == format!("{by}-complete-pair"))
            .unwrap();
        let native = array(&p["actual"]["expectedLegend"]);
        assert_eq!(
            native
                .iter()
                .map(|r| index(&r["nativeId"]))
                .collect::<Vec<_>>(),
            [1, 1, 2, 2]
        );
        let (_, _, layout) = strict_profile(p, false);
        assert_eq!(
            layout
                .scene
                .legend
                .iter()
                .map(|r| &r.id)
                .collect::<BTreeSet<_>>()
                .len(),
            4
        );
        let constant = profiles
            .iter()
            .find(|p| text(&p["id"]) == format!("{by}-complete-constant"))
            .unwrap();
        assert_eq!(array(&constant["actual"]["expectedLegend"]).len(), 1);
        assert_eq!(
            index(&constant["actual"]["expectedLegend"][0]["representativeInputIndex"]),
            0
        );
    }
}

#[test]
fn raw_key_without_factory_and_real_empty_subgroup_remain_distinct_from_legacy_keys() {
    let f = fixture();
    let mut main = 0;
    for p in array(&f["profiles"]) {
        if text(&p["input"]["label"]["kind"]) != "raw-key" {
            continue;
        }
        let (c, o) = setup(&p["input"]);
        let d = source(&p["input"], "effectiveData");
        let layout = boxplot_layout(&c, &d, &o).unwrap();
        check_rows(&layout, array(&p["actual"]["expectedLegend"]), false);
        main += 1;
    }
    assert_eq!(main, 8);
    let mut qualifications = 0;
    for p in array(&f["qualifications"]) {
        if !text(&p["id"]).ends_with("-raw-empty-subgroup") {
            continue;
        }
        strict_profile(p, false);
        let (c, mut o) = setup(&p["input"]);
        let d = source(&p["input"], "effectiveData");
        let by_label = boxplot_layout(&c, &d, &o).unwrap();
        check_rows(&by_label, array(&p["actual"]["expectedLegend"]), false);
        assert_eq!(by_label.boxes.len(), 1);
        assert_eq!(by_label.boxes[0].quantiles, [0.; 5]);
        if o.color_by == BoxPlotColorBy::SubGroup {
            assert_eq!(by_label.scene.legend[0].label, "");
            o.legend_mode = BoxPlotLegendMode::Keys;
            let keys = boxplot_layout(&c, &d, &o).unwrap();
            assert_eq!(keys.scene.legend.len(), 2);
            assert_eq!(keys.scene.legend[1].label, "Samples");
        }
        qualifications += 1;
    }
    assert_eq!(qualifications, 2);
}

#[test]
fn hidden_qualifications_distinguish_upstream_filtering_from_rust_subgroup_visibility() {
    let f = fixture();
    let mut count = 0;
    for p in array(&f["qualifications"]) {
        if !text(&p["id"]).contains("-hidden-upstream-") {
            continue;
        }
        assert_eq!(p["scope"]["nativeHasNoHiddenPropHere"], true);
        let (_, _, filtered) = strict_profile(p, false);
        let (c, mut o) = setup(&p["input"]);
        let raw = source(&p["input"], "rawData");
        assert_eq!(raw.len(), 8);
        assert_eq!(source(&p["input"], "effectiveData").len(), 4);
        if o.color_by == BoxPlotColorBy::SubGroup {
            o.hidden = strings(&p["input"]["hiddenSubGroups"])
                .iter()
                .map(|s| boxplot_series_id(s))
                .collect();
            let mut calls = 0;
            let retained = boxplot_layout_with_legend_label_factory(&c, &raw, &o, &mut |ctx| {
                let n = &p["actual"]["callbackContexts"][calls];
                check_context(ctx, n, &p["actual"]["boxes"][calls], true);
                calls += 1;
                Ok(descriptor_label(ctx, &p["input"]))
            })
            .unwrap();
            assert_eq!(calls, 2);
            check_rows(&retained, array(&p["actual"]["expectedLegend"]), false);
            assert!(retained.boxes.iter().all(|b| b.subgroup == "x"));
            assert_eq!(retained.groups, filtered.groups);
            assert_eq!(retained.subgroups, filtered.subgroups);
        } else {
            assert_eq!(strings(&p["input"]["hiddenGroups"]), ["B"]);
            let raw_layout = boxplot_layout(&c, &raw, &o).unwrap();
            assert_eq!(raw_layout.boxes.len(), 4);
            assert_eq!(filtered.boxes.len(), 2);
            assert!(filtered.boxes.iter().all(|b| b.group == "A")); /* No Group->Series hiding is invented. */
        }
        count += 1;
    }
    assert_eq!(count, 2);
}

#[test]
fn nullable_qualifications_preserve_native_number_null_zero_and_rust_none_difference() {
    let f = fixture();
    let mut count = 0;
    for p in array(&f["qualifications"]) {
        if !text(&p["id"]).contains("-nullable-") {
            continue;
        }
        assert_eq!(p["scope"]["doNotClaimNullableParity"], true);
        let (c, o) = setup(&p["input"]);
        let d = source(&p["input"], "rawData");
        let mut contexts = Vec::new();
        let l = boxplot_layout_with_legend_label_factory(&c, &d, &o, &mut |ctx| {
            contexts.push((
                ctx.computed_box.group.clone(),
                ctx.computed_box.subgroup.clone(),
                ctx.observation_indices.to_vec(),
            ));
            Ok(descriptor_label(ctx, &p["input"]))
        })
        .unwrap();
        assert_eq!(l.boxes.len(), 2);
        assert_eq!(
            contexts,
            [
                ("B".into(), "x".into(), vec![3]),
                ("A".into(), "x".into(), vec![1, 2])
            ]
        );
        assert!(
            l.boxes
                .iter()
                .all(|b| !(b.group == "B" && b.subgroup == "y"))
        );
        let mixed = l.boxes.iter().find(|b| b.group == "A").unwrap();
        assert_eq!(mixed.count, 1);
        assert_eq!(mixed.quantiles, [4.; 5]);
        let zero = l.boxes.iter().find(|b| b.group == "B").unwrap();
        assert_eq!(zero.count, 1);
        assert_eq!(zero.quantiles, [0.; 5]);
        let native = array(&p["actual"]["boxes"]);
        assert_eq!(native.len(), 3);
        assert_eq!(native[0]["sourceSummary"]["n"], 1);
        numbers(&[0.; 5], &native[0]["computedStatistics"]);
        assert_eq!(native[2]["sourceSummary"]["n"], 2);
        numbers(&[0.4, 1., 2., 3., 3.6], &native[2]["computedStatistics"]);
        let retained_rows: Vec<_> = array(&p["actual"]["expectedLegend"])
            .iter()
            .filter(|r| text(&r["representative"]["subGroup"]) == "x")
            .cloned()
            .collect();
        check_rows(&l, &retained_rows, false);
        assert_eq!(array(&p["actual"]["expectedLegend"]).len(), 3);
        assert_eq!(l.scene.legend.len(), 2);
        count += 1;
    }
    assert_eq!(count, 2);
}

#[test]
fn disabled_legend_qualification_has_no_callback_and_keeps_real_boxes() {
    let f = fixture();
    let p = array(&f["qualifications"])
        .iter()
        .find(|p| text(&p["id"]) == "legend-false-callback-not-invoked")
        .unwrap();
    assert!(array(&p["actual"]["callbackContexts"]).is_empty());
    let (calls, rows, layout) = strict_profile(p, false);
    assert_eq!((calls, rows), (0, 0));
    assert_eq!(layout.boxes.len(), 4);
    assert!(layout.scene.legend_targets.is_empty());
}

#[test]
fn native_nested_numeric_accessor_is_an_explicit_string_conversion_not_a_rust_path_api() {
    let f = fixture();
    let p = array(&f["qualifications"])
        .iter()
        .find(|p| text(&p["id"]) == "original-real-lodash-nested-property-accessor")
        .unwrap();
    assert_eq!(
        p["scope"]["notClaimingArbitraryPropertyAccessorRustAPI"],
        true
    );
    assert!(array(&p["actual"]["callbackContexts"]).is_empty());
    assert_eq!(p["actual"]["originalAccessorKind"], "string");
    let (c, o) = setup(&p["input"]);
    let d = source(&p["input"], "effectiveData");
    let mut calls = 0;
    let layout = boxplot_layout_with_legend_label_factory(&c, &d, &o, &mut |ctx| {
        let b = &p["actual"]["boxes"][calls];
        check_context(ctx, b, b, false);
        calls += 1;
        Ok(descriptor_label(ctx, &p["input"]))
    })
    .unwrap();
    assert_eq!(calls, 4);
    assert!(
        array(&p["actual"]["expectedLegend"])
            .iter()
            .all(|r| r["label"].is_number())
    );
    check_rows(&layout, array(&p["actual"]["expectedLegend"]), true);
}
