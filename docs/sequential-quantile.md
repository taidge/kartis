# Sample ranks on a sequential color ramp

[简体中文](sequential-quantile.zh-CN.md) · [Library overview](../README.md)

`ColorScaleKind::SequentialQuantile` maps an observed sample distribution to
the existing continuous color interpolators or named ramps. It retains every
duplicate observation. The input-to-rank mapping is a step function: the color
interpolator is continuous, but values between observations do not acquire
linearly interpolated ranks. This is a D3 sample-rank profile, not a new Nivo
configuration prop or a normalized histogram.

## Rank, quantiles and ordinal colors

For sorted samples `d` of length `n >= 2`, a finite query uses
`(bisectRight(d, x, 1) - 1) / (n - 1)`. Equality takes the last rank in a group
of duplicates; finite queries below/above the sample extent saturate at 0/1.
This differs from `count(d <= x) / n` and from R7 quantile interpolation.
For `[0, 0, 5, 10, 10]`, ranks at 0, 5 and 10 are 0.25, 0.5 and 1.

```rust
use kartis::*;

fn main() -> Result<()> {
    let scale = ColorScaleOptions {
        kind: ColorScaleKind::SequentialQuantile,
        colors: vec![Color(0, 0, 0, 255), Color(255, 255, 255, 255)],
        ..Default::default()
    }.resolve([Some(0.), Some(0.), Some(5.), Some(10.), Some(10.), None])?;
    assert_eq!(scale.sample_domain(), Some([0., 0., 5., 10., 10.].as_slice()));
    assert_eq!(scale.rank(0.), Some(0.25));
    assert_eq!(scale.rank(4.), Some(0.25));
    assert_eq!(scale.quantiles(4)?, Some(vec![0., 0., 5., 10., 10.]));
    assert_ne!(scale.rank_colors().unwrap()[0], scale.map(Some(0.)));
    Ok(())
}
```

`sample_domain()` returns the privately retained sorted distribution. Equal
values preserve authored order, including signed-zero spellings. Clones share
immutable storage; changing the original input vector does not change a
resolved scale. At most 20,000 input entries are accepted, counting `None`
toward the input budget while excluding it from the distribution.

`quantiles(segments)` returns R7 statistics at probabilities `0..=segments`,
divided by `segments`: there are `segments + 1` values, including the minimum
and maximum. Valid segment counts are 1 through 256. Invalid counts reject
before work, including for empty distributions or other scale kinds. Original
R7 arithmetic that produces a nonfinite result returns `Err`, even when rank
queries for the same finite source remain valid.

`rank_colors()` samples the ramp at ordinal positions `i / (n - 1)` for every
sorted observation. It is deliberately different from calling `map` on every
sample, since `map` uses the rightmost rank for duplicates. `palette()` still
means the authored/prepared ramp anchors. `bins()` is empty and
`bin_extent`/`invert_extent` return `None`; this kind has no discrete color
buckets or unique color inverse.

| Distribution | `rank` / `map` | `sample_domain` | `rank_colors` | `quantiles`, valid count |
| --- | --- | --- | --- | --- |
| Empty | `None` / unknown | `Some([])` | `Some([])` | `Ok(None)` |
| One sample | `None` / unknown | One sample | One unknown color | Repeated finite sample |
| At least two | Right-rank / ramp | Full distribution | Ordinal ramp samples | R7 values or overflow error |
| Other scale kind | `None` / its own mapping | `None` | `None` | `Ok(None)` |

For two or more identical samples, a query below their value has rank 0 and a
query equal to or above it has rank 1. This does not use the midpoint-color
rule of a constant ordinary Sequential scale.

## Display window, missing values and checked statistics

The new kind requires `ColorScaleTransform::Linear`, an empty `thresholds`
list and an ascending finite `domain` window; a constant window is allowed.
Authored ramps accept 2 through 256 color anchors in the selected interpolation
space; `resolve_with_profile` can instead select a named ramp.
The full sample distribution is independent of that two-endpoint display
window. An omitted window uses the observed extent, or the existing `[0, 1]`
fallback for empty input. Reversing a palette reverses its color direction;
it does not reverse the sorted distribution.

`rank` remains a distribution statistic outside the window. With the default
`clamp=true`, `map` uses that rank even outside an authored window.
With authored interpolation and `clamp=false`, `map` returns unknown outside
the finite window. This is Kartis's explicit finite-window extension, not a D3
SequentialQuantile clamp method. Named schemes retain their existing
`clamp=true` requirement.

`position(value)` and `value_at(offset)` describe the linear **data axis** and
its checked inverse. They do not return rank or invert it. In particular,
`rank(value_at(p))` need not equal `p`, and `map(quantiles(...)[i])` need not
equal the ramp color at that quantile probability.

