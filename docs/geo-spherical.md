# Spherical measurements and sampled circles

This public API measures longitude/latitude geometry on a unit sphere. Input positions and circle settings use degrees. `geo_spherical_distance` and `geo_spherical_length` return radians; `geo_spherical_area` returns steradians. For a chosen sphere radius R, multiply angular distance by R and area by R². This is spherical geometry, not an ellipsoidal geodesic calculation.

```rust
use kartis::*;

let quarter_turn = geo_spherical_distance([0., 0.], [90., 0.])?;
let ring = geo_circle(&GeoCircleOptions {
    center: [15., 25.],
    radius_degrees: 25.,
    precision_degrees: 3.,
    ..Default::default()
})?;
let area_sr = geo_spherical_area(
    GeoSphericalGeometry::Polygon(std::slice::from_ref(&ring)))?;
let perimeter_rad = geo_spherical_length(
    GeoSphericalGeometry::LineString(&ring))?;
# Ok::<(), kartis::ChartError>(())
```

The example produces a sampled small circle. Its area and perimeter describe the generated polygon with great-circle edges between samples, rather than an analytic circle. The shared consumer in `examples/shared/geo_spherical.rs` uses these actual outputs as a map feature, boundary route, distance-symbol values, visible unit annotations, and an actual centroid/containment annotation for each polygon. A second scene measures a spherical octant: area π/2 sr, reversed area 7π/2 sr, and perimeter 3π/2 rad. Both scenes are ordinary `Scene` values usable by SVG, Canvas script, and the CPU raster renderer. The exporter `examples/geo_spherical_export.rs` writes these outputs to an explicit directory; enable the `raster` and `geojson` features and use the repository's existing ABeeZee test font.

## Geometry and validation

`GeoSphericalGeometry` borrows Point, MultiPoint, LineString, MultiLineString, Polygon, MultiPolygon, and GeometryCollection coordinates. It also represents Empty and Sphere. Positions must be finite, with longitude in [-180,180] and latitude in [-90,90]. A measurement allows at most 100,000 aggregate positions including closing coordinates, 4,096 geometry/line/ring/polygon parts, and nesting depth 32. Invalid input returns `Err`.

Every nonempty polygon ring must have at least four positions and end with a coordinate numerically equal to its first coordinate. Empty Polygon `[]` is allowed; a polygon containing an empty ring is rejected. `GeoSphericalGeometry::from(&feature)` only borrows the feature's polygons. Map rendering can close an open `GeoFeature` ring; spherical measurement of that same open ring returns an explicit error. Close and validate authored data before measuring it. Area rejects antipodal or nearly antipodal edges (angular length ≥ π − 1e-8), which do not identify a unique great-circle arc.

Area follows the pinned D3 3.1.1 oriented integral: a clockwise small exterior selects the small region, reversing it selects the complement, and holes use the opposite winding. It does not repair self-intersections or compute geographic union. Polygon and collection contributions add algebraically; overlapping polygons and duplicate Sphere entries can produce an area greater than 4π. Sphere itself has area 4π and streamed length zero.

## Polygon length and closed perimeters

To preserve D3 `geoLength`, a Polygon omits its duplicate final coordinate from the stream and does not add the final closing edge. This differs from a perimeter. For a closed perimeter, pass the complete closed ring to `GeoSphericalGeometry::LineString`, as in the example. For the octant ring `[[0,0],[0,90],[90,0],[0,0]]`, Polygon length is π rad, while the complete closed LineString length is 3π/2 rad. No implicit perimeter helper changes this distinction.

## Circle precision and closure

`GeoCircleOptions` accepts a finite canonical center, radius in [0,180] degrees, strictly positive precision in (0,90] degrees, and `max_points` in [4,100000]. Precision is an angular sample step around the circle. The implementation checks estimated work and each actual emitted point, including closure. Too fine a step for the chosen point budget returns an error.

