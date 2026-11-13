# Planar contours and weighted density

[简体中文](contour-density.zh-CN.md) · [Library overview](../README.md)

Kartis extracts planar threshold regions as filled polygons with islands and holes, and
estimates weighted two-dimensional density in logical pixels. Both produce the
shared `Scene` used by SVG, Canvas and optional CPU raster output. The APIs are
exported from `kartis::contour` and the crate root. Geographic projection,
polygon repair and automatic topology-changing animation are separate concerns.

## A row-major grid

`ContourGrid` borrows `width * height` samples in row-major order:
`values[y * width + x]`. Width and height must be positive. Present samples
must be finite; `None` consistently lies below every threshold. Classification
uses `value >= threshold`. Output coordinates cover the planar grid extent
`[0, width] × [0, height]`, including the original half-cell boundary convention.

`contour` extracts one finite threshold. `contours` accepts
`ContourThresholds::Values`, `Count` or the default `Sturges` rule through
`ContourOptions`. Explicit thresholds are sorted, with duplicates preserved.
Automatic counts request a tick density rather than an exact number of levels;
constant or all-missing grids have no automatic levels. Smoothing is enabled by
default and can be disabled.

```rust
use kartis::*;

fn main() -> Result<()> {
    let samples = [
        Some(0.), Some(1.), Some(0.),
        Some(1.), Some(4.), Some(1.),
        Some(0.), Some(1.), None,
    ];
    let layout = contour_layout(
        &ChartConfig::default(),
        ContourGrid { width: 3, height: 3, values: &samples },
        &ContourOptions {
            thresholds: ContourThresholds::Values(vec![0.5, 2.]),
            ..Default::default()
        },
        &ContourLayoutOptions::default(),
    )?;
    assert_eq!(layout.levels.len(), 2);
    let svg = render::to_svg(&layout.scene)?;
    assert!(svg.contains("<svg"));
    layout.scene_interaction().validate()?;
    Ok(())
}
```

`contour_layout` maps this grid extent to the configured plot once.
`contour_levels_layout` accepts existing `ContourLevel` values and an explicit
positive `ClipRect` source extent. Each `ContourPolygon` has an explicitly
closed outer ring and explicitly closed holes. Caller-authored rings must
preserve topology and winding; this adapter does not repair them. One nonzero
filled path keeps each complete level's islands and holes together.

## A reusable density field

`DensityPoint { position, weight }` uses estimator-local logical pixels.
Weights may be positive, zero or negative, and must be finite. Signed density
is a weighted field, not a normalized probability distribution. Negative
weights are neither discarded nor replaced with absolute values. Nonfinite
coordinates, nonfinite weights and Float32 accumulation or blur overflow reject.

`estimate_density` returns an immutable, cheaply cloned `DensityField`. It
retains the aggregate, not the input business points. `values()` exposes the
original Float32 grid, including its padding; `max()` and contour thresholds
are in weight per square logical pixel. Renderer DPR does not change these
units. `dimensions()`, `origin()`, `size()`, `bandwidth()`, `cell_size()` and
`requested_cell_size()` describe the accepted estimator.

Defaults are size `[960., 500.]`, bandwidth `sqrt(420)` and requested cell size
`4`. The effective cell size is a floored power of two. Size components must be
positive and at most 1,000,000; bandwidth is between 0 and 100,000; requested
cell size is between 1 and 1,048,576. The resulting padded grid must also fit
the cell and work budgets. A valid numeric option can still exceed that budget.

```rust
use kartis::*;

fn main() -> Result<()> {
    let config = ChartConfig { width: 360., height: 240., ..Default::default() };
    let plot = ClipRect::plot(&config);
    let field = estimate_density(
        &[
            DensityPoint { position: [plot.width * 0.3, plot.height * 0.5], weight: 1. },
            DensityPoint { position: [plot.width * 0.7, plot.height * 0.5], weight: -0.25 },
        ],
        DensityOptions { size: [plot.width, plot.height], bandwidth: 12., ..Default::default() },
    )?;
    assert_eq!(field.sample_count(), 2);
    let layout = density_layout(
        &config, &field, &DensityThresholds::Values(vec![0.0001, 0.001]),
        ContourLimits::default(), &ContourLayoutOptions::default(),
    )?;
    layout.scene_interaction().validate()?;
    Ok(())
}
```

`field.contour(value, limits)` extracts one level;
`field.contours(&thresholds, limits)` extracts multiple levels.
`DensityThresholds::Count` defaults to 20; `Values` accepts finite explicit
thresholds. `density_layout` reuses the aggregate for thresholds, colors,
legends, formatting and contour visibility. Its field size must exactly match
the current plot size. It translates plot-local coordinates into the scene;
do not add the plot offset to estimator inputs a second time.

Recompute the field when sample positions, weights, bandwidth, cell size or
estimator size change. In a data chart, changed data, source visibility, axes or
plot dimensions can change those positions. An explicit affine display in
other units is possible with `contour_levels_layout`; it does not estimate a
new density in those units.

## Density behind a typed Scatter layout

`scatter_density_layout` takes an actual `ScatterLayout`. Its plot supplies the
estimator extent, and the resolved Scatter axes supply positions. All mapped,
non-hidden observations participate, including zero-size nodes and mapped
nodes whose missing-size policy suppresses their glyph. Points in the padded
estimator region can contribute even when their centers lie outside the plot.
Marker `size_value` never supplies density weight.