```rust
use kartis::*;

fn main() -> Result<()> {
    let options = ColorScaleOptions {
        kind: ColorScaleKind::SequentialQuantile,
        domain: Some([2., 8.]),
        clamp: false,
        ..Default::default()
    };
    let scale = options.resolve([Some(0.), Some(0.), Some(5.), Some(10.), Some(10.)])?;
    assert_eq!(scale.rank(0.), Some(0.25));
    assert_eq!(scale.map(Some(0.)), scale.unknown());
    assert_eq!(scale.position(5.), Some(0.5));
    assert_eq!(scale.value_at(0.5)?, 5.);

    let single = options.resolve([Some(7.)])?;
    assert_eq!(single.map(Some(7.)), single.unknown());
    assert_eq!(single.quantiles(2)?, Some(vec![7., 7., 7.]));
    let extremes = options.resolve([Some(-f64::MAX), Some(f64::MAX)])?;
    assert_eq!(extremes.rank(0.), Some(0.));
    assert!(extremes.quantiles(2).is_err());
    Ok(())
}
```

Present source samples must be finite. `None` contributes no observation;
missing and nonfinite queries map to the configured unknown color, and
nonfinite rank queries return `None`. There is no implicit string, Boolean or
object conversion. Original D3's empty domain calls its interpolator with -0;
its singleton domain calls it with NaN. Kartis's unknown-color handling is an
explicit checked difference, independent of how a JavaScript interpolator
happens to render those arguments.

## Exact data-axis legends have their own admission limits

Pass the accepted scale to `append_color_legend`. `stops(steps)` preserves
every effective rank-color jump at its actual data-axis position, using
ordered duplicate offsets. `steps` must still be 2 through 256; it does not
truncate or smooth away jumps. Adjacent plateaus with identical RGBA may merge,
but surviving boundaries never move. One gradient rectangle avoids splitting
the ramp into separate antialiased strips.

```rust
use kartis::*;

fn main() -> Result<()> {
    let scale = ColorScaleOptions {
        kind: ColorScaleKind::SequentialQuantile,
        colors: vec![Color(0, 0, 0, 255), Color(255, 255, 255, 255)],
        ..Default::default()
    }.resolve([Some(0.), Some(10.)])?;
    let mut scene = ChartConfig::default().scene()?;
    let legend = append_color_legend(&mut scene, &scale, &ColorLegendOptions {
        tick_values: Some(vec![0., 10.]),
        ..Default::default()
    })?;
    assert_eq!(legend.stops.iter().map(|stop| stop.offset).collect::<Vec<_>>(), vec![0., 1., 1.]);
    assert_eq!(legend.stops[1].color, Color(0, 0, 0, 255));
    assert_eq!(legend.stops[2].color, Color(255, 255, 255, 255));
    Ok(())
}
```

The final white color above exists only at the endpoint. There is no
positive-width white band and no promise that the last physical pixel becomes
white. Empty/singleton fields draw a flat unknown bar; constant windows use
their actual mapped color.

An individual paint accepts at most **512 stops**; a Scene accepts at most
**16,384 stops** in total. Exact jumps must remain distinct and ordered in both
f64 offsets and backend f32 positions. Interior jumps that round to an endpoint
or collide with another effective jump reject; ordinary f32 rounding alone is
not an error. The API does not raise budgets, downsample observations or widen
bands to force a legend through admission. A distribution of 20,000 samples can
still map colors successfully when its optional exact legend cannot be drawn.
`append_color_legend` rejects transactionally and leaves the old Scene intact.

Automatic legend ticks use the data axis. To choose R7 labels instead, obtain
`quantiles`, remove duplicate values, keep them inside the display window and
pass at most 32 `tick_values`. A percentile-axis legend would be a different
coordinate contract; it is not implied by these labels.

## HeatMap, Calendar and shared-color Choropleth

Use `HeatmapLayoutOptions::colors` with `heatmap_layout`,
`CalendarLayoutOptions::colors` with `calendar_layout`, or the explicit
`choropleth_with_color_scale` / `choropleth_with_color_profile` entry points.
The older independent `choropleth_layout` does not adopt a shared scale
automatically. The `*_with_color_profile` paths accept the same interpolation
spaces or named ramps as the standalone scale.