Original D3 returns an empty ring for zero precision and can fail to terminate for negative precision; this checked API rejects both. It also rejects nonfinite parameters. Original oracle results are preserved separately. To provide a usable exactly closed ring, when the final native sample lies within 1e-12 radians of the first, its coordinate is replaced with the exact first coordinate; otherwise the exact first coordinate is appended. Other sample values and order follow the pinned original function.

A 25° radius circle is demonstrated as actual drawable geometry. A successful circle calculation does not guarantee that every radius, precision, or chosen map projection passes the existing checked renderer: zero/180° circles are degenerate, tiny shapes encounter renderer tolerances, and fine or wide rings can encounter its topology and work limits. Planar Scene hit testing is not a spherical contains operation.

## Independent checks

The portable `scripts/geo-spherical-gallery-geometry.js` consumes a Playwright page and returns a report. From the pinned original D3 bundle it regenerates circle samples, independently projects authored inputs, checks distance/area/length metadata and explicit units, verifies actual fill/symbol pixels, and compares decoded SVG-image and real Canvas export with the existing mean-channel-error threshold <0.5. It checks the sampled geodesic boundary at 0.5 px and uses a separate dense oracle for actual vertex membership. Wrong source coordinates, area values, colors, and Canvas interior pixels are real mutation negative controls. These checks cover the two demonstration scenes; they do not establish arbitrary topology or interactive BrowserChart behavior.

## Centroid and containment

Sensitive polygon calculations use equivalent stable formulas rather than reproducing unstable floating-point outputs from the reference. Small-ring area moments are accumulated in a local frame; near-right-angle edge coefficients avoid the singular derivative of `asin`. A roundoff sensitivity estimate selects this calculation while preserving the original area/line/point fallback. Polygon meridian crossings use a stable edge cross when their comparison is numerically sensitive; exact authored vertices retain the original boundary convention. These engineering estimates are not certified interval bounds. The ordinary fixed-reference tolerance remains unchanged, and independent 80/120-digit witnesses retain the original bad results separately.

`geo_spherical_centroid` returns `Result<Option<[f64; 2]>>` in degrees. It uses the original D3 area vector, then length weighting, then point mean. Empty, Sphere alone and directionless cancellation return `None`. Polygon ring length fallback preserves D3’s asin(cross) weighting, including edges longer than 90°. Collections accumulate authored contributions rather than a geographic union.

`geo_spherical_contains` returns the original spherical predicate. Point uses exact angular zero; lines use D3’s epsilon predicate; polygons use oriented winding and south-pole membership. Boundary behavior varies with geometry and is not uniformly inclusive. Longitude differences at an identical pole can make Point containment false because the original angular calculation is nonzero.

Both operations validate the entire geometry before evaluation, including collection members after an early containing Sphere. They share the 100,000-position, 4,096-part and depth-32 limits, exact ring closure and finite canonical coordinates. Line and ring edges within 1e-8 radians of antipodal return `Err`; an ambiguous direction is distinct from invalid source data. These predicates do not repair topology or replace Scene hit testing.

```rust
use kartis::*;

let region = vec![vec![[0., 0.], [0., 90.], [90., 0.], [0., 0.]]];
let geometry = GeoSphericalGeometry::Polygon(&region);
let center_degrees = geo_spherical_centroid(geometry)?;
let inside = geo_spherical_contains(geometry, [30., 30.])?;
assert!(center_degrees.is_some());
assert!(inside);
# Ok::<(), kartis::ChartError>(())
```

The exporter accepts `--surface direct|offscreen`, `--fallback error|direct` for Offscreen, and `--font sans|system`. Native CPU export explicitly binds the selected generic alias to the supplied ABeeZee face; browser font resolution remains platform-specific. For example:

```sh
cargo run --features raster,geojson --example geo_spherical_export -- output-directory --surface offscreen --fallback direct --font system
```

See [CanvasScript surface options](canvas-script.md) for capability, budget and publication behavior.
