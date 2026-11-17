#![cfg(feature = "geojson")]
use glory::render::{GraphicStroke, StrokeCap};
use kartis::*;
use serde_json::Value;

fn rgba(value: &Value) -> Color {
    let v = value.as_array().unwrap();
    Color(
        v[0].as_u64().unwrap() as u8,
        v[1].as_u64().unwrap() as u8,
        v[2].as_u64().unwrap() as u8,
        v[3].as_u64().unwrap() as u8,
    )
}
fn profile(name: &str, heat: bool) -> InheritedColor {
    match name {
        "native-default" if heat => InheritedColor::FromContext(vec![ColorModifier::Darker(0.8)]),
        "native-default" | "inherit" => InheritedColor::FromContext(vec![]),
        "darker" => InheritedColor::FromContext(vec![ColorModifier::Darker(1.)]),
        "ordered" => InheritedColor::FromContext(vec![
            ColorModifier::Brighter(1.),
            ColorModifier::Opacity(0.5),
            ColorModifier::Darker(1.),
        ]),
        "fixed" => InheritedColor::Fixed(Color(12, 34, 56, 78)),
        "theme" => InheritedColor::Theme,
        _ => panic!("unknown native profile"),
    }
}
fn fixed() -> GraphicStroke {
    GraphicStroke {
        color: Color(70, 80, 90, 123),
        width: 2.,
        cap: StrokeCap::Butt,
    }
}
fn values() -> Value {
    serde_json::from_str(include_str!("fixtures/border-color-role/values.json")).unwrap()
}
#[test]
fn actual_original_bar_hook_and_resolver_results_reach_ordinary_borders() -> Result<()> {
    let all = values();
    let rows = all["bar"].as_array().unwrap();
    assert_eq!(rows.len(), 72);
    let categories = ["one", "zero", "null"].map(str::to_owned).to_vec();
    let source = [
        BarSeries::new("color", vec![Some(2.), Some(0.), None]),
        BarSeries::new("b", vec![Some(3.), None, None]),
    ];
    let style = BarStyleOptions {
        series_order: Some(vec!["color".into(), "b".into()]),
        colors: [
            ("color".into(), Color(20, 40, 60, 255)),
            ("b".into(), Color(100, 120, 140, 255)),
        ]
        .into(),
        ..Default::default()
    };
    let mut checked = 0;
    for layout in ["vertical", "horizontal"] {
        for group in ["grouped", "stacked"] {
            for name in [
                "native-default",
                "inherit",
                "darker",
                "ordered",
                "fixed",
                "theme",
            ] {
                let options = BarOptions {
                    horizontal: layout == "horizontal",
                    layout: if group == "stacked" {
                        BarLayout::Stacked
                    } else {
                        BarLayout::Grouped
                    },
                    border: Some(fixed()),
                    border_color_profile: Some(profile(name, false)),
                    ..Default::default()
                };
                let actual = bar_layout_with_style(
                    &ChartConfig::default(),
                    &categories,
                    &source,
                    &options,
                    &style,
                )?;
                assert_eq!(actual.bars.len(), 3);
                for row in rows.iter().filter(|r| {
                    r["layout"] == layout && r["groupMode"] == group && r["profile"] == name
                }) {
                    let datum = actual
                        .bars
                        .iter()
                        .find(|b| {
                            b.series == row["series"].as_str().unwrap()
                                && b.category == row["index"].as_str().unwrap()
                        })
                        .unwrap();
                    assert_eq!(datum.raw_value, row["raw"].as_f64().unwrap());
                    let mark = actual
                        .scene
                        .marks
                        .iter()
                        .find(|m| m.id == datum.id)
                        .unwrap();
                    assert_eq!(
                        mark.stroke,
                        Some(GraphicStroke {
                            color: rgba(&row["actualBorderRGBA"]),
                            ..fixed()
                        })
                    );
                    checked += 1;
                }
            }
        }
    }
    assert_eq!(checked, 72);
    Ok(())
}
#[test]
fn actual_original_heat_style_null_zero_and_active_results_reach_ordinary_borders() -> Result<()> {
    let all = values();
    let rows = all["heatmap"].as_array().unwrap();
    assert_eq!(rows.len(), 36);
    let source = [
        HeatCell::new("r", "low", 0.),
        HeatCell::new("r", "high", 10.),
        HeatCell {
            row: "r".into(),
            column: "null".into(),
            value: None,
        },
    ];
    let mut checked = 0;
    for active in [false, true] {
        for name in [
            "native-default",
            "inherit",
            "darker",
            "ordered",
            "fixed",
            "theme",
        ] {
            let options = HeatmapLayoutOptions {
                labels: false,
                axes: HeatmapAxes::none(),
                opacity: 0.4,
                active_opacity: 1.,
                inactive_opacity: 0.15,
                active: active.then(|| HeatmapCellKey::new("r", "null")),
                hover_target: HeatmapHoverTarget::Cell,
                border: Some(fixed()),
                border_color_profile: Some(profile(name, true)),
                colors: ColorScaleOptions {
                    domain: Some([0., 10.]),
                    colors: vec![Color(20, 40, 60, 160), Color(200, 180, 160, 220)],
                    unknown: Color(90, 100, 110, 64),
                    ..Default::default()
                },
                ..Default::default()
            };
            let actual = heatmap_layout(&ChartConfig::default(), &source, &options)?;
            for row in rows.iter().filter(|r| {
                r["profile"] == name && r["activeIds"].as_array().unwrap().is_empty() != active
            }) {
                let id = HeatmapCellKey::new("r", row["column"].as_str().unwrap()).mark_id();
                let mark = actual.scene.marks.iter().find(|m| m.id == id).unwrap();
                assert_eq!(
                    mark.stroke,
                    Some(GraphicStroke {
                        color: rgba(&row["actualBorderRGBA"]),
                        ..fixed()
                    })
                );
                assert_eq!(mark.opacity, row["opacity"].as_f64().unwrap());
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 36);
    Ok(())
}
