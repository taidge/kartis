# Conic projections and Albers

[简体中文](geo-conic.zh-CN.md)

Conic Equal Area, Conic Equidistant, Conic Conformal and Albers share the geographic map, route, graticule, symbol, SVG, Canvas and CPU Scene pipeline. Retain the map's `GeoProjectionContext` when adding layers so they use the same camera and fitted coordinates.

```rust
use kartis::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let config = ChartConfig::default();
    let features = vec![GeoFeature {
        id: "region".into(),
        label: "Observed region".into(),
        value: Some(25.),
        polygons: vec![vec![vec![
            [-110., 25.], [-110., 45.], [-80., 45.],
            [-80., 25.], [-110., 25.],
        ]]],
    }];
    let map = geographic_map_layout(
        &config,
        &features,
        &GeoMapOptions {
            projection: GeoProjection::conic_equal_area([29.5, 45.5])?,
            fit: GeoFit::Data,
            ..Default::default()
        },
    )?;
    let context = map.projection;
    let mut scene = map.scene;
    context.add_symbols(
        &mut scene,
        &[GeoSymbol::new("station", [-95., 35.], Some(25.))],
        &GeoSymbolOptions {
            clip_to_projection: true,
            clip_labels_to_projection: true,
            labels: true,
            ..Default::default()
        },
    )?;
    if let Some(pixel) = context.project([-95., 35.])? {
        let geographic = context.invert(pixel)?;
        assert!(geographic.is_some());
    }
    let svg = render::to_svg(&scene)?;
    assert!(svg.contains("<svg"));
    Ok(())
}
```

## Choosing a projection

Use `BrowserChart` for retained SVG/Canvas, or `RasterSurface` for an interactive CPU image in Glory. Projection masks require one of these rendering hosts; Glory's semantic Graphic/Text host has no arbitrary clip-mask operation.

| Projection | Construction | Standard latitudes |
| --- | --- | --- |
| Equal Area | `GeoProjection::conic_equal_area([a, b])?` | Default helper: `[0., 60.]` |
| Equidistant | `GeoProjection::conic_equidistant([a, b])?` | Default helper: `[0., 60.]` |
| Conformal | `GeoProjection::conic_conformal([a, b])?` | Default helper: `[30., 30.]` |
| Albers | `GeoProjection::Albers` | Fixed `[29.5, 45.5]` |

The default helpers are `conic_equal_area_default()`, `conic_equidistant_default()` and `conic_conformal_default()`. Standard latitudes are finite degrees strictly inside `(-90, 90)`. `GeoConicParallels` preserves their authored order and canonicalizes signed zero. Equal standard latitudes and the supported cylindrical limiting cases are valid when their computed coefficients are valid.

Each selected family validates its own coefficients. Nonfinite coefficients, multi-turn cones (`|n| > 1`), and Equidistant radial folds inside the latitude domain reject. The resolved coefficient must also lie between the sines of the standard latitudes, with an allowance of eight neighboring floating-point values at each endpoint. This catches rounded difference quotients that falsely produce a cylindrical fallback on some targets. The allowance is an engineering guard, not certified interval arithmetic. A pair rejected by one family may remain valid for Equal Area. Building the public enum directly does not bypass layout validation. Kartis keeps the original coefficient formulas; it does not silently clamp a problematic cone or substitute another projection.

Albers includes its intrinsic longitude rotation of 96° and projected origin from `[-0.6°, 38.7°]`. The application's `center` and `rotation` still apply before that intrinsic projection frame. Kartis fits the result through its normal layout; it does not reproduce D3's fixed default viewport and scale. `Albers` is one projection, without Alaska/Hawaii/Puerto Rico insets or an `AlbersUsa` composite.

## Fit, paths and clipping

`GeoFit::World` fits the finite projected cone sector using its extrema. `GeoFit::Data` fits conservative spherical curved-edge envelopes, preserving polygon holes and seam fragments. Ordinary geographic-map and choropleth entry points use fixed coarse sampling (about two degrees); path-options entry points opt into pixel-adaptive sampling and finer leaf envelopes. Its bounds deliberately enclose curved edges; they are not bit-identical to D3's tight fit. Path precision and work/point budgets remain enforced. Use `geographic_map_layout_with_path_options` to configure adaptive path precision.

Conformal uses a finite latitude profile capped at ±85.0511287798066° after the application camera. Its point, path, stream, clipping and inverse operations share this profile, including its Mercator limiting case. Points outside it do not produce visible coordinates. Equal Area, Equidistant and Albers retain their full finite latitude domains.

`context.projection_clip_path(precision, max_points)?` returns the projection's closed finite world footprint, transformed through the same fit as the map. It is a cone sector or annular sector; cylindrical limits use their actual projected world rectangle. Precision is in logical pixels and must be `0.0001..=64`; the point limit must be `4..=180_000`. The returned path must also fit the shared ClipPath limit of 4096 commands and the Scene budget. Unrepresentable Scene coordinates or an insufficient budget reject.

`clip_to_projection` applies that footprint to symbol ink; the symbol's plot clip remains independently controlled by `GeoSymbolOptions::clip`. `clip_labels_to_projection` applies an independent footprint mask to labels. A label's projection mask does not imply clipping its text to the plot rectangle. Aggregate mask commands, segments and work are checked before many symbol copies can exhaust the Scene budget. Missing symbols configured to skip do not consume a duplicated mask.

## Inverse coordinates and publication

`project` and `invert` use logical Scene pixels. Browser hosts first map client coordinates through the actual element bounds and undo any Scene view transform once. DPR controls the backing image; it is not an extra factor in geographic inversion. Native hosts likewise convert their real event coordinates using the accepted image bounds.

