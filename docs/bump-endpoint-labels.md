# Independent Bump endpoint text

[简体中文](bump-endpoint-labels.zh-CN.md)

This is an **Unreleased development API**, absent from published Kartis 0.1.0.
Start and End can display different text without changing the series label,
legend, point tooltip data, geometry, paint or interaction identity. Existing
entries and `BumpLayoutOptions` fields retain their behavior.

```rust
use kartis::*;

let data = [BumpSeries::new("Orders", vec![
    BumpDatum::new("Q1", 3.), BumpDatum::new("Q2", 1.),
])];
let options = BumpLayoutOptions {
    labels: BumpEndpointLabels { start: true, end: true, ..Default::default() },
    ..Default::default()
};
let layout = bump_layout_with_label_factory(
    &ChartConfig::default(), &data, &options,
    |ctx| Ok(match ctx.endpoint {
        BumpEndpoint::Start => format!("Opening {}", ctx.series.id),
        BumpEndpoint::End => format!("Closing {}", ctx.series.id),
    }),
)?;
# Ok::<(), ChartError>(())
```

`BumpEndpointLabelContext` borrows the original `series`, the completed
`computed` row, and `default_text` (the original series label). `series_index`
is the source-slice index, including hidden series. `line_position` is the
existing first/last shoulder-extended line point; `position` is the final label
anchor after that endpoint's signed padding. Start/End follow existing source
traversal, including with reversed X; they do not sort arbitrary source points
into geometric left/right endpoints.

Only actual owned endpoint labels invoke the callback. Hidden series, disabled
ends and missing first/last endpoints do not. Empty strings retain their owned
Label slots. Zero opacity, a transparent label color, or zero point size does
not by itself remove an existing endpoint slot or suppress its callback.
Returned text is limited to `MAX_LABEL_BYTES` (1,024 UTF-8 bytes), independent
of the source series-label limit. Source and callbacks are not retained.

## Compose text and point geometry

`bump_layout_with_factories` accepts `BumpFactories` with independently optional
`point` and `endpoint_label` borrowed callbacks. Public `BumpPointFactory` and
`BumpEndpointLabelFactory` aliases describe these callback types.

```rust
# use kartis::*;
# fn example(data: &[BumpSeries], options: &BumpLayoutOptions) -> Result<BumpLayout> {
let mut point = |ctx: &BumpPointContext<'_>| {
    Ok(ctx.inherit(Geometry::Circle {
        x: ctx.position[0], y: ctx.position[1], radius: 4.,
    }))
};
let mut text = |ctx: &BumpEndpointLabelContext<'_>| {
    Ok(format!("{:?}: {}", ctx.endpoint, ctx.default_text))
};
bump_layout_with_factories(
    &ChartConfig::default(), data, options,
    BumpFactories { point: Some(&mut point), endpoint_label: Some(&mut text) },
)
# }
```

For each source series, the existing point callbacks run in source order, then
the line is constructed, then Start and End label callbacks run in that order.
A failure stops construction without returning a partial layout. Effects made
by application callbacks cannot be undone; publish application state only after
the build succeeds. Existing point-only calls retain their behavior, and
returning `ctx.default_text.to_owned()` preserves the old endpoint text.

## Reference and rendering scope

Nivo commit `0dae2c32052a573f9f1d66ec1b453b572119b265` declares independent
`startLabel`/`endLabel` series accessors in `packages/bump/src/bump/types.ts`.
`useBumpSeriesLabels` in `hooks.ts` computes each position in a separate hook,
and invokes its accessor before excluding a missing endpoint. Kartis invokes
only actual labels, with the explicit series-first composition order above.
Callback side effects and invocation order therefore are not React hook
equivalence. Generic Nivo extra fields are not automatically reflected into
Rust structs; a closure can capture application data keyed by source identity.

The returned ordinary Labels use the existing SVG, Canvas, CPU and Glory Text
paths. Font selection, whitespace policy and host measurement remain explicit.
Supplied-font outlined export provides a fixed shaping profile; it does not
establish editable native-text pixel equality, automatic label collision
avoidance, arbitrary components or physical-device acceptance. The executable
`bump_endpoint_labels` demonstrates composition and export. Concrete acceptance
is recorded in [verification](verification.md#development-bump-endpoint-text).
