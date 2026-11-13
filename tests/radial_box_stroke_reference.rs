#![cfg(feature = "geojson")]
//! Actual original hook/transition/renderer fixtures, not shared-resolver-only witnesses.
//! Color-role wiring scope: explicit source palettes; no claim of native defaults,
//! native hidden filtering, arbitrary property access, geometry or animation parity.
use glory::render::{GraphicStroke, StrokeCap};
use kartis::*;
use serde_json::Value;

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/radial-box-stroke/values.json")).unwrap()
}
fn rgba(v: &Value) -> Color {
    Color(
        v[0].as_u64().unwrap() as u8,
        v[1].as_u64().unwrap() as u8,
        v[2].as_u64().unwrap() as u8,
        v[3].as_u64().unwrap() as u8,
    )
}
fn stroke(color: Color, width: f64) -> GraphicStroke {
    GraphicStroke {
        color,
        width,
        cap: StrokeCap::Butt,
    }
}
fn profile(name: &str, role: &str) -> InheritedColor {
    match name {
        "native-default" => InheritedColor::FromContext(match role {
            "radial" => vec![ColorModifier::Darker(1.)],
            "median" => vec![ColorModifier::Darker(2.)],
            _ => vec![],
        }),
        "inherit" => InheritedColor::FromContext(vec![]),
        "darker" => InheritedColor::FromContext(vec![ColorModifier::Darker(1.)]),
        "ordered" => InheritedColor::FromContext(vec![
            ColorModifier::Brighter(1.),
            ColorModifier::Opacity(0.5),
            ColorModifier::Darker(1.),
        ]),
        "alpha0-visible" => {
            InheritedColor::FromContext(vec![ColorModifier::Darker(1.), ColorModifier::Opacity(1.)])
        }
        "fixed" => InheritedColor::Fixed(Color(12, 34, 56, 78)),
        "theme" => InheritedColor::Theme,
        _ => panic!("arbitrary native property accessor has no typed role counterpart"),
    }
}
fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn original_radial_whole_hook_transition_colors_reach_actual_segment_strokes() -> Result<()> {
    let all = fixture();
    let cases = all["radial"].as_array().unwrap();
    assert_eq!(cases.len(), 32);
    let (mut compared, mut alpha0_boundaries, mut nested, mut zeros, mut nulls, mut disabled) =
        (0, 0, 0, 0, 0, 0);
    for case in cases {
        let rows = case["bars"].as_array().unwrap();
        let name = case["profile"].as_str().unwrap();
        if name == "nested" {
            nested += rows.len();
            assert!(
                rows.iter()
                    .all(|r| !r["rustComparableRole"].as_bool().unwrap())
            );
            continue;
        }
        let categories = strings(&case["categories"]);
        let groups = strings(&case["groups"]);
        let source = groups
            .iter()
            .map(|group| {
                BarSeries::new(
                    group.clone(),
                    categories
                        .iter()
                        .map(|category| {
                            rows.iter()
                                .find(|r| {
                                    r["groupId"] == group.as_str()
                                        && r["category"] == category.as_str()
                                })
                                .and_then(|r| r["rawValue"].as_f64())
                        })
                        .collect(),
                )
            })
            .collect::<Vec<_>>();
        let options = StackedRadialBarOptions {
            colors: rows
                .iter()
                .map(|r| {
                    (
                        r["category"].as_str().unwrap().to_owned(),
                        rgba(&r["numericRGBA"]),
                    )
                })
                .collect(),
            // Native width zero still evaluates its role. Rust's radial API
            // uses None to disable the border and rejects Some(width=0).
            // Check no-stroke output for these cases, without pretending to
            // observe a disabled role's RGB through an enabled outline.
            border: (case["borderWidth"].as_f64().unwrap() > 0.).then(|| {
                stroke(
                    Color(70, 80, 90, 123),
                    case["borderWidth"].as_f64().unwrap(),
                )
            }),
            border_color_profile: Some(profile(name, "radial")),
            tracks: false,
            legend: false,
            ..Default::default()
        };
        let layout =
            layout_stacked_radial_bar(&ChartConfig::default(), &categories, &source, &options)?;
        for row in rows {
            let segment = layout.segments.iter().find(|s| {
                s.ring_id == row["groupId"].as_str().unwrap()
                    && s.category == row["category"].as_str().unwrap()
            });
            let Some(raw) = row["rawValue"].as_f64() else {
                nulls += 1;
                assert!(
                    segment.is_none(),
                    "native computed null is not a Rust Some(0)"
                );
                continue;
            };
            let segment = segment.unwrap();
            assert_eq!(segment.value, raw);
            if raw == 0. {
                zeros += 1;
                assert!(!segment.visible);
                assert!(!layout.scene.marks.iter().any(|m| m.id == segment.mark_id));
                continue;
            }
            let mark = layout
                .scene
                .marks
                .iter()
                .find(|m| m.id == segment.mark_id)
                .unwrap();
            if options.border.is_none() {
                assert!(mark.stroke.is_none());
                disabled += 1;
                continue;
            }
            let actual = mark.stroke.unwrap();
            assert_eq!(actual.width, options.border.unwrap().width);
            assert_eq!(mark.opacity, 1.);
            let expected = rgba(&row["roleBorderRGBA"]);
            if row["cssAlpha0ParserBoundary"].as_bool().unwrap() {
                alpha0_boundaries += 1;
                assert!(!row["rustComparableRole"].as_bool().unwrap());
                assert_ne!(
                    actual.color, expected,
                    "typed zero-alpha RGB is deliberately retained"
                );
                assert_eq!(actual.color.3, expected.3);
            } else {
                assert!(row["rustComparableRole"].as_bool().unwrap());
                assert_eq!(actual.color, expected, "{} / {}", case["id"], row["id"]);
                compared += 1;
            }
        }
    }
    assert_eq!(
        (compared, alpha0_boundaries, nested, zeros, nulls, disabled),
        (24, 4, 16, 28, 28, 28)
    );
    Ok(())
}

