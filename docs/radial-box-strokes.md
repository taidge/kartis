# Inherited RadialBar and BoxPlot stroke colors

These four optional roles reuse `InheritedColor` without changing the fixed defaults. Each defaults to `None`; `Some(InheritedColor::Theme)` resolves its own existing stroke color. A profile does not enable a missing border, and the configured width and cap retain their meaning.

| Configuration | Inherited color and contrast surface | Theme fallback |
| --- | --- | --- |
| `StackedRadialBarOptions.border_color_profile` | Original numeric category palette, before `paints` | Segment `border.color` |
| `BoxPlotLayoutOptions.border_color_profile` | Original numeric subgroup palette | Box `border.color` |
| `BoxPlotLayoutOptions.median_color_profile` | Original numeric subgroup palette | `median.color` |
| `BoxPlotLayoutOptions.whisker_color_profile` | Original numeric subgroup palette | `whisker.color` |

The complete program checks the resolved RGBA and stored mark opacity, then exports both Scenes through the ordinary SVG and Canvas APIs. `ColorModifier::Opacity` replaces the role color's alpha; it does not replace or premultiply mark opacity.

```rust
use kartis::*;
use glory::render::{GraphicStroke, StrokeCap};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let config = ChartConfig::default();
    let inherited = |darker, opacity| {
        InheritedColor::FromContext(vec![
            ColorModifier::Darker(darker),
            ColorModifier::Opacity(opacity),
        ])
    };
    let border = GraphicStroke {
        color: Color(70, 80, 90, 101),
        width: 3.,
        cap: StrokeCap::Square,
    };
    let categories = vec!["sales".into(), "cost".into()];
    let rings = vec![BarSeries::new("quarter", vec![Some(2.), Some(8.)])];
    let radial = layout_stacked_radial_bar_with_arc_style(
        &config,
        &categories,
        &rings,
        &StackedRadialBarOptions {
            colors: [("sales".into(), Color(20, 40, 60, 160))].into(),
            border: Some(border),
            border_color_profile: Some(inherited(1., 0.5)),
            opacity: 0.4,
            ..Default::default()
        },
        &StackedRadialArcStyle {
            corner_radius: 6.,
            ..Default::default()
        },
    )?;
    let sales = radial
        .scene
        .marks
        .iter()
        .find(|mark| mark.series == "sales")
        .ok_or("missing sales segment")?;
    assert_eq!(sales.stroke.map(|s| s.color), Some(Color(14, 28, 42, 128)));
    assert_eq!(sales.opacity, 0.4);
    radial.interaction_binding()?;

    let samples = [1., 2., 3., 4.]
        .into_iter()
        .enumerate()
        .map(|(i, value)| BoxObservation::new(i.to_string(), "Q1", "sales", Some(value)))
        .collect::<Vec<_>>();
    let boxes = boxplot_layout(
        &config,
        &samples,
        &BoxPlotLayoutOptions {
            colors: [("sales".into(), Color(20, 40, 60, 160))].into(),
            border: Some(border),
            border_color_profile: Some(inherited(1., 0.5)),
            median_color_profile: Some(inherited(2., 0.75)),
            whisker_color_profile: Some(InheritedColor::FromContext(vec![ColorModifier::Opacity(
                0.25,
            )])),
            fill_opacity: 0.2,
            ..Default::default()
        },
    )?;
    for (part, expected) in [
        (":border", Color(14, 28, 42, 128)),
        (":median", Color(10, 20, 29, 191)),
        (":whiskers", Color(20, 40, 60, 64)),
        (":caps", Color(20, 40, 60, 64)),
    ] {
        let mark = boxes
            .scene
            .marks
            .iter()
            .find(|mark| mark.id.ends_with(part))
            .ok_or("missing boxplot stroke")?;
        assert_eq!(mark.stroke.map(|s| s.color), Some(expected));
        assert_eq!(mark.opacity, 1.);
    }
    for scene in [&radial.scene, &boxes.scene] {
        scene.validate()?;
        assert!(render::to_svg(scene)?.contains("<svg"));
        assert!(!render::to_canvas_script(scene, "chart")?.is_empty());
    }
    Ok(())
}
```

RadialBar resolves its segment border from the original category palette before choosing any custom fill paint. Tracks and their colors remain independent. The role is shared by the ordinary and arc-style Scene/layout wrappers. An actual `Some` source supplies a context even when hidden, zero-valued or collapsed by padding; empty and all-`None` sources do not fabricate one. Intrinsic modifier validation still applies without visible ink. For an arc-style segment collapsed by padding, the resolved border alpha and segment opacity determine whether outline-only ink remains; the earlier fixed color must not decide that after an override. A configured RadialBar border must have positive width: `Some` with `0.0` or `-0.0` is rejected; use `border: None` to disable it. Existing zero/collapse geometry and interaction policies otherwise remain in force.

BoxPlot uses its existing subgroup palette and the complete subgroup order, including hidden identities, so hiding one subgroup does not reassign another's palette slot. The same roles apply through `boxplot_layout` and `boxplot_layout_with_scale_options`. Only an actual visible, nonempty summary box has a context. Reserved empty slots and nullable-only observations do not become zero-valued boxes; a genuine constant-value box still has a context. Profiles also validate for that real box when the border is absent or the median/whisker width is zero. With an absent border, no stroke is emitted; the unused Theme fallback is the numeric palette.

The three BoxPlot stroke roles stay independent of `fill_opacity`. The example stores border, median and whisker/cap alpha as 128, 191 and 64 while the box fill mark has opacity 0.2. The whisker role supplies both stems and both end caps. For BoxPlot, `border: Some(...)` and `outlier_border: Some(...)` with width `0.0` or `-0.0` suppress that stroke, just as `None` does. Filled outliers retain their fill, radius and geometry when their border is disabled. Median and whisker width zero retain their existing skip behavior, and `cap_size: 0` still suppresses end caps. These roles do not recolor outliers, labels, axes, grid or legend, and leave source IDs, statistics, domains and scale projection unchanged. This adds neither group-based `colorBy` nor a custom BoxPlot `FillPaint` API or private controlled-factory update protocol. Data, palette or profile changes require rebuilding the layout.

`Fixed`, ordered modifiers and `Contrast` use the shared resolver. Contrast is a Rust extension evaluated against the original numeric RGBA surface and chart background; a translucent surface requires an opaque backdrop. It does not promise visual contrast against every pixel of a gradient, pattern or fill-opacity composite. Typed alpha-zero channel retention and its difference from CSS alpha restoration are described in [Border color roles](border-color-roles.md).

The resulting ordinary Scene can be consumed by SVG, Canvas, CPU raster and Glory hosts within their existing capabilities. The semantic Graphic's gradient/pattern limits remain; these roles do not add semantic gradient support. Separate category bands, label options and summary-domain configuration are covered by [Labels and bands](chart-labels-and-bands.md) and [Summary domains](summary-index-legend.md).
