#![cfg(feature = "geojson")]
//! The expected rows are actual original Nivo hook/helper output. Caption-based
//! native deduplication and Rust's full legacy subgroup legend remain distinct.
use std::collections::{BTreeMap, BTreeSet};

use kartis::*;
use serde_json::Value;

fn reference() -> Value {
    serde_json::from_str(include_str!("fixtures/box-legend-labels/values.json")).unwrap()
}
fn text(v: &Value) -> &str {
    v.as_str().unwrap()
}
fn rgba(v: &Value) -> Color {
    Color(
        v[0].as_u64().unwrap() as u8,
        v[1].as_u64().unwrap() as u8,
        v[2].as_u64().unwrap() as u8,
        v[3].as_u64().unwrap() as u8,
    )
}
fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|v| text(v).into())
        .collect()
}
fn domain(input: &Value) -> Vec<String> {
    strings(
        &input[if input["colorBy"] == "group" {
            "groups"
        } else {
            "subGroups"
        }],
    )
}
fn source(input: &Value) -> Vec<BoxObservation> {
    input["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            BoxObservation::new(
                text(&row["id"]),
                text(&row["group"]),
                text(&row["subGroup"]),
                row["v"].as_f64(),
            )
        })
        .collect()
}
fn config(input: &Value) -> ChartConfig {
    let mut c = ChartConfig {
        width: 480.,
        height: 300.,
        margin: Margin {
            left: 0.,
            top: 0.,
            right: 0.,
            bottom: 0.,
        },
        ..Default::default()
    };
    c.theme.colors = input["paletteRGBA"]
        .as_array()
        .unwrap()
        .iter()
        .map(rgba)
        .collect();
    c
}
fn options(input: &Value) -> BoxPlotLayoutOptions {
    BoxPlotLayoutOptions {
        groups: Some(strings(&input["groups"])),
        subgroups: Some(strings(&input["subGroups"])),
        color_by: if input["colorBy"] == "group" {
            BoxPlotColorBy::Group
        } else {
            BoxPlotColorBy::SubGroup
        },
        auto_domain_source: BoxPlotAutoDomainSource::SummaryQuantiles,
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
    }
}
fn captions(input: &Value) -> BTreeMap<String, String> {
    assert_eq!(input["caption"]["kind"], "map");
    input["caption"]["map"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(key, label)| (key.clone(), text(label).into()))
        .collect()
}
fn selected<'a>(input: &Value, b: &'a Value) -> &'a str {
    text(
        &b[if input["colorBy"] == "group" {
            "group"
        } else {
            "subGroup"
        }],
    )
}
fn legend_id(input: &Value, key: &str) -> String {
    if input["colorBy"] == "group" {
        key.into()
    } else {
        boxplot_series_id(key)
    }
}
fn native_rows(snapshot: &Value) -> &[Value] {
    assert_eq!(snapshot["legendsData"].as_array().unwrap().len(), 2);
    assert_eq!(
        snapshot["legendsData"][0]["data"],
        snapshot["legendsData"][1]["data"]
    );
    snapshot["legendsData"][0]["data"].as_array().unwrap()
}
fn native_contract(input: &Value, snapshot: &Value) {
    let keys = domain(input);
    assert_eq!(strings(&snapshot["ordinalDomain"]), keys);
    assert_eq!(snapshot["legendLabelRuntimeType"], "function");
    assert_eq!(snapshot["originalAccessorKeepsCallback"], true);
    // This is an observed budget of the original two legend configurations and
    // direct cross-check, not a promise that Rust invokes a text callback.
    assert_eq!(
        snapshot["callbackCalls"].as_array().unwrap().len(),
        snapshot["boxes"].as_array().unwrap().len() * 3
    );
    for row in native_rows(snapshot) {
        let index = row["id"].as_u64().unwrap() as usize;
        assert_eq!(keys[index], text(&row["sourceKey"]));
        let actual_box = snapshot["boxes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| selected(input, b) == keys[index])
            .unwrap();
        assert_eq!(row["rgba"], actual_box["numericRGBA"]);
        assert!(
            snapshot["callbackCalls"]
                .as_array()
                .unwrap()
                .iter()
                .any(|call| call["selectedKey"] == row["sourceKey"]
                    && call["returnedCaption"] == row["label"])
        );
    }
}
fn checked_map_layout(input: &Value, snapshot: &Value) -> Result<BoxPlotLayout> {
    native_contract(input, snapshot);
    let c = config(input);
    let data = source(input);
    let mut opts = options(input);
    let unchanged = boxplot_layout(&c, &data, &opts)?;
    let map = captions(input);
    opts.legend_labels = Some(map.clone());
    let actual = boxplot_layout(&c, &data, &opts)?;
    let mut only_captions_changed = unchanged;
    let keys = domain(input);
    let originals = snapshot["boxes"].as_array().unwrap();
    let eligible: Vec<_> = keys
        .iter()
        .filter(|key| {
            input["colorBy"] != "group"
                || originals.iter().any(|b| selected(input, b) == key.as_str())
        })
        .collect();
    assert_eq!(actual.scene.legend.len(), eligible.len());
    assert_eq!(actual.boxes.len(), originals.len());
    for (row, key) in actual.scene.legend.iter().zip(eligible) {
        assert_eq!(row.id, legend_id(input, key));
        assert_eq!(
            actual.scene.legend_role(&row.id),
            Some(if input["colorBy"] == "group" {
                LegendRole::Category
            } else {
                LegendRole::Series
            })
        );
        let old_row = only_captions_changed
            .scene
            .legend
            .iter_mut()
            .find(|r| r.id == row.id)
            .unwrap();
        old_row.label = map
            .get(key)
            .cloned()
            .unwrap_or_else(|| old_row.label.clone());
        assert_eq!(row.label, old_row.label);
        if let Some(original) = originals
            .iter()
            .find(|b| selected(input, b) == key.as_str())
        {
            // Actual original computed color, before legend text deduplication.
            assert_eq!(row.color, rgba(&original["numericRGBA"]));
        } else {
            assert_eq!(input["colorBy"], "subGroup");
            // Native has no summary/legend row for a reserved subgroup. Rust
            // deliberately keeps the complete legacy palette slot instead.
            assert_eq!(
                row.color,
                rgba(&input["paletteRGBA"][keys.iter().position(|k| k == key).unwrap()])
            );
        }
    }
    assert_eq!(
        actual, only_captions_changed,
        "caption map may only change legend labels"
    );
    Ok(actual)
}
fn assert_native_rows_present(input: &Value, snapshot: &Value, actual: &BoxPlotLayout) {
    let mut positions = Vec::new();
    for row in native_rows(snapshot) {
        let id = legend_id(input, text(&row["sourceKey"]));
        let (index, item) = actual
            .scene
            .legend
            .iter()
            .enumerate()
            .find(|(_, item)| item.id == id)
            .unwrap();
        assert_eq!(item.label, text(&row["label"]));
        assert_eq!(item.color, rgba(&row["rgba"]));
        positions.push(index);
    }
    assert!(
        positions.windows(2).all(|p| p[0] < p[1]),
        "actual native rows keep original numeric-domain ordering"
    );
}

#[test]
fn original_unique_captions_match_real_rows_colors_order_and_reserved_slots() -> Result<()> {
    let fixture = reference();
    let mut counts = (0, 0, 0, 0);
    for p in fixture["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["input"]["caption"]["variant"] == "unique")
    {
        let input = &p["input"];
        assert_eq!(p["restorationExact"], true);
        assert_eq!(p["baseline"], p["restored"]);
        for phase in ["baseline", "active", "restored"] {
            let original = &p[phase];
            let actual = checked_map_layout(input, original)?;
            assert_native_rows_present(input, original, &actual);
            if input["colorBy"] == "group" {
                assert_eq!(actual.scene.legend.len(), native_rows(original).len());
            } else {
                counts.3 += actual.scene.legend.len() - native_rows(original).len();
            }
            counts.1 += native_rows(original).len();
            counts.2 += actual.boxes.len();
        }
        counts.0 += 1;
    }
    assert_eq!(counts, (10, 42, 66, 24));
    Ok(())
}

#[test]
fn original_duplicate_and_empty_text_dedup_is_not_rust_identity_dedup() -> Result<()> {
    let fixture = reference();
    let mut counts = (0, 0, 0, 0);
    for p in fixture["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["input"]["caption"]["variant"] != "unique")
    {
        let input = &p["input"];
        for phase in ["baseline", "active", "restored"] {
            let native = &p[phase];
            let actual = checked_map_layout(input, native)?;
            assert_native_rows_present(input, native, &actual);
            let native_rows = native_rows(native);
            assert!(
                native_rows.len() <= 1,
                "observed real Nivo uniqBy(label) output"
            );
            let distinct_native_source_keys = native["boxes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|b| selected(input, b))
                .collect::<BTreeSet<_>>();
            if !distinct_native_source_keys.is_empty() {
                assert_eq!(native_rows.len(), 1);
                let first_source_key = domain(input)
                    .into_iter()
                    .find(|key| distinct_native_source_keys.contains(key.as_str()))
                    .unwrap();
                assert_eq!(text(&native_rows[0]["sourceKey"]), first_source_key);
            }
            assert_eq!(
                actual
                    .scene
                    .legend
                    .iter()
                    .map(|r| &r.id)
                    .collect::<BTreeSet<_>>()
                    .len(),
                actual.scene.legend.len()
            );
            assert!(
                actual
                    .scene
                    .legend
                    .iter()
                    .all(|r| r.label == text(&input["caption"]["map"][domain(input)[0].as_str()]))
            );
            counts.1 += native_rows.len();
            counts.2 += actual.scene.legend.len();
            counts.3 += usize::from(actual.scene.legend.len() > native_rows.len());
        }
        counts.0 += 1;
    }
    assert_eq!(counts, (20, 48, 132, 48));
    Ok(())
}

#[test]
fn original_partial_empty_arbitrary_callback_and_checked_map_boundaries() -> Result<()> {
    let fixture = reference();
    let qualifications = fixture["qualifications"].as_array().unwrap();
    assert_eq!(qualifications.len(), 7);
    for q in qualifications {
        let input = &q["input"];
        let native = &q["actual"];
        native_contract(input, native);
        match text(&q["id"]) {
            "group-partial-map" | "subGroup-partial-map" | "empty-raw-subgroup-explicit-empty" => {
                let actual = checked_map_layout(input, native)?;
                assert_native_rows_present(input, native, &actual);
                if q["id"] == "empty-raw-subgroup-explicit-empty" {
                    assert_eq!(actual.scene.legend[0].label, "");
                }
            }
            "empty-raw-subgroup-unmapped-fallback" => {
                assert_eq!(native_rows(native)[0]["label"], "");
                let actual = checked_map_layout(input, native)?;
                assert_eq!(actual.scene.legend[0].label, "Samples");
                assert_eq!(actual.scene.legend[0].id, boxplot_series_id(""));
                assert_eq!(
                    actual.scene.legend[0].color,
                    rgba(&native_rows(native)[0]["rgba"])
                );
            }
            "group-one-key-multiple-labels" => {
                let rows = native_rows(native);
                assert_eq!(rows.len(), 4);
                assert_eq!(rows[0]["id"], rows[1]["id"]);
                assert_eq!(rows[0]["sourceKey"], rows[1]["sourceKey"]);
                assert_ne!(rows[0]["label"], rows[1]["label"]);
                let mut opts = options(input);
                opts.legend_labels = Some(BTreeMap::from([(
                    text(&rows[0]["sourceKey"]).into(),
                    text(&rows[0]["label"]).into(),
                )]));
                let actual = boxplot_layout(&config(input), &source(input), &opts)?;
                assert_eq!(actual.scene.legend.len(), 2);
                assert_eq!(actual.scene.legend[0].label, text(&rows[0]["label"]));
                assert_ne!(actual.scene.legend[0].label, text(&rows[1]["label"]));
                // No Rust callback/one-key-multiple-label support is implied.
            }
            "native-unused-extra-map-key" | "native-caption-over-rust-byte-budget" => {
                let mut opts = options(input);
                opts.legend_labels = Some(captions(input));
                let error = boxplot_layout(&config(input), &source(input), &opts).unwrap_err();
                if q["id"] == "native-unused-extra-map-key" {
                    assert!(
                        error
                            .to_string()
                            .contains("unknown boxplot legend label key")
                    );
                    assert_eq!(native_rows(native).len(), 2);
                } else {
                    assert!(
                        error
                            .to_string()
                            .contains("boxplot legend label exceeds byte budget")
                    );
                    assert_eq!(text(&native_rows(native)[1]["label"]).len(), 1025);
                }
                opts.legend = false;
                assert!(boxplot_layout(&config(input), &source(input), &opts).is_err());
            }
            other => panic!("unreviewed original qualification: {other}"),
        }
    }
    Ok(())
}
