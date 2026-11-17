# Parallel axis filtering

Unreleased development API; not included in crates.io Kartis 0.1.0.

[简体中文](parallel-filtering.zh-CN.md)

`ParallelFilterController` owns a complete parallel-coordinates source. Its
`layout()` is a ready-to-render `ParallelCoordinatesLayout`: filtering removes
rejected row paths, their labels and interaction sites. Axis domains, original
row/source indices, palette allocation and group legends are computed from the
complete source. A rejected row remains in metadata with `drawn == false`;
filtering one row never hides its entire group.

Construct with `new(config, dimensions, rows, options, profiles)` using the same
inputs as `parallel_layout_with_scale_options`. Supply an empty profile map to
use dimension options. `set_filter("temperature", Some(ParallelFilterRange::new([10., 30.])))`
sets one inclusive raw-data interval. Active axes combine with AND; equal
endpoints are valid. `None` clears one axis; `clear()` clears all. Unknown axes,
non-finite or descending data bounds reject. Finite intervals outside the data
domain are allowed and may select no rows.

`ParallelFilterMissing::Exclude` rejects missing values only on actively
filtered axes; `Include` lets those missing slots satisfy that axis. This is
independent of the layout's `ParallelMissing`: a selected record can still be
undrawn because it is hidden or uses `SkipRecord`.

`set_pixel_filter` accepts Scene logical-pixel coordinates along the actual
resolved axis, normalizes reversed endpoints and rejects points outside its
range. It inverts the resolved numeric scale; UTC uses continuous millisecond
boundaries so a fractional interval does not silently include truncated times.
Existing source values exactly on a rendered endpoint are included despite
inverse rounding. Clamped values outside the original domain do not become
endpoint proxies. Rounded scales use their ordinary inverse, not a reconstruction
of all values that rounded to a pixel.

`set_brush_filter` accepts an existing `Brush`: Y is used for Horizontal parallel
layout, X for Vertical. Hosts must convert client/host coordinates into Scene
coordinates first. No browser DOM brush, capture or per-axis control widget is
mounted automatically. This Rust interaction extension is not a claim that
Nivo's parallel-coordinates package exposes an identical brush API.

`replace_filters` admits an entire filter map together. `replace_data` validates
every new record, re-infers axes from the full new source and preserves existing
data-space intervals. Rejected updates leave source, filters, frame and picking
binding unchanged. Existing `options.hidden` stays independent. Use the returned
scene and interaction binding together when updating a host.
