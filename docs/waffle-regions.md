# Waffle group event areas (Unreleased)

`waffle_area_interaction(&layout)` returns a checked `SceneInteraction` with one
logical site for each visible group that occupies at least one grid slot.
The group area includes the full slots, including cell padding. The function
preserves the Scene exactly: SVG, Canvas and raster exports keep their real
cells, empty-cell paint, colors and custom geometry.

```rust
use kartis::*;
let layout = waffle_layout(
    &ChartConfig::default(),
    &[RadialDatum::new("completed", 6.), RadialDatum::new("pending", 3.)],
    &WaffleLayoutOptions { total: Some(10.), ..Default::default() },
)?;
let frame = waffle_area_interaction(&layout)?;
let widget = chart_layout(frame).legend(false);
# Ok::<(), kartis::ChartError>(())
```

Pass the frame to `chart_layout`, `BrowserChart::set_layout` or the corresponding
RasterSurface layout entry. Browser mounting accepts `frame.scene`,
`frame.interaction` as `BrowserOptions::interaction_data`, and `frame.plot` as
`BrowserOptions::interaction.plot`. CPU image Charts use
`chart_layout(frame).raster(fonts, options)`. The shared Surface model handles
camera inversion, hover, activation, hidden series and transactional updates.
Native tests inject the finite Glory PlotPointer protocol into Memory; they
do not establish platform OS event transport.

In Cells mode the first occupied cell remains the real canonical Mark. Its
site retains the group ID, source value, formatted value and accepted anchor.
Other painted cells become physical aliases of that site, so each group has
one keyboard control. Areas mode uses the existing solid union Mark. Empty
slots, zero allocations and groups squeezed out by capacity create no site.
Reallocation can change the canonical cell ID; applications should use the
site's `series` field for the original group identity.

Logical area selection and physical paint observations are separate. Padding
or an opacity-zero cell can select a group while `physical_hit_at` returns
`None`. A painted alias still reports its actual physical ID and canonical
logical ID. Ink extending beyond its declared group area remains observable
as physical ink but does not select that group. Hiding a series disables both.
Regions are independent of live Widget replacements: replacing source ink does
not remove the explicitly declared logical event area. Browser DOM components
can still receive their own native events according to the composition host.

`InteractionBinding::regions` also accepts explicit `InteractionRegion`
rectangles for other existing sites. It requires `PaintedGeometry` and
`Picking::Geometry`; polar and nearest/slice modes cannot be combined with it.
Each owner is unique, every owner must be a real bound site, and total rectangle
count is bounded by `MAX_MARKS`. Coordinates, positive extents and endpoints are
validated before publication. Rectangles include left/top and exclude
right/bottom; the last matching declared region wins. Bounds use Scene
coordinates and follow the accepted camera, but do not follow geometry springs
automatically. Publish fresh Scene and region metadata together after relayout.
Mark opacity and per-Mark clipping do not define this independent event layer;
the accepted input plot and viewport still restrict input.

The adapter checks the complete existing Waffle layout, allocation, generated
marks and binding before conversion. Already adapted regions, aliases, part
metadata and polar bindings are rejected. Public Waffle metadata can still be
coherently edited as supported by the existing cell adapter; there is no private
immutable original-input snapshot. Custom cell factories and Path anchors work
without inventing transparent physical marks.

Nivo's SVG/HTML Waffle area layer is transparent event geometry; its Canvas
lookup uses full cells. This API supplies that occupied-group event role.
Nivo's inclusive shared edges and first-source-cell precedence differ from
Kartis's half-open rectangles and explicit reverse-declaration priority.
`WaffleMode::Areas` continues to mean solid union paint. There is no Nivo
`areaComponent` prop, and this API does not supply a new group Widget factory,
React spring props, arbitrary live Widget export or GPU/worker rendering.

The browser acceptance driver is `scripts/waffle-regions-acceptance.mjs`;
`--chromium-only` runs the CI subset. The shared profiles are in
`examples/shared/waffle_regions.rs`. These APIs are not in published 0.1.0.
