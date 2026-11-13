//! Key-stable caption overrides; no arbitrary legend callback or label-based
//! row deduplication is implied. Original-function fixture boundaries stay intact.
use std::collections::{BTreeMap, BTreeSet};

use kartis::*;

fn config() -> ChartConfig {
    let mut c = ChartConfig {
        width: 480.,
        height: 300.,
        ..Default::default()
    };
    c.theme.colors = vec![
        Color(90, 100, 110, 255),
        Color(20, 40, 60, 160),
        Color(200, 180, 160, 220),
        Color(10, 20, 30, 255),
    ];
    c
}
fn source() -> Vec<BoxObservation> {
    let mut source = Vec::new();
    for group in ["A", "B/北"] {
        for subgroup in ["x", ""] {
            for (i, value) in [0., 2., 4.].into_iter().enumerate() {
                source.push(BoxObservation::new(
                    format!("{group}/{subgroup}/{i}"),
                    group,
                    subgroup,
                    Some(value),
                ));
            }
        }
    }
    source.push(BoxObservation::new("none", "null", "x", None));
    source
}
fn options(mode: BoxPlotColorBy) -> BoxPlotLayoutOptions {
    BoxPlotLayoutOptions {
        color_by: mode,
        groups: Some(
            ["reserved", "B/北", "A", "null"]
                .map(str::to_string)
                .to_vec(),
        ),
        subgroups: Some(["reserved", "x", ""].map(str::to_string).to_vec()),
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
fn captions(entries: &[(&str, &str)]) -> Option<BTreeMap<String, String>> {
    Some(
        entries
            .iter()
            .map(|(key, label)| (key.to_string(), label.to_string()))
            .collect(),
    )
}
fn layout(
    c: &ChartConfig,
    data: &[BoxObservation],
    o: &BoxPlotLayoutOptions,
    scaled: bool,
) -> Result<BoxPlotLayout> {
    if scaled {
        Ok(boxplot_layout_with_scale_options(
            c,
            data,
            o,
            &ContinuousScaleOptions {
                reverse: true,
                round: true,
                ..Default::default()
            },
        )?
        .layout)
    } else {
        boxplot_layout(c, data, o)
    }
}
fn key(mode: BoxPlotColorBy) -> &'static str {
    if mode == BoxPlotColorBy::Group {
        "A"
    } else {
        "x"
    }
}
fn assert_caption(l: &BoxPlotLayout, mode: BoxPlotColorBy, text: &str) {
    let id = if mode == BoxPlotColorBy::Group {
        "A".to_string()
    } else {
        boxplot_series_id("x")
    };
    assert_eq!(
        l.scene
            .legend
            .iter()
            .find(|item| item.id == id)
            .unwrap()
            .label,
        text
    );
}

#[test]
fn none_and_empty_maps_preserve_complete_layout_svg_and_canvas_in_all_entry_profiles() {
    assert_eq!(BoxPlotLayoutOptions::default().legend_labels, None);
    for mode in [BoxPlotColorBy::SubGroup, BoxPlotColorBy::Group] {
        for orientation in [
            StatisticsOrientation::Vertical,
            StatisticsOrientation::Horizontal,
        ] {
            for scaled in [false, true] {
                let o = BoxPlotLayoutOptions {
                    orientation,
                    ..options(mode)
                };
                let none = layout(&config(), &source(), &o, scaled).unwrap();
                let empty = layout(
                    &config(),
                    &source(),
                    &BoxPlotLayoutOptions {
                        legend_labels: Some(BTreeMap::new()),
                        ..o.clone()
                    },
                    scaled,
                )
                .unwrap();
                assert_eq!(empty, none);
                assert_eq!(
                    render::to_svg(&empty.scene).unwrap(),
                    render::to_svg(&none.scene).unwrap()
                );
                assert_eq!(
                    render::to_canvas_script(&empty.scene, "canvas").unwrap(),
                    render::to_canvas_script(&none.scene, "canvas").unwrap()
                );
                if !scaled {
                    assert_eq!(
                        boxplot_with_options(&config(), &source(), &o).unwrap(),
                        none.scene
                    );
                }
                if mode == BoxPlotColorBy::Group {
                    assert_eq!(
                        none.scene
                            .legend
                            .iter()
                            .map(|i| i.label.as_str())
                            .collect::<Vec<_>>(),
                        ["B/北", "A"]
                    );
                } else {
                    assert_eq!(
                        none.scene
                            .legend
                            .iter()
                            .map(|i| i.label.as_str())
                            .collect::<Vec<_>>(),
                        ["reserved", "x", "Samples"]
                    );
                    assert!(none.scene.legend_roles.is_empty());
                }
            }
        }
    }
}

#[test]
fn explicit_partial_captions_only_change_display_text_and_keep_raw_ids_roles_and_color() {
    for mode in [BoxPlotColorBy::SubGroup, BoxPlotColorBy::Group] {
        let o = options(mode);
        let original = boxplot_layout(&config(), &source(), &o).unwrap();
        let labels = if mode == BoxPlotColorBy::Group {
            captions(&[("B/北", "North captions"), ("reserved", "not a new row")])
        } else {
            captions(&[("reserved", "Reserved samples"), ("", "Ungrouped captions")])
        };
        let actual = boxplot_layout(
            &config(),
            &source(),
            &BoxPlotLayoutOptions {
                legend_labels: labels,
                ..o
            },
        )
        .unwrap();
        let mut expected = original.clone();
        if mode == BoxPlotColorBy::Group {
            expected.scene.legend[0].label = "North captions".into();
        } else {
            expected.scene.legend[0].label = "Reserved samples".into();
            expected.scene.legend[2].label = "Ungrouped captions".into();
        }
        assert_eq!(
            actual, expected,
            "all geometry, source/statistics, mark styles, group opacity and ownership must stay identical"
        );
        for (before, after) in original.scene.legend.iter().zip(&actual.scene.legend) {
            assert_eq!((&before.id, before.color), (&after.id, after.color));
            assert_eq!(
                original.scene.legend_role(&before.id),
                actual.scene.legend_role(&after.id)
            );
        }
    }
}

#[test]
fn raw_key_domain_includes_empty_and_canonical_looking_strings_without_reinterpreting_ids() {
    let raw = boxplot_series_id("x");
    let data = [BoxObservation::new("sample", "A", raw.clone(), Some(1.))];
    let o = BoxPlotLayoutOptions {
        legend_labels: captions(&[(&raw, "Raw identity caption")]),
        ..Default::default()
    };
    let l = boxplot_layout(&config(), &data, &o).unwrap();
    assert_eq!(l.scene.legend[0].id, boxplot_series_id(&raw));
    assert_eq!(l.scene.legend[0].label, "Raw identity caption");
    assert_eq!(
        l.scene.legend_role(&l.scene.legend[0].id),
        Some(LegendRole::Series)
    );
    let bad = BoxPlotLayoutOptions {
        legend_labels: captions(&[(&boxplot_series_id(&raw), "encoded ID is not this raw key")]),
        ..o
    };
    assert_eq!(
        boxplot_layout(&config(), &data, &bad).unwrap_err().0,
        "unknown boxplot legend label key"
    );
    let empty = [BoxObservation::new("single", "A", "", Some(0.))];
    let l = boxplot_layout(
        &config(),
        &empty,
        &BoxPlotLayoutOptions {
            legend_labels: captions(&[("", "Explicit ungrouped caption")]),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(l.scene.legend[0].id, boxplot_series_id(""));
    assert_eq!(l.scene.legend[0].label, "Explicit ungrouped caption");
}

#[test]
fn reserved_hidden_nullable_and_empty_qualification_never_depends_on_caption_presence() {
    for mode in [BoxPlotColorBy::SubGroup, BoxPlotColorBy::Group] {
        let mut o = options(mode);
        o.legend_labels = if mode == BoxPlotColorBy::Group {
            captions(&[
                ("reserved", "Reserved"),
                ("null", "Nullable only"),
                ("A", "Area A"),
            ])
        } else {
            captions(&[("reserved", "Reserved"), ("x", "Hidden samples")])
        };
        o.hidden.insert(boxplot_series_id("x"));
        let hidden = boxplot_layout(&config(), &source(), &o).unwrap();
        assert!(hidden.boxes.iter().all(|b| b.subgroup.is_empty()));
        if mode == BoxPlotColorBy::Group {
            assert_eq!(
                hidden
                    .scene
                    .legend
                    .iter()
                    .map(|i| i.id.as_str())
                    .collect::<Vec<_>>(),
                ["B/北", "A"]
            );
        } else {
            assert_eq!(hidden.scene.legend.len(), 3);
            assert_eq!(hidden.scene.legend[1].label, "Hidden samples");
        }
        o.hidden.insert(boxplot_series_id(""));
        for data in [
            source(),
            vec![],
            vec![BoxObservation::new("none", "null", "x", None)],
        ] {
            let l = boxplot_layout(&config(), &data, &o).unwrap();
            assert!(l.boxes.is_empty());
            if mode == BoxPlotColorBy::Group {
                assert!(l.scene.legend.is_empty() && l.scene.legend_roles.is_empty());
            } else {
                assert_eq!(l.scene.legend.len(), 3);
                assert_eq!(l.scene.legend[1].label, "Hidden samples");
            }
        }
    }
}

#[test]
fn duplicate_and_empty_captions_preserve_each_identity_and_do_not_deduplicate_rows() {
    for mode in [BoxPlotColorBy::SubGroup, BoxPlotColorBy::Group] {
        for caption in ["", "Repeated display caption", "北/南 😀"] {
            let o = options(mode);
            let base = boxplot_layout(&config(), &source(), &o).unwrap();
            let keys = if mode == BoxPlotColorBy::Group {
                o.groups.as_ref().unwrap()
            } else {
                o.subgroups.as_ref().unwrap()
            };
            let labels = keys
                .iter()
                .map(|k| (k.clone(), caption.to_string()))
                .collect();
            let l = boxplot_layout(
                &config(),
                &source(),
                &BoxPlotLayoutOptions {
                    legend_labels: Some(labels),
                    ..o
                },
            )
            .unwrap();
            assert_eq!(
                l.scene.legend.len(),
                if mode == BoxPlotColorBy::Group { 2 } else { 3 }
            );
            assert_eq!(
                l.scene
                    .legend
                    .iter()
                    .map(|v| &v.id)
                    .collect::<BTreeSet<_>>()
                    .len(),
                l.scene.legend.len()
            );
            assert!(l.scene.legend.iter().all(|v| v.label == caption));
            assert_eq!(l.scene.legend_roles, base.scene.legend_roles);
            for (a, b) in l.scene.legend.iter().zip(&base.scene.legend) {
                assert_eq!((&a.id, a.color), (&b.id, b.color));
            }
            l.scene.validate().unwrap();
        }
    }
}

#[test]
fn unknown_domain_keys_are_rejected_before_optional_legend_emission() {
    for mode in [BoxPlotColorBy::SubGroup, BoxPlotColorBy::Group] {
        for legend in [false, true] {
            for hidden in [false, true] {
                let wrong = if mode == BoxPlotColorBy::Group {
                    "x".to_string()
                } else {
                    boxplot_series_id("x")
                };
                let o = BoxPlotLayoutOptions {
                    legend,
                    legend_labels: captions(&[(&wrong, "not in selected raw domain")]),
                    hidden: if hidden {
                        [boxplot_series_id("x"), boxplot_series_id("")].into()
                    } else {
                        Default::default()
                    },
                    ..options(mode)
                };
                for data in [source(), vec![]] {
                    let original = data.clone();
                    assert_eq!(
                        boxplot_layout(&config(), &data, &o).unwrap_err().0,
                        "unknown boxplot legend label key"
                    );
                    assert_eq!(data, original);
                }
            }
        }
    }
}

#[test]
fn utf8_byte_budget_applies_to_unused_reserved_captions_even_without_legend_or_boxes() {
    let limit = format!("{}a", "北".repeat(MAX_LABEL_BYTES / 3));
    assert_eq!(limit.len(), MAX_LABEL_BYTES);
    assert!(limit.chars().count() < MAX_LABEL_BYTES);
    for mode in [BoxPlotColorBy::SubGroup, BoxPlotColorBy::Group] {
        for legend in [false, true] {
            let o = BoxPlotLayoutOptions {
                legend,
                legend_labels: Some([("reserved".into(), limit.clone())].into()),
                hidden: [boxplot_series_id("x"), boxplot_series_id("")].into(),
                ..options(mode)
            };
            assert!(boxplot_layout(&config(), &[], &o).is_ok());
            let too_long = BoxPlotLayoutOptions {
                legend_labels: Some([("reserved".into(), format!("{limit}b"))].into()),
                ..o
            };
            assert_eq!(
                boxplot_layout(&config(), &[], &too_long).unwrap_err().0,
                "boxplot legend label exceeds byte budget"
            );
        }
    }
}

#[test]
fn real_controller_source_caption_updates_preserve_active_bindings_and_reject_atomically() {
    for mode in [BoxPlotColorBy::SubGroup, BoxPlotColorBy::Group] {
        for scaled in [false, true] {
            let o = BoxPlotLayoutOptions {
                legend_labels: captions(&[(key(mode), "Before caption")]),
                ..options(mode)
            };
            let ctl_options = BoxPlotInteractionOptions {
                opacity: 0.6,
                active_opacity: 0.8,
                inactive_opacity: 0.2,
            };
            let mut ctl = if scaled {
                BoxPlotInteraction::with_scale_options(
                    &config(),
                    &source(),
                    &o,
                    &ContinuousScaleOptions {
                        reverse: true,
                        round: true,
                        ..Default::default()
                    },
                    &ctl_options,
                )
                .unwrap()
            } else {
                BoxPlotInteraction::new(&config(), &source(), &o, &ctl_options).unwrap()
            };
            let active = BoxPlotKey::new("A", "x");
            ctl.set_active(Some(active.clone())).unwrap();
            let before = ctl.layout.clone();
            let binding = ctl.interaction_binding().unwrap();
            let after_options = BoxPlotLayoutOptions {
                legend_labels: captions(&[(key(mode), "After caption")]),
                ..o
            };
            ctl.replace_source(&config(), &source(), &after_options)
                .unwrap();
            assert_eq!(ctl.active(), Some(&active));
            assert_caption(&ctl.layout, mode, "After caption");
            assert_eq!(ctl.layout.boxes, before.boxes);
            assert_eq!(ctl.layout.scene.marks, before.scene.marks);
            assert_eq!(ctl.layout.scene.opacity_groups, before.scene.opacity_groups);
            assert_eq!(ctl.interaction_binding().unwrap(), binding);
            let accepted = ctl.layout.clone();
            let active_keys = ctl.active_keys().to_vec();
            for labels in [
                captions(&[("unknown-key", "bad")]),
                Some([(key(mode).into(), "x".repeat(MAX_LABEL_BYTES + 1))].into()),
            ] {
                let invalid = BoxPlotLayoutOptions {
                    legend_labels: labels,
                    ..after_options.clone()
                };
                assert!(ctl.replace_source(&config(), &source(), &invalid).is_err());
                assert_eq!(ctl.layout, accepted);
                assert_eq!(ctl.active(), Some(&active));
                assert_eq!(ctl.active_keys(), active_keys.as_slice());
                assert_eq!(ctl.interaction_binding().unwrap(), binding);
            }
            ctl.layout.scene.legend[0]
                .label
                .push_str(" deliberate public mutation");
            assert!(ctl.set_active(None).is_err());
            assert!(
                ctl.replace_source(&config(), &source(), &after_options)
                    .is_err()
            );
            ctl.layout = accepted;
            ctl.set_active(None).unwrap();
            assert_caption(&ctl.layout, mode, "After caption");
            let hidden = BoxPlotLayoutOptions {
                hidden: [boxplot_series_id("x")].into(),
                ..after_options
            };
            ctl.set_active(Some(active)).unwrap();
            ctl.replace_source(&config(), &source(), &hidden).unwrap();
            assert!(ctl.active().is_none());
            assert_caption(&ctl.layout, mode, "After caption");
        }
    }
}

#[cfg(feature = "geojson")]
#[test]
fn frozen_original_group_property_legends_are_consumable_without_rewriting_reference_flags() {
    use serde_json::Value;
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/box-colorby-hover/values.json")).unwrap();
    let strings = |v: &Value| {
        v.as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap().to_string())
            .collect::<Vec<_>>()
    };
    let rgba = |v: &Value| {
        Color(
            v[0].as_u64().unwrap() as u8,
            v[1].as_u64().unwrap() as u8,
            v[2].as_u64().unwrap() as u8,
            v[3].as_u64().unwrap() as u8,
        )
    };
    let mut compared = 0;
    for case in fixture["profiles"].as_array().unwrap().iter().filter(|p| {
        p["input"]["colorBy"] == "group" && p["input"]["legendLabel"] == "group-property"
    }) {
        let input = &case["input"];
        if input["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["v"].is_null())
        {
            continue;
        }
        assert_eq!(
            case["rustScope"]["legendLabelSupported"], false,
            "the frozen historical arbitrary-accessor scope remains unchanged"
        );
        let data = input["data"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, d)| {
                BoxObservation::new(
                    format!("native/{i}"),
                    d["group"].as_str().unwrap(),
                    d["subGroup"].as_str().unwrap(),
                    d["v"].as_f64(),
                )
            })
            .collect::<Vec<_>>();
        let expected = case["baseline"]["legendsData"][0]["data"]
            .as_array()
            .unwrap();
        let labels = expected
            .iter()
            .map(|d| {
                (
                    d["sourceKey"].as_str().unwrap().to_string(),
                    d["label"].as_str().unwrap().to_string(),
                )
            })
            .collect();
        let mut c = config();
        c.theme.colors = input["paletteRGBA"]
            .as_array()
            .unwrap()
            .iter()
            .map(rgba)
            .collect();
        let o = BoxPlotLayoutOptions {
            color_by: BoxPlotColorBy::Group,
            groups: Some(strings(&input["groups"])),
            subgroups: Some(strings(&input["subGroups"])),
            legend_labels: Some(labels),
            ..Default::default()
        };
        let l = boxplot_layout(&c, &data, &o).unwrap();
        assert_eq!(l.scene.legend.len(), expected.len());
        for (actual, native) in l.scene.legend.iter().zip(expected) {
            assert_eq!(actual.id, native["sourceKey"].as_str().unwrap());
            assert_eq!(actual.label, native["label"].as_str().unwrap());
            assert_eq!(actual.color, rgba(&native["rgba"]));
            assert_eq!(l.scene.legend_role(&actual.id), Some(LegendRole::Category));
        }
        compared += 1;
    }
    assert_eq!(compared, 6);
}

#[cfg(feature = "geojson")]
#[test]
fn native_cross_dimension_label_collisions_are_recorded_not_forged_into_series_aliases() {
    use serde_json::Value;
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/box-colorby-hover/values.json")).unwrap();
    let native = fixture["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "subGroup-authored-complete-group-property")
        .unwrap();
    let rows = native["baseline"]["legendsData"][0]["data"]
        .as_array()
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["id"], rows[1]["id"]);
    assert_eq!(rows[0]["sourceKey"], rows[1]["sourceKey"]);
    assert_ne!(rows[0]["label"], rows[1]["label"]);
    let l = boxplot_layout(
        &config(),
        &source(),
        &BoxPlotLayoutOptions {
            legend_labels: captions(&[("reserved", "Common"), ("x", "Common"), ("", "Common")]),
            ..options(BoxPlotColorBy::SubGroup)
        },
    )
    .unwrap();
    assert_eq!(l.scene.legend.len(), 3);
    assert_eq!(
        l.scene
            .legend
            .iter()
            .map(|i| &i.id)
            .collect::<BTreeSet<_>>()
            .len(),
        3
    );
    assert!(l.scene.legend.iter().all(|i| i.label == "Common"));
    assert!(l.scene.legend_roles.is_empty());
}
