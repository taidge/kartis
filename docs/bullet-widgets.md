# Live Bullet part Widgets

[简体中文](bullet-widgets.zh-CN.md) · [Typed slot protocol](typed-plot-widgets.md)

This development API is not included in the published 0.1.0 package.
`bullet_part_widgets(&layout, revision, placement)` creates measured Glory
Widget slots and owned metadata for actual range, measure and marker marks.
Pass its result to `Chart::plot_widgets_with_data`,
`BrowserChart::mount_with_plot_widgets_data` or
`BrowserHandle::set_layout_with_plot_widgets_data`.

The placement callback borrows the current row, part and actual mark. Return
`Some(PlotWidgetPlacement)` to mount an overlay, or `None` to omit it.
Measure the intended inline child explicitly; marker paths can have zero width,
so their ink bounds alone do not define a usable control rectangle. Placement
remains in Scene coordinates. The host applies camera, responsive CSS sizing,
clipping and input eligibility independently of Canvas or raster DPR.

Each accepted `TypedPlotWidgetFrame<BulletPartWidgetData>` contains its
`PlotWidgetFrame` and a snapshot of the current part, visible row index,
within-kind rank, row domain and track. The part preserves source index,
raw value, raw and visible intervals and geometry. Read paint, visibility and
rendered rectangle from `frame`. Dispatch on `data.part.kind` for separate
range, measure and marker components. Metadata-only updates do not replace the
mounted child; stable mark ID and revision retain local state. An explicit new
revision requests a new child.

Ranks include zero or cropped parts which have no actual mark and do not invoke
the placement callback. The synthetic final range has `source_index: None`.
Constant domains, including marker-only `[5, 5]`, are accepted. Row reordering
changes `row_index` while source identity stays stable. This does not decode
metadata from labels, reverse a scale, or create artificial marks.

Build the Scene and packet from the same current layout. In reactive Chart
code, derive both inputs from a shared source, as in the typed slot guide. For
browser updates, submit them together with
`set_layout_with_plot_widgets_data`. Invalid metadata keys, wrong mounted
metadata type, invalid placement, unsupported clipping or invalid Scene retain
the prior accepted frame and data. A placement callback failure returns no
packet and does not change the layout. Application callback side effects remain
application-owned. A child factory/build failure has the existing local
revision behavior: the old child remains and reads the new accepted frame and
metadata; the whole Scene is not rolled back for that child failure.

These are real Glory Widgets above the original Bullet ink. They do not replace
the range/measure/marker drawing, and Scene SVG/Canvas/PNG exports omit their
content. Nivo 0.99 Bullet provides SVG React components with computed rect or
marker data and spring `animatedProps`; it has no Bullet Canvas component.
Kartis exposes its own stable source IDs, ranks and measured Widget lifecycle.
React spring props, arbitrary React content and layer-for-layer replacement
semantics are outside this API.

The executable API example is on `bullet_part_widgets`. The shared model in
`examples/shared/bullet_widgets.rs` and browser-host fixture exercise all three
roles, current source values, row reorder, metadata-only updates, state/focus,
hidden retention, resize/CSS scaling, rejected packets, failed child revisions,
cleanup and remount. See [verification](verification.md#development-typed-bullet-widget-overlays).
