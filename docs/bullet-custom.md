# Custom Bullet parts

[简体中文](bullet-custom.zh-CN.md)

This API is available in Kartis 0.2.0. It adds a borrowed single-mark geometry and paint factory for
Bullet ranges, measures and markers.

`bullet_layout_with_part_factory(config, data, options, &mut factory)` returns
`CustomBulletLayout` with `layout: BulletLayout`, `interaction: InteractionBinding`
and `plot: ClipRect`. The existing `BulletLayout` keeps its fields. Returning
`ctx.inherit(ctx.geometry.clone())` for every part preserves the complete ordinary
inner layout; the wrapper additionally supplies its typed binding.

```rust
# use kartis::*;
let data = [BulletDatum {
    id: "Orders".into(),
    ranges: vec![40., 75., 100.],
    measures: vec![62.],
    markers: vec![80.],
}];
let custom = bullet_layout_with_part_factory(
    &ChartConfig::default(),
    &data,
    &BulletLayoutOptions::default(),
    &mut |ctx| {
        let geometry = if ctx.part.kind == BulletPartKind::Measure {
            Geometry::Circle {
                x: ctx.position[0],
                y: ctx.position[1],
                radius: 10.,
            }
        } else {
            ctx.geometry.clone()
        };
        Ok(ctx.inherit(geometry))
    },
)?;
let svg = render::to_svg(&custom.layout.scene)?;
# let _ = svg;
# Ok::<(), ChartError>(())
```

`bullet_layout_with_scale_options_and_part_factory(config, data, options, scale,
&mut factory)` combines the factory with the existing continuous-scale options.
It returns `CustomScaledBulletLayout`: `layout` is the custom wrapper, while
`shared_value_scale` and `row_value_scales` retain the resolved query scales.
Custom ink does not change domains, scale projection or configured reversal.

## Callback data and ordering

The callback borrows `BulletPartContext` with:

- `datum`: the original `BulletDatum`; `source_row_index` is its input index;
- `row`: the ordinary computed row; `row_index` follows configured visible row order;
- `part`: its kind, stable ID, original `source_index`, raw value and intervals;
- `rank`: the part's ordinal within its kind, including parts without a drawn mark;
- original `geometry`, `position` at the original visible part's center, chart
  `plot`, and inherited
  `fill`, `fill_paint`, `stroke`, `stroke_dash` and `opacity`.

Ranges and measures follow their sorted endpoint order, then markers follow
their original order, within each visible row. `source_index` identifies the
original element in that kind's input vector; it is not the sorted `rank`.
A synthetic final range ending at the domain maximum has `source_index: None`
and invokes the callback when it has an actual mark.

Only actual part marks invoke the factory. Hidden rows, zero-length or fully
cropped intervals, and markers or parts suppressed by size settings do not gain
marks or callbacks. Their existing metadata is retained. Each context's `row`
is the original row snapshot; it does not expose replacements returned by earlier
callbacks. The accepted result updates each replaced `part.geometry` together
after callbacks finish.

## Geometry, paint and interaction

Return `ScatterNodeGeometry`. `ctx.inherit(geometry)` keeps the original paint,
including marker strokes and part opacity. Shared solid, gradient and pattern
fills, stroke, dash and opacity are available. A rectangle or circle must remain
centered at `position`; use a Path for offset ink. Paths can specify checked
`anchor_commands` for following compatible animation frames.

Part IDs, row ownership, raw values, original and visible intervals, domains,
axes and titles keep their original meaning. Replacement geometry retains the
original row-track clip, even if the authored shape extends beyond that track.
`part.geometry` describes the accepted replacement; its raw interval remains
source metadata rather than a reconstruction from that shape.

Use the wrapper's `interaction` with `layout.scene` through the existing typed
Surface APIs. Exact geometry picking follows the replacement mark and its clip;
it retains the existing geometry picker's stroke tolerance. Nearest picking
uses the original visible part center, with Path anchors following the delivered
animation geometry. A displaced Path can therefore separate visible ink from
its logical origin. Interaction-site colors use the effective custom paint's
representative color. Row legend colors retain the original row-palette policy;
they are visibility keys, not a recomputed legend of custom part paints.

## Validation and Nivo scope

The source and callback are borrowed during construction and are not retained.
The function returns a validated candidate or an error, without publishing a
partial layout. Invalid geometry, paint, opacity, dash or anchor data and shared
cumulative Scene budgets reject the candidate. External state changed inside a
callback cannot be rolled back if a later callback or validation fails; publish
application state after construction succeeds.

Nivo's `rangeComponent` and `measureComponent` receive rectangle data, dimensions,
border props, animated props and mouse handlers through `BulletRects.tsx`.
`markerComponent` receives position, size, rotation, animated props and handlers
through `BulletMarkers.tsx`; their types are declared in `types.ts`. Kartis
provides the static geometry and paint subset through one kind-aware callback,
using the shared Scene SVG, Canvas, CPU and Glory paths. It does not reproduce
arbitrary React components, animated-prop objects or component lifecycles.

There is also a data-contract difference: Nivo `compute.ts::stackValues` removes
zero endpoints before sorting ranges and measures, and its `index` is the
resulting ordinal. Kartis retains zero and negative values and their original
source indices. An endpoint at zero can form a nonempty interval when the domain
starts below zero; a zero-length interval still has no painted part. Both the
sorted rank and source identity therefore matter when porting custom rendering.

Use `custom.into_scene_interaction()` for the legacy Surface/browser geometry and nearest-point profile. For Glory `chart_layout`, use `custom.into_painted_scene_interaction()?`: it explicitly preflights the required solid PaintedGeometry profile. This rejects gradient/pattern eligible fills and admits Geometry picking only, without Nearest or slices. Ordinary semantic Graphic limits (including 256 commands per mark and unsupported rotated ordinary Text) still apply. Both wrappers provide these conversions.