#[test]
fn original_subgroup_box_transition_colors_reach_border_median_whiskers_and_caps() -> Result<()> {
    let all = fixture();
    let (mut compared, mut alpha0_boundaries, mut group_mode_roles) = (0, 0, 0);
    for case in all["box"].as_array().unwrap() {
        let rows = case["boxes"].as_array().unwrap();
        if case["colorBy"] != "subGroup" {
            assert!(!case["rustComparableMode"].as_bool().unwrap());
            group_mode_roles += rows.len() * 3;
            continue;
        }
        assert!(case["rustComparableMode"].as_bool().unwrap());
        let mut source = Vec::new();
        for row in rows {
            // Every main fixture box really comes from two observations; its
            // original source extrema reconstruct those inputs, not quantiles.
            assert_eq!(row["sourceSummary"]["n"], 2);
            for (i, raw) in row["sourceSummary"]["extrema"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
            {
                source.push(BoxObservation::new(
                    format!("{}:{i}", row["key"]),
                    row["group"].as_str().unwrap(),
                    row["subGroup"].as_str().unwrap(),
                    raw.as_f64(),
                ));
            }
        }
        let name = case["profile"].as_str().unwrap();
        let options = BoxPlotLayoutOptions {
            groups: Some(vec!["A".into(), "B".into(), "C".into()]),
            subgroups: Some(vec!["x".into(), "y".into(), "z".into()]),
            colors: rows
                .iter()
                .map(|r| {
                    (
                        r["subGroup"].as_str().unwrap().to_owned(),
                        rgba(&r["numericRGBA"]),
                    )
                })
                .collect(),
            orientation: if case["layout"] == "horizontal" {
                StatisticsOrientation::Horizontal
            } else {
                StatisticsOrientation::Vertical
            },
            // Main original profiles explicitly enable a real border; the
            // separate qualification cases retain the native width-zero path.
            border: Some(stroke(
                Color(70, 80, 90, 123),
                case["widths"]["border"].as_f64().unwrap(),
            )),
            median: stroke(Color(120, 20, 80, 90), 2.),
            whisker: stroke(Color(20, 90, 120, 200), 2.),
            border_color_profile: Some(profile(name, "border")),
            median_color_profile: Some(profile(name, "median")),
            whisker_color_profile: Some(profile(name, "whisker")),
            fill_opacity: 0.8,
            legend: false,
            ..Default::default()
        };
        assert_eq!(case["widths"]["border"], 2);
        let layout = boxplot_layout(&ChartConfig::default(), &source, &options)?;
        assert_eq!(layout.boxes.len(), rows.len());
        for row in rows {
            let b = layout
                .boxes
                .iter()
                .find(|b| {
                    b.group == row["group"].as_str().unwrap()
                        && b.subgroup == row["subGroup"].as_str().unwrap()
                })
                .unwrap();
            assert_eq!(b.count, 2);
            for role in ["border", "median", "whisker"] {
                let native = &row["roles"][role];
                let suffixes: &[&str] = match role {
                    "whisker" => &["whiskers", "caps"],
                    "median" => &["median"],
                    _ => &["border"],
                };
                for suffix in suffixes {
                    let id = format!("{}:{suffix}", b.id);
                    let mark = layout.scene.marks.iter().find(|m| m.id == id).unwrap();
                    let color = mark.stroke.unwrap().color;
                    assert_eq!(
                        mark.opacity, 1.,
                        "role alpha is independent of fill opacity"
                    );
                    if native["cssAlpha0ParserBoundary"].as_bool().unwrap() {
                        assert!(!native["rustComparable"].as_bool().unwrap());
                        assert_ne!(color, rgba(&native["rgba"]));
                        assert_eq!(color.3, rgba(&native["rgba"]).3);
                    } else {
                        assert!(native["rustComparable"].as_bool().unwrap());
                        assert_eq!(
                            color,
                            rgba(&native["rgba"]),
                            "{} / {} / {role}",
                            case["id"],
                            row["key"]
                        );
                    }
                }
                if native["cssAlpha0ParserBoundary"].as_bool().unwrap() {
                    alpha0_boundaries += 1;
                } else {
                    compared += 1;
                }
            }
        }
    }
    assert_eq!(
        (compared, alpha0_boundaries, group_mode_roles),
        (190, 20, 210)
    );
    Ok(())
}

#[test]
fn native_nullable_property_hidden_and_alpha0_boundaries_are_retained_explicitly() -> Result<()> {
    let all = fixture();
    let cases = all["qualificationCases"].as_array().unwrap();
    assert_eq!(cases.len(), 11);
    let find = |id: &str| cases.iter().find(|c| c["id"] == id).unwrap();
    let native_null = &find("box-native-null-number-zero")["boxes"][0]["sourceSummary"];
    assert_eq!(native_null["n"], 3);
    assert_eq!(native_null["extrema"][0], 0);
    let input = find("box-native-null-number-zero")["input"]
        .as_array()
        .unwrap();
    let source = input
        .iter()
        .enumerate()
        .map(|(i, r)| {
            BoxObservation::new(
                format!("nullable:{i}"),
                r["group"].as_str().unwrap(),
                r["subgroup"]
                    .as_str()
                    .or_else(|| r["subGroup"].as_str())
                    .unwrap(),
                r["v"].as_f64(),
            )
        })
        .collect::<Vec<_>>();
    let rust = boxplot_layout(&ChartConfig::default(), &source, &Default::default())?;
    assert_eq!(rust.boxes.len(), 1);
    assert_eq!(rust.boxes[0].count, 2);
    assert_eq!(rust.boxes[0].extrema, [2., 4.]);
    assert_eq!(
        find("box-upstream-filter-hidden-B")["boxes"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        find("radial-upstream-filter-hidden-later")["bars"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(find("radial-null-only")["bars"][0]["value"].is_null());
    assert_eq!(find("radial-zero-only")["bars"][0]["value"], 0);
    assert_eq!(all["nativeErrors"].as_array().unwrap().len(), 6);
    let props = all["propertyBoundaries"].as_array().unwrap();
    assert_eq!(props.len(), 3);
    assert_eq!(props[0]["returned"]["nativeType"], "number");
    assert_eq!(props[0]["returned"]["value"], 123);
    assert_eq!(rgba(&props[1]["returned"]), Color(9, 8, 7, 0));
    assert_eq!(props[2]["returned"]["nativeType"], "undefined");
    assert!(props[2]["returned"]["value"].is_null());
    // Group colorBy and arbitrary paths are recorded native behavior; this
    // suite neither invents those APIs nor claims Rust Contrast is native.
    Ok(())
}