```rust
use kartis::*;

fn main() -> Result<()> {
    let config = ChartConfig::default();
    let colors = ColorScaleOptions { kind: ColorScaleKind::SequentialQuantile, ..Default::default() };
    let values = [Some(0.), Some(0.), Some(5.), Some(10.), Some(10.), None];
    let cells = values.iter().copied().enumerate().map(|(i, value)| HeatCell {
        row: "observed".into(), column: format!("c{i}"), value,
    }).collect::<Vec<_>>();
    let heat = heatmap_layout(&config, &cells, &HeatmapLayoutOptions {
        colors: colors.clone(), axes: HeatmapAxes::none(), labels: false, ..Default::default()
    })?;
    assert_eq!(heat.cell(&HeatmapCellKey::new("observed", "c2")).unwrap().value, Some(5.));

    let days = values.iter().copied().enumerate().map(|(i, value)| {
        CalendarCell::new((2024, 1, i as u8 + 1), value)
    }).collect::<Vec<_>>();
    let calendar = calendar_layout(&config, &days, &CalendarLayoutOptions {
        colors: colors.clone(), start: Some((2024, 1, 1)), end: Some((2024, 1, 6)), ..Default::default()
    })?;
    assert_eq!(calendar.day((2024, 1, 6)).unwrap().value, None);

    let features = values.iter().copied().enumerate().map(|(i, value)| {
        let x = -75. + i as f64 * 25.;
        GeoFeature { id: format!("region-{i}"), label: format!("Region {i}"), value,
            polygons: vec![vec![vec![[x, 0.], [x + 12., 0.], [x + 12., 12.], [x, 12.], [x, 0.]]]],
        }
    }).collect::<Vec<_>>();
    let map = choropleth_with_color_scale(&config, &features, &GeoMapOptions {
        fit: GeoFit::Data, border: None, ..Default::default()
    }, &colors)?;
    let missing = map.scene.marks.iter().find(|mark| mark.series == "region-5").unwrap();
    let original = features.iter().find(|feature| feature.id == missing.series).unwrap();
    assert_eq!(original.value, None);
    assert_eq!(missing.value, 0.);
    assert_eq!(missing.fill, Some(map.color_scale.unknown()));
    Ok(())
}
```

HeatMap resolves the distribution from cells in non-hidden rows/columns.
Calendar uses supplied values from non-hidden years; generated missing days do
not become zero observations. Hiding, adding/removing data or changing duplicate
counts requires a new layout/scale. Reordering samples preserves mapped colors;
equal-value signed-zero spellings retain their input order in the getter.
The shared-color Choropleth uses **all supplied feature values before geographic
projection clipping**. Changing the camera alone does not redefine its sample
distribution.
Each consumer retains its own layout budgets; the color scale's 20,000-entry
limit does not increase them.

HeatMap and Calendar retain nullable raw values in their computed cells/days
and tooltip data. The shared Geo result currently provides Scene, projection
and color scale, not an equivalent nullable feature-metadata/binding helper.
Its legacy `Scene` scalar uses `0.` for a missing `GeoFeature::value`.
Keep the authoritative `GeoFeature` table keyed by original ID, and use it to
construct application-owned nullable tooltip/interaction data. Do not infer
an observed zero from that placeholder. Rank changes color, not business values,
IDs or application selection identity.

## Original-function evidence and current scope

`node scripts/continuous-quantile-reference.mjs --check` executes the preserved
original d3-scale 4.0.2 / d3-array 3.2.5 bundle without npm or another checkout.
The immutable reference covers 124 distributions; the finite n>=2 comparison
profile has 109 distributions and 7,021 finite rank queries. The separate
`continuous-quantile-overflow-reference.mjs --check` preserves six edge cases,
including 24 nonfinite original R7 requests that the checked API rejects.
See the [source manifest](../tests/fixtures/sequential-quantile-d3-source.json)
and [third-party notices](third-party-notices.md).

Core, original-function, consumer, final native and fresh four-engine browser
acceptance have passed for this feature. SVG/Canvas at DPR1/2 cover full samples,
raw nullable values, exact data-axis jumps and actual publication/resize faults.
The explicit three-engine regression subset passed 177/177, with the unresolved
piecewiseScales suite excluded; four system Chrome CLI groups passed separately.
The SVG Mark/Label cache rollback defect was reproduced and repaired. The later
SVG wrapper wave also repaired five demonstrated clip/mask/reparent/first-blend
faults: 40 normal-host cases in four engines at DPR1/2 and a fresh 180-case
explicit regression subset, excluding the entire unresolved piecewiseScales suite.
That wrapper result is SVG-only; companion Canvas, arbitrary persistent DOM
faults, legacy proposal-cache atomicity and physical-device certification are
not claimed. See the [verification record](verification.md).
The older strict compositor-screen benchmark remains failed and separate.
This is not full D3/JavaScript/Nivo parity; GPU/worker remain deferred.
