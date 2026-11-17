# Custom Funnel stage bodies

[简体中文](funnel-custom.zh-CN.md)

This API is **Unreleased** and is not included in the published `kartis 0.1.0`.
It extends the existing Nivo-inspired stage layout with a borrowed Rust geometry
factory. It does not implement Nivo's arbitrary React custom layers or live Widgets.

`funnel_layout_with_stage_factory` accepts the same config, source and options as
`funnel_layout`, followed by `&mut` a callback. Each context contains the original
`datum`, `source_index`, computed `part`, original body `geometry`, inherited
`fill`, `fill_paint`, `opacity`, and the plot rectangle. Sources are borrowed;
returned frames retain no callbacks. Numeric and string category identities stay
distinct. Callbacks run in source order only for stages with an original body;
zero-width bodyless stages keep their labels and metadata without fake hit sites.

```rust
# use kartis::*;
let data = [FunnelDatum::new("Visitors", 80.), FunnelDatum::new("Orders", 20.)];
let layout = funnel_layout_with_stage_factory(
    &ChartConfig::default(), &data, &FunnelLayoutOptions::default(),
    &mut |ctx| {
        let [x, y] = ctx.part.center;
        let mut stage = ctx.inherit(Geometry::Circle { x, y, radius: 12. });
        stage.retain_profile_border = false;
        Ok(stage)
    },
)?;
# Ok::<(), ChartError>(())
```

The returned `FunnelStageGeometry.body` supports solid/gradient/pattern fill,
opacity, stroke and dash through `ScatterNodeGeometry`. Circles and rectangles
must stay centered at `part.center`; an offset Path preserves its original data
anchor and may specify animation support command indices. Exact geometry picking
uses the new silhouette; nearest picking uses the original data center. Custom
ink does not change stage bands, source widths, profile points, labels' positions,
separator placement, values, formatting or series ownership. `part.bounds` remains
the authored start-width rectangle, **not** the replacement's ink bounds.

`ctx.inherit(ctx.geometry.clone())` preserves the complete original frame, including
gradient source colors and legacy anchors. Changed bodies resolve their visible
representative color for part metadata, interaction, legends and inherited label
color. `retain_profile_border = true` keeps the original open-sided profile
border, with its own opacity and paint; it is not a stroke around the new body.
Set it to `false` for a different silhouette and use `body.stroke` if needed.
Custom stroke and retained border are separate layers and can overlap.

All factories finish before a frame is returned. Invalid geometry, nonfinite
opacity, inconsistent anchors, invalid paint/dash or cumulative Scene budgets
reject the candidate. Path commands are capped at 200,000 across the frame;
shared paint, dash and clip budgets also apply. Callback-owned side effects are
not undone if a later callback or validation rejects the candidate.

The result uses the shared Scene export and Glory interaction paths. Rust
regression tests cover inherited output, offset picking and rejection. Actual
browser/device validation is recorded separately; these API checks do not claim
physical input or arbitrary Widget support.
