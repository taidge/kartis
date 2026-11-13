# BoxPlot colors and controlled hover

[简体中文](boxplot-color-and-hover.zh-CN.md)

`BoxPlotLayoutOptions::color_by` selects `BoxPlotColorBy::SubGroup` (the existing default) or `Group`. Palette positions follow the complete configured domain, including reserved and hidden slots. Group coloring also supplies inherited stroke colors and the default outlier fill. It does not change subgroup series IDs or the meaning of `hidden`.

Group legends contain groups with actual, nonempty summaries after hidden-series filtering, in full-domain order. Hover opacity zero does not remove their identities. Their IDs are the original group strings and their role is `LegendRole::Category`; activating them does not implicitly hide a series. The existing SubGroup legend keeps its complete configured domain. Custom Nivo property accessors and `legendLabel` callbacks are outside this API.

```rust
use kartis::*;

fn main() -> Result<()> {
    let config = ChartConfig::default();
    let data: Vec<_> = ["North", "South"].into_iter().flat_map(|group| {
        ["Actual", "Plan"].into_iter().flat_map(move |subgroup| {
            [2., 4., 6., 8.].into_iter().enumerate().map(move |(i, value)| {
                BoxObservation::new(format!("{group}/{subgroup}/{i}"), group, subgroup, Some(value))
            })
        })
    }).collect();
    let options = BoxPlotLayoutOptions {
        color_by: BoxPlotColorBy::Group,
        ..Default::default()
    };
    let mut controller = BoxPlotInteraction::new(
        &config, &data, &options, &BoxPlotInteractionOptions::default(),
    )?;
    let baseline = controller.layout.scene.clone();
    controller.set_active(Some(BoxPlotKey::new("North", "Actual")))?;
    assert_eq!(controller.active_keys().len(), 3);
    let _svg = render::to_svg(&controller.layout.scene)?;
    let _binding = controller.scene_interaction()?;
    controller.set_active(None)?;
    assert_eq!(controller.layout.scene, baseline);
    Ok(())
}
```

An active summary highlights every summary with the same group **or** subgroup, independently of color selection. Default factors are `opacity = 1`, `active_opacity = 1`, and `inactive_opacity = 0.25`. Each factor applies once to the whole box, including its border, median, whiskers, caps, and emitted outliers. Existing fill opacity and role RGBA remain independent. SVG uses a parent group; Canvas and CPU raster use a bounded transparent layer; the semantic Glory chart uses a CSS parent group.

Use `set_hovered_mark` with a currently emitted source mark ID, or `key_for_mark` to resolve a body, border, median, whisker, cap, or outlier to its original identity. Setters reject unknown or stale identities atomically; `key_for_mark` and `outlier_for_mark` return `Ok(None)` for unknown IDs. Constant summaries retain logical center anchors without fabricated pointer ink. A zero-opacity group is excluded from geometric picking. `replace_source` recomputes from observations and preserves the active key only when it remains present. Editing the controller's public layout causes subsequent controlled operations to reject; restore the accepted frame or rebuild from source.

Pass `scene_interaction()` to BrowserChart or RasterSurface's layout API. A semantic Chart instead receives `controller.layout.scene`, typically through a `Cage<Scene>`. Applications own the hover event and call the controller to publish the next frame. Set `Chart::highlight(false)`, `BrowserHighlight::Controlled`, or `RasterSurfaceOptions { highlight: false, ..Default::default() }` so the host does not add its independent single-mark dimming. For immediate browser updates select `BrowserOptions::motion = MotionPreference::Reduced`. This API does not automatically register device hover handlers.

The binding supplies one logical body site per summary. Standard Browser/Raster geometry hover resolves those body sites. Applications route hits on whiskers, caps, borders and outliers through the controller's `hit_test`/`key_for_mark` and the controlled setters. Changing group structure can remount semantic Chart mark nodes; source mark IDs remain stable across these changes. Retained SVG moves existing path nodes between the group parents.

`MarkOpacityGroup` is flat, disjoint, and contains consecutive painted marks; labels are separate. Canvas and CPU raster currently reject a fractional group whose member itself needs an isolated artistic blend or fill-and-stroke opacity layer. BoxPlot's separately emitted parts use the supported path. Spring/property topology transitions reject grouped frames; use immediate replacement or `Scene::interpolate` with compatible group membership. These limits do not change ordinary ungrouped frames.

The [original-function reference](../tests/fixtures/box-colorby-hover/README.md) preserves Nivo's actual palette prewarming, legend computation, and active-key results. Nullable observations retain Rust's omission policy, whereas the recorded upstream numeric coercion can create zero-valued samples. Numeric upstream legend IDs, arbitrary accessors, and stale active objects are not claimed as equivalent Rust inputs.
