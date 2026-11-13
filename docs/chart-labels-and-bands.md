# Radial label colors, Sunburst text and BoxPlot bands

These explicit options use the ordinary shared Scene, SVG and Canvas renderers. Existing entry points and default options retain their previous output. The complete example below checks the source values, geometry and interaction metadata as it changes presentation.

| Chart | Configuration | Effect |
| --- | --- | --- |
| RadialBar | `RadialBarLabels.color_profile: Option<InheritedColor>` | Resolve label color for each original category palette color. |
| Sunburst | `sunburst_layout_with_label_factory` | Build owned label text from the original datum and computed node. |
| BoxPlot | `BoxPlotLayoutOptions.category_scale: Option<BandScaleOptions>` | Configure full group-band padding, alignment and rounding. |

For RadialBar, `None` retains `labels.color` or the theme text color. `Some` accepts the shared Theme, Fixed, FromContext and Contrast roles. Theme uses that same fallback; inherited and surface use the original category palette, and backdrop uses the chart background. Custom fill paints do not replace that palette context. Label alpha remains independent of segment opacity: the example's segments have opacity 0.4 while inherited labels remain opaque. Modifier validation also applies with no rendered labels. Context-dependent resolution checks categories with an actual `Some` source value, including hidden, zero and threshold-skipped segments; absent sparse values do not fabricate a context.

The Sunburst callback borrows `SunburstLabelContext`: original `datum`, computed `node`, `default_text`, actual baseline-adjusted `label_position`, and plot. The node provides source index, depth, aggregate value, formatted value, arc and logical anchor. Its mark exists when the callback runs; its label index has not been assigned. Callbacks run once for eligible owned labels in breadth-first order with authored sibling order. Disabled labels, hidden/root-excluded/zero nodes and labels rejected by ordinary arc or skip rules do not invoke them. These are Rust label-slot counts, rather than a promise of Nivo accessor invocation counts.

Returning an empty string retains the owned label slot and draws no glyphs. Each result is checked immediately against the existing 1,024-byte UTF-8 limit. An error discards construction; external closure side effects are not rolled back. The layout retains no callback. Custom text is captured before measured-label candidates and signatures are created, so measure those candidates after construction. Measurements for different text are rejected. Placement, controlled hiding/restoration and font changes keep their existing managed-label rules. Raw source, aggregate values, formatted values and tooltips do not change.

BoxPlot's `category_scale: Some(...)` uses the shared D3 band scale for group positions, axes, subgroup boxes, medians, whiskers and outliers. It replaces the legacy `padding` calculation; legacy style fields must still be valid. Rounding happens on the complete group band before subgroup division, while `inner_padding` remains a pixel gap between subgroups. The returned `layout.category_scale` exposes the resolved full-group scale. Visible subgroups, including explicitly reserved slots with no observations, still require strictly positive breadth: a valid D3 profile with zero bandwidth or an oversized subgroup gap is rejected. `None` preserves the previous layout and returns no configured scale. `category_reverse` requires an explicit profile and reverses the full range while retaining subgroup order. Reversal and the additional shared padding/alignment controls are D3/Rust extensions; Nivo's BoxPlot `indexScale` exposes rounding.