`DensityWeights` is keyed by original `(series ID, datum ID)`. An absent key
means weight one; an unknown key or nonfinite weight rejects. Source metadata,
painted sites, indices and resolved positions must agree. Change source data or
visibility through a new Scatter layout rather than editing metadata to filter
an existing one. `DensityObservation` retains raw coordinates, source index,
original IDs, mapped position and actual weight. Source marks remain in the
foreground with their typed picking, fonts, clips and order.
`ScatterLayout` is publicly mutable: validation checks consistency between
metadata, sites and matching marks, but cannot establish the original business
provenance of caller-authored decoration without metadata. The caller remains
responsible for constructing the intended input.

```rust
use kartis::*;

fn main() -> Result<()> {
    let config = ChartConfig::default();
    let scatter = scatter_layout(
        &config,
        &[ScatterSeries::new("observed", vec![
            ScatterDatum::new("a", 1., 2.),
            ScatterDatum::new("b", 3., 4.),
        ])],
        &ScatterLayoutOptions::default(),
    )?;
    let weights = DensityWeights::from([
        (("observed".into(), "b".into()), -0.25),
    ]);
    let layout = scatter_density_layout(
        &config, &scatter, &weights, ScatterDensityOptions::default(),
        &DensityThresholds::Values(vec![0.0001]), &ContourLayoutOptions::default(),
    )?;
    assert_eq!(layout.observations[0].node_id, "a");
    let restyled = layout.recontour(
        &DensityThresholds::Values(vec![0.0002, 0.001]),
        ContourLimits::default(),
        &ContourLayoutOptions { opacity: 0.6, ..Default::default() },
    )?;
    assert_eq!(layout.field.values(), restyled.field.values());
    restyled.scene_interaction().validate()?;
    Ok(())
}
```

`ScatterDensityLayout::recontour` uses privately retained accepted config,
Scatter scene, observations and field. Its public output fields are snapshots;
editing them does not replace the inputs to `recontour`. It re-extracts contours
without re-estimating density or rerunning the Scatter projection. For changed
data, source visibility, weights, bandwidth, cell size, axes or plot extent,
call `scatter_density_layout` again with the current inputs. Hiding a source
mark only in a host does not remove its contribution from an existing field.

## Admission, identity and interaction

Limits may be lowered, not raised above the fixed profile. A multi-level call
charges cumulative contour work across its levels; each new call has its own
budget. Hidden geometry also counts toward Scene admission.

| Resource | Maximum |
| --- | ---: |
| Grid cells | 1,048,576 |
| Contour levels | 256 |
| Algorithm work | 32,000,000 |
| Edges / points | 1,000,000 each |
| Algorithm rings | 20,000 |
| Density input samples | 20,000 |
| Contour Scene commands | 200,000 |
| Scene polygons / rings | 20,000 each |

`ContourLimits` controls computation and `ContourSceneLimits` controls layout
admission. Scatter composition reserves the source scene's command usage
before admitting contours. Complete rings are never simplified or split to
evade a renderer limit. A large path may require SVG, Canvas or CPU raster
rather than a host's smaller semantic `Graphic` representation.

`ContourLayoutOptions` configures namespace `id`, shared `colors` and
`color_profile`, opacity, stroke, `value_format`, `color_legend`, `level_legend`
and hidden level IDs. `ContourLevelMetadata` supplies threshold, original input
index, duplicate rank, geometry counts and optional actual mark ID. Level IDs
combine namespace, threshold bits and current duplicate rank; signed zeros
share an identity. There is no historical threshold cache. Empty levels and
hidden levels have metadata but no datum control. Read accepted IDs from
`layout.levels` when configuring visibility.

`scene_interaction()` packages scene, plot and typed binding for a host update.
Exact picking follows actual path ink, holes and clips; the keyboard anchor is
a real path endpoint. Keep the layout and its interaction binding together.
Compute and validate a proposed layout before publishing it; an error from
these functions does not mutate a previously retained layout.

## Rendering and reference scope

Use `render::to_svg` for standalone SVG, the shared Scene with the Canvas
renderer, or the `raster` feature for CPU output. The `web` host can retain SVG
or Canvas nodes and publish the typed layout. Contour extraction has no
JavaScript runtime dependency. GPU computation, workers, polygon repair and
automatic topology-changing morphs are not implemented here.

The [original-function checker](../scripts/contour-density-reference.mjs) runs
vendored **d3-contour 4.0.2** and **d3-array 3.2.4**, without npm or another
source checkout: `node scripts/contour-density-reference.mjs --check`.
The frozen references contain 546 grid records, 500 density records and
844,260 exact Float32 cells. These figures describe original-function reference
checks, not completed browser acceptance.

Two differences remain explicit. `None` uniformly follows NaN-low semantics;
native D3 `null` handling differs at some grid boundaries for thresholds -1 and
0. With a negative maximum, native bulk density can attach geometry to another
threshold label; Kartis binds each geometry to its own threshold and preserves
independent single-field references. Neither difference is claimed as bulk or
null equivalence. See the [source manifest](../tests/fixtures/contour-density-d3-source.json)
and [third-party notices](third-party-notices.md).

Formal retained-browser functionality passed Chrome, Firefox, Playwright WebKit and Edge with SVG/Canvas at DPR1/2, including independent hole/source pixels, typed input, rejection, SVG publication recovery and cleanup. CPU and the complete headless Apace regression also passed. See the [verification record](verification.md) for the scoped regression scores and preserved failures. Physical touch devices and OS windows are outside these checks. The separate strict compositor screen benchmark remains unresolved; this feature acceptance does not establish full Nivo equivalence.