`invert` returns `Ok(None)` outside the intrinsic footprint or finite latitude profile. It does not test polygon coverage or the plot rectangle; use `invert_in_plot` for the latter. Invalid input, nonrepresentable arithmetic and insufficient inverse precision return `Err`. The conditioning check estimates numerical uncertainty and rejects ill-conditioned results; it is an engineering guard, not a certified interval bound for every platform or camera. At poles and the antimeridian, compare spherical positions rather than demanding a unique longitude representation.

Applications own the original `GeoFeature` and `GeoSymbol` data, including `Option<f64>`. A legacy `Mark::value` placeholder cannot establish that an absent measurement was zero. Use typed `InteractionBinding` sites to retain missing values and source identities.

Stage a proposed Scene, interaction binding, source table and projection context together. Publish the Scene first, and replace the accepted application context only after successful publication. Invalid standard latitudes, source values, work budgets or renderer publication must leave pointer inversion and typed selection attached to the previously accepted frame. Rebuild the context when fitting inputs, visibility, camera or dimensions change.

Reference fixtures and guarded inverse tests are under `tests/fixtures/geo-conic`, `tests/fixtures/geo-conic-unsafe` and `tests/geo_conic*.rs`. The unsafe supplement preserves original D3 multi-turn and radial-fold results, including incorrect inverse branches, rather than treating raw reprojection alone as proof of a valid inverse. See [third-party notices](third-party-notices.md) for the pinned reference sources and licenses.

## Midpoint-progress correction and verified scope

The previous gallery audit exposed a real Conic sampling blind spot: quarter/midpoint/three-quarter chord errors could remain below the requested tolerance while a short, sharply turning near-endpoint portion deviated by about nine pixels. The correction applies midpoint progress along the chord to trigger further subdivision for the new Conic projections only, retaining the existing work, depth and point budgets. Separate old/new WASM binaries in Chrome measured 9.01848 pixels before and 0.4967817 pixels after the correction, below the unchanged 0.5 target. This paired diagnostic does not replace the fresh full feature run. Original D3's separate near-pole resampling deviation of about 0.000485 is retained as a reference diagnostic; it does not explain away the repaired Rust blind spot.

The pre-correction 1,126-path cohort, its 183 selected regression passes and original feature evidence remain immutable. That baseline explicitly has featureAccepted=false. No pre-correction pass or image is combined with the corrected binary's results.

Corrected-source Rust and consumer checks: 1,082 minimal-feature tests, 1,285 all-feature tests and 15 doctests passed; strict native/WASM Clippy, Rust 1.95 checks and four rebuilt browser consumers all completed successfully. Fresh four-engine SVG/Canvas and DPR1/2 feature checks: Chrome, Firefox, Playwright WebKit and Edge each passed independent DPR1/2 SVG/Canvas contexts: 208 ordinary/typed cases, plus 96 seam checks, 32 paint-parity checks, 32 invalid proposals, 16 publication failures and 32 resize checks (16 accepted and 16 actual publication failures). All 288 PNGs and 96 raw rollback pairs were independently checked; 144 marker pixels, 480 trusted input packets and 11 tamper controls passed. The maximum paint-plane mean was 0.207068 against the unchanged 0.5 gate; source and HTTP/WASM pins matched. Fresh native focused/full checks: normal-target fmt and strict Clippy passed; focused and full headless Apace runs each passed 16 cases and retained 32 PNGs. All 64 PNGs were independently decoded, with 3,476 exact source-color pixels and 175 native-source pins checked; source hashes matched before and after. Fresh gallery geometry, fit, mask and export checks: 246 actual scenes retain the previous 244 standalone SVG files byte for byte. Both new cards pass the portable original-D3 geometry/paint checker, including 761 authored graticule nodes per card at 1e-9, shared source colors and missing-value representation, plot/projection masks and SVG-image/Canvas means below 0.5. The corrected rotated Albers visible-grid sampled distances are 0.056184 and 0.495643 pixels in the two directions. This is not bit-identical native tessellation or a global Hausdorff guarantee; the separate 25,001-point direct-function witness checks the repaired narrow turn. Corrected-source normal regression and Chrome CLI closeout: one fresh complete cohort 2863059d-9ad6-4490-a2be-a0f1caf966bf passed 183/183, with all 61 explicitly selected suites completed by Firefox, WebKit and Edge; the entire piecewiseScales suite remains excluded. Four system Chrome CLI groups passed separately, with the host strict-compositor CLI excluded. The first corrected-source cohort remains a recorded 182/183 failure because WebKit scaleProfiles timed out waiting for scroll stability. Four unchanged-verifier old/current diagnostics passed without reproducing that timeout; its cause is not established. This final result uses three complete new child reports without reusing partial passes or changing assertions, timeouts or pixel gates.

Ordinary/typed case counts and the additional seam, parity, invalid, publication and resize checks have distinct counting scopes. Accepted resizes and actual resize-publication failures are reported separately. Native-source pins are distinct from browser source and HTTP/WASM pins. Only complete same-version reports support acceptance; partial successes and earlier runs remain separate.

Default geographic-map and choropleth entry points retain fixed coarse sampling with conservative spherical curved-edge envelopes. Path-options entry points opt into pixel-adaptive sampling and finer leaf envelopes. Neither promises identical D3 tessellation or tight-fit bits. AlbersUsa composition, spherical area/length/centroid/contains/circle APIs, arbitrary spherical topology and broader projection options remain open. Projection masks require BrowserChart, RasterSurface or SVG export; semantic Graphic/Text has no arbitrary-mask operation. GPU/worker remain deferred, the older strict compositor benchmark remains unpassed, and the overall D3/Nivo equivalence goal remains active. See [verification](verification.md) and [current gaps](feature-gaps.md).