```rust
use kartis::*;
use std::{cell::RefCell, collections::BTreeMap};

fn main() -> kartis::Result<()> {
    let config = ChartConfig {
        width: 420., height: 360.,
        margin: Margin { left: 24., right: 24., top: 24., bottom: 24. },
        ..Default::default()
    };
    let categories = ["low", "high"].map(str::to_string);
    let rows = [BarSeries::new("ring", vec![Some(2.), Some(4.)])];
    let mut radial_options = StackedRadialBarOptions {
        tracks: false, legend: false, opacity: 0.4,
        axes: PolarAxesOptions {
            radial_start: None, circular_outer: None,
            grid: PolarGridStyle { radial: false, circular: false, ..Default::default() },
            ..Default::default()
        },
        colors: BTreeMap::from([
            ("low".into(), Color(20, 40, 60, 255)),
            ("high".into(), Color(200, 100, 50, 255)),
        ]),
        labels: Some(RadialBarLabels {
            text: RadialBarLabelText::Category,
            color: Some(Color(40, 50, 60, 101)),
            ..Default::default()
        }),
        ..Default::default()
    };
    let radial_default = layout_stacked_radial_bar(&config, &categories, &rows, &radial_options)?;
    radial_options.labels.as_mut().unwrap().color_profile =
        Some(InheritedColor::FromContext(vec![ColorModifier::Darker(1.)]));
    let radial = layout_stacked_radial_bar(&config, &categories, &rows, &radial_options)?;
    assert_eq!(radial.scene.marks, radial_default.scene.marks);
    assert_eq!(radial.scene.labels.iter().map(|label| label.color).collect::<Vec<_>>(),
        [Color(14, 28, 42, 255), Color(140, 70, 35, 255)]);
    assert_eq!(radial.interaction_binding()?, radial_default.interaction_binding()?);

    let box_config = ChartConfig {
        width: 221., height: 240.,
        margin: Margin { left: 60., right: 60., top: 24., bottom: 24. },
        ..Default::default()
    };
    let observations = ["A", "B", "C"].into_iter().flat_map(|group| {
        [1., 2., 3.].into_iter().enumerate().map(move |(i, value)|
            BoxObservation::new(format!("{group}/{i}"), group, "", Some(value)))
    }).collect::<Vec<_>>();
    let mut box_options = BoxPlotLayoutOptions { inner_padding: 0., legend: false, ..Default::default() };
    let box_default = boxplot_layout(&box_config, &observations, &box_options)?;
    assert!(box_default.category_scale.is_none());
    box_options.category_scale = Some(scales::BandScaleOptions {
        padding_inner: 0.1, padding_outer: 0.1, align: 0.5, round: true,
    });
    let boxes = boxplot_layout(&box_config, &observations, &box_options)?;
    assert_eq!(boxes.boxes.iter().map(|b| b.center).collect::<Vec<_>>(), [78.5, 110.5, 142.5]);
    assert_eq!(boxes.category_scale.as_ref().unwrap().bandwidth, 29.);
    assert_eq!(boxes.domain, box_default.domain);

    let hierarchy = [
        HierarchyDatum::new("root", None, 0.),
        HierarchyDatum::new("sales", Some("root".into()), 0.),
        HierarchyDatum::new("leaf", Some("sales".into()), 4.),
        HierarchyDatum::new("outside", Some("root".into()), 4.),
    ];
    let sun_options = SunburstLayoutOptions {
        value_format: Some(ValueFormat::Fixed(0)),
        labels: Some(SunburstLabelOptions::default()),
        ..Default::default()
    };
    let sun_default = sunburst_layout(&config, &hierarchy, &sun_options)?;
    let events = RefCell::new(Vec::new());
    let sun = {
        let mut text = |context: &SunburstLabelContext<'_>| {
            events.borrow_mut().push(context.datum.id.clone());
            Ok(if context.datum.id == "outside" { String::new() } else {
                format!("{}@{}:{}", context.datum.id, context.node.depth, context.node.formatted_value)
            })
        };
        sunburst_layout_with_label_factory(&config, &hierarchy, &sun_options, &mut text)?
    };
    assert_eq!(*events.borrow(), ["sales", "outside", "leaf"]);
    assert_eq!(sun.scene.marks, sun_default.scene.marks);
    assert_eq!(sun.interaction_binding()?, sun_default.interaction_binding()?);
    assert_eq!(sun.scene.labels.iter().map(|label| label.text.as_str()).collect::<Vec<_>>(),
        ["sales@1:4", "", "leaf@2:4"]);
    assert_eq!(sun.label_candidates().len(), 3);
    assert_eq!(events.borrow().len(), 3);
    let _svg = render::to_svg(&radial.scene)?;
    let _script = render::to_canvas_script(&sun.scene, "sunburst")?;
    Ok(())
}
```

The band example reserves a 101-pixel plot. The explicit rounded scale produces group centres 78.5, 110.5 and 142.5, with a 29-pixel band. The unchanged legacy profile retains fractional group centres. The two Sunburst labels with aggregate value 4 receive different text from their IDs and depths; the third returns an empty owned label. See [rendering and typography](../README.md) for font-aware bounds and [feature gaps](feature-gaps.md) for remaining work.
