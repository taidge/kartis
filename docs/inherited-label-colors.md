# Inherited Bar and HeatMap label colors

Set `BarLabels.color_profile` or `HeatmapLayoutOptions.label_color_profile` to reuse the shared `InheritedColor` role. `None` preserves the existing fixed label style or theme. `Some(Theme)` also uses that existing style/theme fallback; `Fixed`, `FromContext` and `Contrast` explicitly override the static color.

```rust
use kartis::*;

fn main() -> kartis::Result<()> {
    let config = ChartConfig::default();
    let profile = InheritedColor::FromContext(vec![ColorModifier::Darker(1.)]);
    let categories = ["A", "B"].map(str::to_string);
    let data = [BarSeries::new("sales", vec![Some(4.), Some(8.)])];
    let mut bar = bar_layout_with_bar_factory(
        &config,
        &categories,
        &data,
        &BarOptions::default(),
        &BarStyleOptions {
            colors: [("sales".into(), Color(20, 40, 60, 160))].into(),
            labels: BarLabels {
                enabled: true,
                color_profile: Some(profile.clone()),
                ..Default::default()
            },
            ..Default::default()
        },
        |context| {
            let mut mark = context.inherit(context.geometry.clone());
            mark.fill = Some(Color(240, 30, 180, 255));
            mark.opacity = 0.5;
            Ok(mark)
        },
    )?;
    let label = bar.layout.bars[0].label_index.unwrap();
    assert_eq!(bar.layout.scene.labels[label].color, Color(14, 28, 42, 80));
    let base = bar.layout.scene.clone();
    let active_id = bar.layout.bars[0].id.clone();
    bar.set_active(Some(&active_id), 0.2)?;
    bar.set_active(None, 0.2)?;
    assert_eq!(bar.layout.scene, base);

    let heat = heatmap_layout_with_cell_factory(
        &config,
        &[
            HeatCell { row: "row".into(), column: "empty".into(), value: None },
            HeatCell::new("row", "zero", 0.),
            HeatCell::new("row", "value", 10.),
        ],
        &HeatmapLayoutOptions {
            axes: HeatmapAxes::none(),
            hover_target: HeatmapHoverTarget::Cell,
            label_color_profile: Some(profile),
            colors: ColorScaleOptions {
                domain: Some([0., 10.]),
                colors: vec![Color(20, 40, 60, 160), Color(200, 180, 160, 220)],
                unknown: Color(90, 100, 110, 64),
                ..Default::default()
            },
            opacity: 0.8,
            active_opacity: 0.9,
            inactive_opacity: 0.2,
            ..Default::default()
        },
        |context| {
            let mut mark = context.inherit(context.geometry.clone());
            mark.fill = Some(Color(240, 30, 180, 255));
            mark.opacity = 0.5;
            Ok(mark)
        },
    )?;
    let zero = |frame: &HeatmapLayout| {
        frame.scene.labels.iter().find(|label| label.text == "0").unwrap().color
    };
    assert_eq!(zero(&heat), Color(14, 28, 42, 64));
    let active = heat.with_active(Some(HeatmapCellKey::new("row", "empty")))?;
    assert_eq!(zero(&active), Color(14, 28, 42, 16));
    assert_eq!(active.with_active(None)?.scene, heat.scene);
    let _binding = heat.interaction_binding()?;
    let _bar_svg = render::to_svg(&bar.layout.scene)?;
    let _heatmap_canvas = render::to_canvas_script(&heat.scene, "heatmap")?;
    Ok(())
}
```

The example uses palette RGBA (20,40,60,160). Darker(1) resolves to (14,28,42,160). The Bar factory factor 0.5 produces label alpha 80. HeatMap base opacity 0.8 times the same factory factor produces alpha 64; activating the missing cell gives the zero cell inactive alpha 16. Clearing the active key restores the original frame.

Both roles resolve before custom geometry and paint factories. The inherited color and contrast surface are the numeric layout palette; HeatMap nulls use its unknown color. A custom magenta fill, gradient or pattern does not replace that palette with the interaction paint representative. Contrast compares candidates over this palette surface and chart background; it is not a guarantee for every pixel of an arbitrary custom paint.

`ColorModifier::Opacity` replaces color alpha; the remaining layout, factory and controlled-hover factors apply afterward. Each accepted hover frame starts from its original resolved RGBA and rounds final label alpha once. Ordinary Bar labels keep their existing eligibility, position and value format; HeatMap keeps nullable/empty text and owned-label source checks. This option does not add a per-datum text accessor or zero-extent Bar labels. Rebuild the layout to change data or profiles.

Profiles are validated before factory callbacks, including hidden sources or disabled labels. Context-dependent validation uses actual source palette colors; a genuinely empty source does not invent one. The existing contrast rule requires an opaque effective background. Invalid profiles return an error before a controlled frame is accepted.

Typed RGBA preserves hidden RGB channels at alpha zero. Applying Opacity(1) can reveal them. Original Nivo modifiers that first parse a CSS alpha-zero string through d3-color lose those channels and may produce black instead. The reference data retains both results; this API preserves the existing Rust typed-color contract. Ordinary opaque and semitransparent modifier profiles are checked against original D3/Nivo results.

Standalone SVG uses `fill-opacity` for a partially transparent final mark with one solid fill, no stroke or paint server, and normal blending. It keeps the original numeric opacity and avoids WebKit's object-opacity rounding difference. Other marks retain object opacity. This export correction leaves CanvasScript and retained browser SVG unchanged; SVG postprocessors should read the applicable attribute and must not apply both factors.

Use `..Default::default()` when constructing options, as these are additional public fields. [Bar geometry and paint](bar-custom-geometry.md) · [HeatMap cells](heatmap-custom.md)
