# Dormant Funnel motion (Kartis 0.2.0)

[简体中文](funnel-dormant-motion.zh-CN.md) · [Motion guide](funnel-motion.md)

A source part owns its body and border springs even while one role has no paint.
Zero border width, zero border opacity and completely collapsed bodies do not
discard that state. Dormant paths use constructor-captured geometry, receive new
targets and advance on the same clock as visible paths. They participate in
`is_settled`, `finish`, reduced motion and the geometry/role budgets. Removing the
source key discards its state; readding the key starts at its new target.

## Inspecting a role without a Mark

```rust
use kartis::*;

pub fn dormant_border() -> Result<f64> {
    let config = ChartConfig::default();
    let source = [FunnelDatum::new("a", 8.)];
    let mut options = FunnelLayoutOptions { border_width: 0., ..Default::default() };
    let first = FunnelSvgFrame::new(&config, &source, &options)?;
    let mut motion = FunnelMotion::new(first, FunnelMotionOptions::default())?;
    options.border_width = 6.;
    motion.retarget(FunnelSvgFrame::new(&config, &source, &options)?)?;
    motion.tick(0.05)?;
    Ok(motion.frame().part_property(0, FunnelMotionPartRole::Border,
        FunnelMotionMarkChannel::StrokeWidth)?.unwrap().value)
}
```

The default spring samples approximately `0.8365095561160942` here. The
`part_property`, `part_path_progress` and `part_geometry_states` queries use the
current source index and work without an actual Mark. Existing `mark_property`
and `path_progress` queries describe actual Mark slots. None of these queries
creates paint, a logical site, a tooltip selection or an input target.

## Capturing custom geometry for every source

The original stage-factory constructors call the application only for originally
drawable bodies. A callback that was never called supplies no custom collapsed
target. Enabling motion on that unresolved custom body returns an error before
adoption. Use the explicit all-source pass when such parts need motion:

```rust
use kartis::*;

pub fn custom_target(source: &[FunnelDatum]) -> Result<FunnelSvgFrame> {
    let config = ChartConfig::default();
    let frame = FunnelSvgFrame::new(&config, source, &FunnelLayoutOptions::default())?;
    frame.with_all_stage_factory(&mut |context| {
        Ok(context.inherit(Geometry::Circle {
            x: context.part.center[0], y: context.part.center[1], radius: 12.,
        }))
    })
}
```

This pass calls the factory once for every source part, including those without
a body Mark. Inactive results remain motion descriptors and add no paint or
interaction. Size/style factories are not replayed. Apply it before binding
annotations; it operates on the annotation source cohort. For owned application
data, use `data.with_computed(data.computed().with_all_stage_factory(&mut factory)?)`
to retain the same raw source pool. The callback is never retained by motion.

`part_geometry_origin` identifies built-in, executed stage-factory and uncalled
legacy stage-factory captures. Actual edited Mark geometry takes precedence over
the constructor descriptor. An absent custom role uses its captured geometry,
never an inferred default silhouette. Invalid geometry/paint and excessive
latent work reject the candidate. Application callback side effects remain external.

Pure sampled Scenes work with SVG, Canvas and raster rendering. The dedicated
browser motion owner is SVG; GPU/worker remains deferred. Runtime acceptance is
recorded separately in the [verification record](verification.md).
