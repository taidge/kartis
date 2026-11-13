#![cfg(feature = "geojson")]
//! Expected colors, legends, active keys and opacity come from complete original
//! Nivo hooks, not a Rust copy of their algorithms. Native boundaries stay explicit.
use kartis::*;
use serde_json::Value;
use std::collections::BTreeSet;

fn reference() -> Value {
    serde_json::from_str(include_str!("fixtures/box-colorby-hover/values.json")).unwrap()
}
fn text(v: &Value) -> &str {
    v.as_str().unwrap()
}
fn number(v: &Value) -> f64 {
    v.as_f64().unwrap()
}
fn rgba(v: &Value) -> Color {
    Color(
        v[0].as_u64().unwrap() as u8,
        v[1].as_u64().unwrap() as u8,
        v[2].as_u64().unwrap() as u8,
        v[3].as_u64().unwrap() as u8,
    )
}
fn near(a: f64, b: f64) {
    assert!(
        (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.),
        "{a} != {b}"
    );
}
fn keys(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|v| text(v).to_owned())
        .collect()
}
fn source(input: &Value) -> Vec<BoxObservation> {
    input["data"]
        .as_array()
        .unwrap()
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
fn config(input: &Value) -> ChartConfig {
    // Exact public dimensions configured by the original producer; original
    // positions are plot-local, hence zero margins in the Rust counterpart.
    let mut c = ChartConfig {
        width: 480.,
        height: 300.,
        margin: Margin {
            left: 0.,
            right: 0.,
            top: 0.,
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
fn layout_options(input: &Value) -> BoxPlotLayoutOptions {
    BoxPlotLayoutOptions {
        groups: Some(keys(&input["groups"])),
        subgroups: Some(keys(&input["subGroups"])),
        color_by: if input["colorBy"] == "group" {
            BoxPlotColorBy::Group
        } else {
            BoxPlotColorBy::SubGroup
        },
        auto_domain_source: BoxPlotAutoDomainSource::SummaryQuantiles,
        padding: 0.1,
        inner_padding: 4.,
        fill_opacity: 1.,
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
fn interaction_options(input: &Value) -> BoxPlotInteractionOptions {
    BoxPlotInteractionOptions {
        opacity: number(&input["opacity"]),
        active_opacity: number(&input["activeOpacity"]),
        inactive_opacity: number(&input["inactiveOpacity"]),
    }
}
fn make(input: &Value) -> Result<BoxPlotInteraction> {
    BoxPlotInteraction::new(
        &config(input),
        &source(input),
        &layout_options(input),
        &interaction_options(input),
    )
}
fn native_keys(snapshot: &Value) -> Vec<BoxPlotKey> {
    let boxes = snapshot["boxes"].as_array().unwrap();
    snapshot["activeKeys"]
        .as_array()
        .unwrap()
        .iter()
        .map(|key| {
            // Resolve the actual original numeric key through original boxes;
            // never reconstruct OR association as the expected active set.
            let b = boxes.iter().find(|b| b["key"] == *key).unwrap();
            BoxPlotKey::new(text(&b["group"]), text(&b["subGroup"]))
        })
        .collect()
}
fn active_key(snapshot: &Value) -> BoxPlotKey {
    let item = &snapshot["sourceActiveItem"];
    BoxPlotKey::new(text(&item["group"]), text(&item["subGroup"]))
}
fn group_opacity(frame: &BoxPlotLayout, id: &str) -> f64 {
    frame
        .scene
        .opacity_groups
        .iter()
        .find(|g| g.id == id)
        .map_or(1., |g| g.opacity)
}
fn assert_factors(frame: &BoxPlotLayout, native: &Value) {
    for b in &frame.boxes {
        let expected = native["boxes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["group"] == b.group.as_str() && x["subGroup"] == b.subgroup.as_str())
            .unwrap();
        near(group_opacity(frame, &b.id), number(&expected["opacity"]));
    }
}
fn assert_layout(layout: &BoxPlotLayout, input: &Value, native: &Value) {
    let originals = native["boxes"].as_array().unwrap();
    assert_eq!(layout.boxes.len(), originals.len());
    for b in &layout.boxes {
        let expected = originals
            .iter()
            .find(|x| x["group"] == b.group.as_str() && x["subGroup"] == b.subgroup.as_str())
            .unwrap();
        assert!(expected["rustNullableSourceComparable"].as_bool().unwrap());
        let stats = &expected["sourceSummary"];
        assert_eq!(b.count, stats["n"].as_u64().unwrap() as usize);
        for (actual, expected) in b.extrema.iter().zip(stats["extrema"].as_array().unwrap()) {
            near(*actual, number(expected));
        }
        for (actual, expected) in b.quantiles.iter().zip(stats["values"].as_array().unwrap()) {
            near(*actual, number(expected));
        }
        near(b.mean, number(&stats["mean"]));
        near(b.whiskers[0], number(&stats["values"][0]));
        near(b.whiskers[1], number(&stats["values"][4]));
        let pos = &expected["position"];
        for (actual, key) in [
            (b.rect.x, "x"),
            (b.rect.y, "y"),
            (b.rect.width, "width"),
            (b.rect.height, "height"),
        ] {
            near(actual, number(&pos[key]));
        }
        let mark = layout.scene.marks.iter().find(|m| m.id == b.id).unwrap();
        assert_eq!(mark.fill, Some(rgba(&expected["numericRGBA"])));
        assert_eq!(mark.series, boxplot_series_id(&b.subgroup));
        near(mark.value, b.quantiles[2]);
        near(mark.opacity, 1.);
        let actual_ids = b.source_ids.iter().cloned().collect::<BTreeSet<_>>();
        let raw_ids = input["data"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| {
                r["group"] == b.group.as_str()
                    && r["subGroup"] == b.subgroup.as_str()
                    && !r["v"].is_null()
            })
            .map(|r| text(&r["id"]).to_owned())
            .collect::<BTreeSet<_>>();
        assert_eq!(actual_ids, raw_ids);
    }
}

#[test]
fn actual_original_boxes_legends_and_hover_match_nonnullable_profiles() -> Result<()> {
    let fixture = reference();
    let profiles = fixture["profiles"].as_array().unwrap();
    assert_eq!(profiles.len(), 32);
    let (
        mut checked,
        mut box_count,
        mut group_legends,
        mut subgroup_legends,
        mut subgroup_reserved,
    ) = (0, 0, 0, 0, 0);
    for profile in profiles
        .iter()
        .filter(|p| p["rustScope"]["nullableStatisticsComparable"] == true)
    {
        let input = &profile["input"];
        let mut chart = make(input)?;
        assert_layout(&chart.layout, input, &profile["baseline"]);
        assert_factors(&chart.layout, &profile["baseline"]);
        let domain = if input["colorBy"] == "group" {
            &input["groups"]
        } else {
            &input["subGroups"]
        };
        assert_eq!(profile["baseline"]["ordinalDomain"], *domain);
        assert_eq!(
            profile["baseline"]["legendsData"][0]["data"],
            profile["baseline"]["legendsData"][1]["data"]
        );
        if input["legendLabel"] == "default" {
            let original = profile["baseline"]["legendsData"][0]["data"]
                .as_array()
                .unwrap();
            if input["colorBy"] == "group" {
                assert_eq!(chart.layout.scene.legend.len(), original.len());
                for (actual, expected) in chart.layout.scene.legend.iter().zip(original) {
                    assert_eq!(actual.id, text(&expected["sourceKey"]));
                    assert_eq!(actual.label, text(&expected["label"]));
                    assert_eq!(actual.color, rgba(&expected["rgba"]));
                    assert_eq!(
                        chart.layout.scene.legend_role(&actual.id),
                        Some(LegendRole::Category)
                    );
                    assert_eq!(
                        input["groups"][expected["id"].as_u64().unwrap() as usize],
                        expected["sourceKey"]
                    );
                }
                group_legends += 1;
            } else {
                // Preserve the existing Rust complete-domain Series legend.
                // Original native default legend only includes actual boxes.
                assert_eq!(
                    chart.layout.scene.legend.len(),
                    input["subGroups"].as_array().unwrap().len()
                );
                assert!(chart.layout.scene.legend_roles.is_empty());
                for (i, actual) in chart.layout.scene.legend.iter().enumerate() {
                    let key = text(&input["subGroups"][i]);
                    assert_eq!(actual.id, boxplot_series_id(key));
                    assert_eq!(actual.label, key);
                    assert_eq!(
                        actual.color,
                        rgba(
                            &input["paletteRGBA"]
                                [i % input["paletteRGBA"].as_array().unwrap().len()]
                        )
                    );
                    assert_eq!(
                        chart.layout.scene.legend_role(&actual.id),
                        Some(LegendRole::Series)
                    );
                }
                for expected in original {
                    let actual = chart
                        .layout
                        .scene
                        .legend
                        .iter()
                        .find(|l| l.id == boxplot_series_id(text(&expected["sourceKey"])))
                        .unwrap();
                    assert_eq!(actual.label, text(&expected["label"]));
                    assert_eq!(actual.color, rgba(&expected["rgba"]));
                }
                assert_eq!(chart.layout.scene.legend.len(), original.len() + 1);
                subgroup_reserved += 1;
                subgroup_legends += 1;
            }
        } else {
            assert!(
                !profile["rustScope"]["legendLabelSupported"]
                    .as_bool()
                    .unwrap()
            );
            // The source's explicit property getter is recorded, not passed to
            // an invented Rust label-accessor API. Geometry/colors/hover still compare.
        }
        let baseline = chart.layout.clone();
        let svg = render::to_svg(&baseline.scene)?;
        let canvas = render::to_canvas_script(&baseline.scene, "reference")?;
        chart.set_active(Some(active_key(&profile["active"])))?;
        assert_eq!(
            chart.active_keys(),
            native_keys(&profile["active"]).as_slice()
        );
        assert_factors(&chart.layout, &profile["active"]);
        assert_eq!(chart.layout.boxes, baseline.boxes);
        assert_eq!(
            chart.layout.scene.marks, baseline.scene.marks,
            "group opacity must not premultiply member paint"
        );
        chart.set_active(None)?;
        assert!(chart.active().is_none());
        assert!(chart.active_keys().is_empty());
        assert_eq!(chart.layout, baseline);
        assert_eq!(render::to_svg(&chart.layout.scene)?, svg);
        assert_eq!(
            render::to_canvas_script(&chart.layout.scene, "reference")?,
            canvas
        );
        assert!(profile["restorationExact"].as_bool().unwrap());
        box_count += chart.layout.boxes.len();
        checked += 1;
    }
    assert_eq!(
        (
            checked,
            box_count,
            group_legends,
            subgroup_legends,
            subgroup_reserved
        ),
        (24, 80, 6, 6, 6)
    );
    Ok(())
}

#[test]
fn original_nullable_source_differences_are_verified_without_synthesizing_zero_boxes() -> Result<()>
{
    let fixture = reference();
    let mut checked = 0;
    for profile in fixture["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["rustScope"]["nullableStatisticsComparable"] == false)
    {
        let mut chart = make(&profile["input"])?;
        let native = profile["baseline"]["boxes"].as_array().unwrap();
        assert_eq!(native.len(), 5);
        assert_eq!(chart.layout.boxes.len(), 4);
        let mixed = native
            .iter()
            .find(|b| b["group"] == "B" && b["subGroup"] == "y")
            .unwrap();
        assert_eq!(mixed["sourceSummary"]["n"], 3);
        assert_eq!(mixed["sourceSummary"]["extrema"][0], 0);
        assert!(!mixed["rustNullableSourceComparable"].as_bool().unwrap());
        let rust = chart
            .layout
            .boxes
            .iter()
            .find(|b| b.group == "B" && b.subgroup == "y")
            .unwrap();
        assert_eq!(rust.count, 2);
        assert_eq!(rust.extrema, [2., 4.]);
        let null_only = native
            .iter()
            .find(|b| b["group"] == "A" && b["subGroup"] == "missing")
            .unwrap();
        assert_eq!(null_only["sourceSummary"]["n"], 1);
        assert_eq!(
            null_only["sourceSummary"]["extrema"],
            serde_json::json!([0, 0])
        );
        assert!(
            !chart
                .layout
                .boxes
                .iter()
                .any(|b| b.group == "A" && b.subgroup == "missing")
        );
        let zero = chart
            .layout
            .boxes
            .iter()
            .find(|b| b.group == "B" && b.subgroup == "x")
            .unwrap();
        assert_eq!(zero.count, 2);
        assert_eq!(zero.quantiles, [0.; 5]);
        let before = chart.layout.clone();
        assert!(
            chart
                .set_active(Some(BoxPlotKey::new("A", "missing")))
                .is_err()
        );
        assert_eq!(chart.layout, before);
        checked += 1;
    }
    assert_eq!(checked, 8);
    Ok(())
}

#[test]
fn all_native_qualification_snapshots_keep_typed_identity_and_unsupported_boundaries_explicit()
-> Result<()> {
    let fixture = reference();
    let qualifiers = fixture["qualifications"].as_array().unwrap();
    assert_eq!(qualifiers.len(), 10);
    let mut checked = 0;
    for q in qualifiers {
        let input = &q["input"];
        let native = &q["actual"];
        let mut chart = make(input)?;
        let old = chart.layout.clone();
        let native_active = native_keys(native);
        match text(&q["id"]) {
            "empty-complete-domain" => {
                assert!(native["boxes"].as_array().unwrap().is_empty());
                assert!(chart.layout.boxes.is_empty());
                assert!(chart.layout.scene.legend.is_empty());
                assert!(native_active.is_empty());
                assert_eq!(native["ordinalDomain"], input["groups"]);
            }
            "upstream-filter-group-B" | "upstream-filter-subgroup-y" => {
                assert_eq!(
                    native_active.len(),
                    1,
                    "native stale original item still partially associates"
                );
                assert!(
                    chart
                        .set_active(Some(BoxPlotKey::new(
                            text(&q["sourceActiveItem"]["group"]),
                            text(&q["sourceActiveItem"]["subGroup"])
                        )))
                        .is_err()
                );
                assert_eq!(chart.layout, old);
                assert!(chart.active_keys().is_empty());
            }
            "stale-key-same-identities" => {
                assert_eq!(q["sourceActiveItem"]["key"], "not-an-original-key");
                // Rust's key has group/subgroup only, so the matching existing
                // identities remain valid; no fake claim that this must reject.
                chart.set_active(Some(active_key(native)))?;
                assert_eq!(chart.active_keys(), native_active.as_slice());
                assert_eq!(native_active.len(), 3);
                assert_factors(&chart.layout, native);
            }
            "stale-group-existing-subgroup" | "stale-both-identities" => {
                let partial = q["id"] == "stale-group-existing-subgroup";
                assert_eq!(native_active.len(), if partial { 2 } else { 0 });
                assert!(chart.set_active(Some(active_key(native))).is_err());
                assert_eq!(chart.layout, old);
                if !partial {
                    assert!(
                        native["boxes"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .all(|b| b["opacity"] == input["inactiveOpacity"])
                    );
                }
            }
            "malformed-empty-active" => {
                assert!(q["sourceActiveItem"].as_object().unwrap().is_empty());
                assert!(native_active.is_empty());
                assert!(
                    native["boxes"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .all(|b| b["opacity"] == input["inactiveOpacity"])
                );
                assert!(chart.active().is_none());
                assert!(chart.active_keys().is_empty());
                // A JavaScript {} active object has no representation as a
                // current typed source identity. Do not invent a conversion.
            }
            "explicit-callback-legend-pair" => {
                assert_eq!(input["legendLabel"], "callback-pair");
                let legends = native["legendsData"][0]["data"].as_array().unwrap();
                assert_eq!(legends.len(), 4);
                assert_eq!(chart.layout.scene.legend.len(), 2);
                assert!(legends.iter().all(|l| text(&l["label"]).contains(" / ")));
                let ids = legends
                    .iter()
                    .map(|l| l["id"].as_u64().unwrap())
                    .collect::<BTreeSet<_>>();
                assert_eq!(ids.len(), 2);
            }
            "native-null-number-zero" => {
                assert_eq!(native["boxes"].as_array().unwrap().len(), 5);
                assert_eq!(chart.layout.boxes.len(), 4);
                assert!(
                    native["boxes"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|b| b["sourceSummary"]["n"] == 1)
                );
            }
            "same-name-group-and-subgroup" => {
                assert_layout(&chart.layout, input, native);
                assert_eq!(chart.layout.boxes.len(), 3);
                let identities = chart
                    .layout
                    .boxes
                    .iter()
                    .map(|b| BoxPlotKey::new(&b.group, &b.subgroup))
                    .collect::<BTreeSet<_>>();
                assert_eq!(identities.len(), 3);
                assert!(identities.contains(&BoxPlotKey::new("same", "other")));
                assert!(identities.contains(&BoxPlotKey::new("other", "same")));
            }
            _ => panic!("unreviewed native qualification"),
        }
        checked += 1;
    }
    assert_eq!(checked, 10);
    Ok(())
}
