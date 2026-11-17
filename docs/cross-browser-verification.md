# Cross-browser verification

## Kartis rename acceptance (2026-11-13)

The project, active identifiers and documentation now use Kartis. The configured Git origin is `https://github.com/taidge/kartis.git`; this local setting does not migrate or publish a GitHub repository. Historical acceptance artifacts retain their original bytes and paths, including legacy names in those paths. The earlier dated sections below retain their own source-revision scope.

Fresh checks of the current Kartis revision passed 1,424 minimal-feature tests (248 suites), 1,691 all-feature/all-target tests (262 suites), 17 doctests, strict native/WASM Clippy and three actual Rust 1.95 checks. The 248-scene gallery was regenerated. The root run ended terminal/passed with exit 0 in `target/kartis-rename-root-final-state.json`; its command log is `target/kartis-rename-root-first.log`.

Ten fixture fonts were regenerated and checked by their original producers; outlines, metrics, character maps and color tables remain exact, with only name/head bytes changed (`target/kartis-font-regeneration/receipt.json`). Original numerical references were rerun and checked (`target/kartis-rename-reference-regeneration/check-receipt.json` and `additional-producers-receipt.json`). Four rebuilt browser consumers passed their strict checks, and the native headless Apace consumer passed (`target/kartis-consumers/browser-build-receipt.json`, `native-build-receipt.json`). Fresh four-engine Typography, ordinary text-baseline and COLR checks passed all 12 contexts (`target/kartis-consumers/font-browser.json`); all 16 demo-load contexts passed (`target/kartis-demo-smoke/report.json`). These checks do not establish strict compositor or full D3/Nivo equivalence. The current master receipt is `target/kartis-rename/acceptance.json`.

## Calendar/Radar/kerning integration epoch (2026-11-13)

Five core sources now integrate Calendar month spacing, Radar owned point-label text and consistent Canvas kerning. The formal application is `target/viora-final-application/receipt.json`. This is a new source epoch after color-factory acceptance; the historical `0418…` freeze and color artifacts retain their earlier scope. Formal new-epoch checks passed 1,424 minimal tests (248 suites), 1,691 all-feature/all-target tests (262 suites), 17 doctests, strict native/WASM Clippy and actual Rust 1.95 checks. `target/viora-final-root-final-state.json` records terminal/exit 0.

`CalendarLayoutOptions::month_spacing` defaults to zero. Positive FullYears spacing separates months along the week axis and moves dates, month bounds and labels together; Compact rejects positive spacing, while month-outline padding remains independent. The borrowed `RadarPointLabelContext` factory changes only eligible owned text, retaining raw values, logical positions, tooltips, interaction identity and colors. Geometry and text factories compose without retaining callbacks in layout/hover caches. See the [Calendar guide](calendar-month-spacing.md) and [Radar guide](radar-point-labels.md).

Source and static-paint evidence remain distinct. Calendar's independent eight-context, 32-frame SVGImage/Canvas checks reached maximum mean 0.15039879261363637; all 23,424 source RGBA readbacks had zero channel error, and all 32 transparent-erasure/saved-bitmap restorations were exact. This is static-export coverage in `target/viora-calendar-independent-review/execution-receipt.json`. Combined candidate `target/viora-combined-validation-stage/READY.json` (`9cdc06…`) binds the five now-applied core sources; its eight static contexts reached maximum mean 0.31948153409090907, still strictly below 0.5, with independently recorded source pixels, PNG/gzip readback and bitmap restoration. These results are not a new four-host interaction matrix or a substitute for formal full-repository checks.

The original Radar Chrome DPR2 mean 0.5215916193181818 remains a recorded failure. An actual old Root build with only six owned strings replaced was byte-identical to the original label-factory Scene/Layout/Binding Debug, SVG and pre-fix CanvasScript. The failure is therefore not attributed to new label geometry. The correction explicitly selects `fontKerning = normal`, retaining the alphabetic baseline, original face and size, and sets it only when the Canvas prototype provides the member. Unsupported contexts do not create or overwrite an expando. The earlier unguarded-setter failure remains preserved.

Independent direct Rust `render::draw_canvas` execution contains 108 real `fillText` calls across eight contexts: six natively supporting contexts made 72 normal draws; two unsupported WebKit contexts made 36 fallback draws. This does not establish native normal-kerning support in all eight contexts. Eighteen PNG/gzip RGBA frames matched, with absent-prototype and existing-own-setter negatives proving no expando creation/write. Existing begin/reset still owns frame state; whole caller-Canvas state preservation is not promised. The independent receipt is `target/viora-text-kerning-browser-consumer/execution-receipt.json` (`7b04343a…`).

Compatibility is representation-specific: tested layouts, Debug, SVG and CPU PNGs retain their bytes. CanvasScript intentionally adds capability-guarded normal kerning: 17 of 22 combined exports are unchanged non-JavaScript files; the five JavaScript files differ only by that renderer policy. The earlier 152-profile/228-file byte proof does not extend to the corrected scripts. A separate default-regression receipt, `target/viora-final-default-regression/receipt.json` (`9efd16ae…`), checks 483 files: 482 are byte-identical; the sole records.json delta contains 1,230 expected guard/normal insertions in 128 profiles’ Canvas fields, with 896 fields and the complete JSON strictly checked. The 152 default profiles retain their 176 CPU PNGs and remaining exports, while all 255 color-export files also retain their bytes. Strict physical compositor parity, arbitrary fonts/devices, full React lifecycle equivalence and GPU/worker remain outside this result.

## BoxPlot per-summary color-factory acceptance (2026-11-13)

The selected color-factory scope is accepted: borrowed per-summary RGBA functions combine with caption factories, feeding bodies, default outliers, inherited strokes and ByLabel's first representative. Fixed stroke/outlier overrides remain independent. Controlled hover and legend input reuse cached colors; source replacement supplies the required factories explicitly. See the [English guide](boxplot-color-factory.md), [中文指南](boxplot-color-factory.zh-CN.md) and master receipt `target/viora-colors-function-acceptance.json` (`89948aae…`).

- **The tested color-factory source revision** passed 1,407 minimal tests, 1,671 all-feature/all-target tests, 17 doctests, strict native/WASM checks and MSRV checks. These counts do not describe the later Calendar/Radar/text-rendering integration epoch. All 204 frozen source/package/Native inputs held at acceptance; that freeze is now released and subsequent changes need separate revision evidence.
- The original Nivo reference executed 32 main profiles, 16 qualifications and 81 cold calls, recording partial-domain priming separately from actual-summary calls. Typed per-summary contexts retain explicit None versus Number(null), transparent-RGB, invalid-CSS and arbitrary-accessor boundaries. This is not full mounted React lifecycle or callback-count parity.
- Native production contains 63 frames, 126 CPU PNGs and 160 rejections, with 1,200 independently checked CPU color witnesses. Four engines at DPR1/2 passed 504 whole SVGImage/Canvas pairs: maximum mean channel difference 0.09683333333333333 against the unchanged strict <0.5 gate. All 9,600 source RGBA readbacks stayed within 2; 504 actual erasure/bitmap restorations had zero changed channels. Bitmap restoration is not renderer redraw/recovery.

The complete second four-host cohort passed Chrome, Firefox, Playwright WebKit and Edge at DPR1/2: eight contexts, 256 logical hosts, 288 actual mounts, 1,024 frames, 5,120 original UTF-8 rejection returns, 6,080 trusted legend pointer/Enter/Space inputs and 992 true physical observations. An independent rational model recomputed raw input order, R7, full-index bands/value projections, function colors, fixed/inherited roles, first representatives and canonical targets. All 84,384 body/median-over-body/outlier source samples stayed within 2, including white alpha-zero body negatives without fabricated positive ink. All 3,552 normal Canvas/Raster whole-bitmap restorations, 2,560 rejected-operation bitmap comparisons and 128 actual erasure restorations were exact.

Independent readback checked 1,134,384 CAS logical/history references, 41,920 logical PNG references (3,844 globally unique PNG contents), and 26,208 complete original DOM/computed-CSS/resource captures. Detailed counts and provenance are in `target/viora-colors-function-independent-host-audit/final-receipt.json` (`ec2f70a5…`) and `terminal-receipt.json` (`6b679dd5…`). Aliases are not counted as independent images.

The first Windows EPERM atomic-report-publication failure, interrupted processes and original reports/images remain preserved. All four original second-cohort processes and all four independent audit processes exited 0; no first-cohort passes were combined. An isolated diagnostic reproduced the open-reader-descriptor rename failure and successful publication after closing that descriptor. This establishes a mechanism consistent with the first failure, not direct proof of its sole cause. The rerun only stopped live-report reads; driver, package, cases and the 0.5/RGBA-2/zero-difference gates stayed unchanged.

Physical Chart/SVG screenshots remain diagnostic; complete DOM/CSS/resource and native Canvas/Raster bitmap gates remain hard. Strict compositor parity, physical-device certification, arbitrary raw getters, full React lifecycle equivalence, GPU/worker and overall D3/Nivo equivalence are not accepted by this result. Earlier entries retain their own source-revision scope.

## Earlier naming acceptance (2026-11-13)

This earlier naming cohort covered the then-current project, packages, build entry points, CSS/DOM identifiers, bilingual documentation and internal fixture fonts. Its acceptance records and results below belong to that earlier revision.

Checks of that earlier revision passed 1,407 minimal-feature tests, 1,671 all-feature/all-target tests, 17 doctests, strict native/WASM Clippy and Rust 1.95 checks. All four browser consumers passed strict checks and release builds. The 248-scene gallery passed the Chrome verifier; the four demos loaded in all four engines, and the headless Apace consumer ran successfully. All 15 README examples and the color-factory guide ran, with matching bilingual code blocks and valid relative documentation links. Default regressions retain 176 byte-identical CPU PNGs: 227 of 228 complete files are identical, and the remaining JSON differs only by the explicit brand rename. See `target/viora-rename/acceptance.json`. Color-factory host and standalone paint acceptance is recorded separately; these rename checks do not establish full D3/Nivo equivalence.

The host and raster demo packages were rebuilt after font regeneration. The four WASM files tested in that earlier cohort contained no preceding brand in ASCII or UTF-16. Fresh four-engine Typography, ordinary-text-baseline and COLR checks passed all 12 selected suites after that rebuild; the separately frozen color-factory Widget package did not change. See `target/viora-rename/font-browser-first.json` and `target/viora-rename/generated-wasm-brand-check.json`.

## Midpoint-progress correction and verified scope

The previous gallery audit exposed a real Conic sampling blind spot: quarter/midpoint/three-quarter chord errors could remain below the requested tolerance while a short, sharply turning near-endpoint portion deviated by about nine pixels. The correction applies midpoint progress along the chord to trigger further subdivision for the new Conic projections only, retaining the existing work, depth and point budgets. Separate old/new WASM binaries in Chrome measured 9.01848 pixels before and 0.4967817 pixels after the correction, below the unchanged 0.5 target. This paired diagnostic does not replace the fresh full feature run. Original D3's separate near-pole resampling deviation of about 0.000485 is retained as a reference diagnostic; it does not explain away the repaired Rust blind spot.

The pre-correction 1,126-path cohort, its 183 selected regression passes and original feature evidence remain immutable. That baseline explicitly has featureAccepted=false. No pre-correction pass or image is combined with the corrected binary's results.

Corrected-source Rust and consumer checks: 1,082 minimal-feature tests, 1,285 all-feature tests and 15 doctests passed; strict native/WASM Clippy, Rust 1.95 checks and four rebuilt browser consumers all completed successfully. Fresh four-engine SVG/Canvas and DPR1/2 feature checks: Chrome, Firefox, Playwright WebKit and Edge each passed independent DPR1/2 SVG/Canvas contexts: 208 ordinary/typed cases, plus 96 seam checks, 32 paint-parity checks, 32 invalid proposals, 16 publication failures and 32 resize checks (16 accepted and 16 actual publication failures). All 288 PNGs and 96 raw rollback pairs were independently checked; 144 marker pixels, 480 trusted input packets and 11 tamper controls passed. The maximum paint-plane mean was 0.207068 against the unchanged 0.5 gate; source and HTTP/WASM pins matched. Fresh native focused/full checks: normal-target fmt and strict Clippy passed; focused and full headless Apace runs each passed 16 cases and retained 32 PNGs. All 64 PNGs were independently decoded, with 3,476 exact source-color pixels and 175 native-source pins checked; source hashes matched before and after. Fresh gallery geometry, fit, mask and export checks: 246 actual scenes retain the previous 244 standalone SVG files byte for byte. Both new cards pass the portable original-D3 geometry/paint checker, including 761 authored graticule nodes per card at 1e-9, shared source colors and missing-value representation, plot/projection masks and SVG-image/Canvas means below 0.5. The corrected rotated Albers visible-grid sampled distances are 0.056184 and 0.495643 pixels in the two directions. This is not bit-identical native tessellation or a global Hausdorff guarantee; the separate 25,001-point direct-function witness checks the repaired narrow turn. Corrected-source normal regression and Chrome CLI closeout: one fresh complete cohort 2863059d-9ad6-4490-a2be-a0f1caf966bf passed 183/183, with all 61 explicitly selected suites completed by Firefox, WebKit and Edge; the entire piecewiseScales suite remains excluded. Four system Chrome CLI groups passed separately, with the host strict-compositor CLI excluded. The first corrected-source cohort remains a recorded 182/183 failure because WebKit scaleProfiles timed out waiting for scroll stability. Four unchanged-verifier old/current diagnostics passed without reproducing that timeout; its cause is not established. This final result uses three complete new child reports without reusing partial passes or changing assertions, timeouts or pixel gates.

Ordinary/typed case counts and the additional seam, parity, invalid, publication and resize checks have distinct counting scopes. Accepted resizes and actual resize-publication failures are reported separately. Native-source pins are distinct from browser source and HTTP/WASM pins. Only complete same-version reports support acceptance; partial successes and earlier runs remain separate.

Default geographic-map and choropleth entry points retain fixed coarse sampling with conservative spherical curved-edge envelopes. Path-options entry points opt into pixel-adaptive sampling and finer leaf envelopes. Neither promises identical D3 tessellation or tight-fit bits. AlbersUsa composition, spherical area/length/centroid/contains/circle APIs, arbitrary spherical topology and broader projection options remain open. Projection masks require BrowserChart, RasterSurface or SVG export; semantic Graphic/Text has no arbitrary-mask operation. GPU/worker remain deferred, the older strict compositor benchmark remains unpassed, and the overall D3/Nivo equivalence goal remains active. See [verification](verification.md) and [current gaps](feature-gaps.md).


## Retained SVG wrapper transactions accepted (2026-11-12)

The normal rebuilt host passed five one-shot SVG transaction faults in four engines at DPR1/2: rectangular clip removal, path-mask removal, failed reparent after successful detach, Label-mask removal and first blend-wrapper style publication. The eight contexts contain 40 cases, 80 PNGs and 80 raw HTML sidecars. Only the exact surface-root data-revision is normalized. Retained identity is a recorded browser observation, not inferred from HTML alone.

Two explicit probes cover 16 committed-source/raw-value/typed-value-7 cases, data controls, 32 trusted pointer/Enter packets and successful retry. The other 24 legacy cases do not establish phase/mode application-cache atomicity. Their companion Canvas is excluded. Arbitrary persistent DOM failure, physical-device input and cross-surface atomicity are not claimed.

Independent PNG validation checks CRC, decoding and Scene-versus-CSS-capture dimensions, with same-engine/version geometry replay. Original bounding boxes were not recorded. Installed Playwright enclosingIntRect outward rounding at fractional y adds one CSS row to the screenshot, not to Scene/backing dimensions. Forty fixed solid-interior 3×3 pairs cover 720 RGBA pixels; this is not whole-screen bit equality or an antialiasing exception. Eighteen meaningful tamper controls are rejected.

One fresh concurrent cohort passed 180/180: 60 explicitly selected complete suites per Firefox, WebKit and Edge, with the entire unresolved piecewiseScales suite excluded. Three complete raw reports, timestamps, child exits, suite identities and source/runtime hashes are bound to the same cohort. Four system Chrome CLI groups passed separately; the host strict benchmark remains excluded.

This wave reran normal WASM strict Clippy/MSRV and all four browser consumer builds. Native full-test counts from the previous SequentialQuantile wave remain historical, not new results. The summary is target/acceptance-summary-svg-wrapper.json, bound to 1,002 frozen files. Earlier RED, mixed candidate, isolated-stage and formatting failures remain separate. The strict compositor benchmark and overall D3/Nivo equivalence remain incomplete; Conic/Albers are still preparation and GPU/worker remain deferred.

## SequentialQuantile feature acceptance (2026-11-12)

The same frozen root implementation passed fresh Chrome, Firefox, Playwright WebKit and Edge feature runs, each with SVG/Canvas at DPR1/2. The eight contexts contain 816 checks, 96 actual-screen PNGs, 336 rejected candidates, 48 publication faults and 48 resize faults. Actual raw values, missing values, full-sample right ranks, data-axis hard stops, pointer/Enter interaction, partial per-surface success and cleanup were checked.

The largest of 408 reported SVG-image/Canvas paint means was 0.333434 against the unchanged 0.5 gate. All 24 wrong-smooth-gradient controls exceeded 0.5; the smallest was 1.079727. Independent offline evidence decodes the 96 PNGs and rechecks captured complete fault states. It does not claim to recompute unretained per-state paint buffers. The native audit separately covers 24 final Apace PNGs, 126 business pixels and 60 legend pixels; explicit UiEvent/Bounds adapters are not physical-device certification.

A fresh explicit subset passed 177/177: 59 suites in each of Firefox, WebKit and Edge, excluding the entire unresolved piecewiseScales suite from the 60-suite catalog. Four system Chrome 157 CLI groups passed; the host strict benchmark is excluded. An earlier CLI driver used bundled Chromium 141 and failed exact dash opacity; that original failure is retained and is not counted as Chrome success.

The 244-scene gallery preserves all previous 241 standalone SVG bytes. The three new scenes have independent original-D3 rank/stop/geometry checks and real DOM negative controls. Same-runner old/new binary tests reproduce then repair the SVG Mark/Label cache rollback defect. Five structure rollback failures remain open: three clip/mask cases in the normal host, plus Label mask removal and first blend creation in an independently compiled probe using the same frozen renderer. The candidate is unapplied; arbitrary DOM-fault recovery is not claimed.

The feature summary is target/acceptance-summary-sequential-quantile.json, bound to a 1,000-file source/runtime freeze and actual HTTP/disk WASM hashes. Earlier strict compositor failures remain failed, Conic/Albers remain preparation, full D3/Nivo equivalence remains incomplete, and GPU/worker work remains deferred.

## Contour/Density feature acceptance (2026-11-12)

The formal root Contour/Density feature passed its dedicated Chrome, Firefox, Playwright WebKit and Edge checks with SVG/Canvas at DPR1/2. Independent validation covers 96 PNGs, 16 SVG publication-fault records and 32 original HTML sidecars, including hole/ink witnesses, trusted pointer and Enter input, current raw source identity, threshold reuse, rebuilds, admission rejection, resize recovery and cleanup. All SVG/Canvas means were below 0.01425 against the unchanged 0.5 gate; removed-contour negative controls were rejected.

The fresh explicitly scoped regression run passed 174/174: 58 suites in each of Firefox, WebKit and Edge, excluding the entire piecewiseScales suite from the current 59-suite catalog. Four Chrome CLI groups passed: gallery, Canvas example, Widget and raster. The host CLI group containing the unresolved strict screen benchmark is not credited as a pass. These scores do not establish a complete browser matrix or strict compositor acceptance.

Gallery validation now requires exactly 241 scenes. The previous 239 SVGs remain byte-identical; two new scenes have independent original-D3 topology, axes, gradient-legend and 200-source-glyph checks, plus ten deliberate page-damage negative controls. Native checks passed 1,031 minimal tests, 1,231 all-feature tests and 13 doctests, strict Clippy, Rust 1.95/WASM checks, consumer builds, 29 CPU/Glory smoke scenes and the full headless Apace regression with four new Contour/Density cases. The pointer bridge uses Apace hit testing and explicit host-coordinate dispatch; keyboard checks use intrinsic datum buttons. No physical-device or OS-window claim is made.

The dedicated feature receipt uses its 973-file freeze; the later 974-file regression freeze differs only in four gallery test-harness files. Product sources and runtime bytes are identical and are verified file by file. The root host WASM is pinned before and after disk/HTTP execution. The feature summary is target/acceptance-summary-contour-density.json, with overallGoalComplete=false and strictCompositorAccepted=false.

The first feature failure is preserved: a publication error changed only the root DIV data-revision telemetry. Complete original strings and structural evidence justify excluding only that root attribute from the live-paint comparison; all other bytes stay strict. The gallery-count failure and interrupted run are separate historical evidence. The previous five advanced-scale failures remain unchanged. Their strict inter-frame screen benchmark is still unresolved; this feature result does not replace it.

## Earlier checkpoint — advanced scales and contour integration (2026-11-12)

The historical completed runs below describe their own frozen revisions. The advanced-scale wave has not passed overall browser acceptance. Its fifth frozen matrix passed 173/174 suites: WebKit's alignment fixture observed a pending-queue change across separate asynchronous reads, with unchanged scene, DOM and local geometry. Chrome CLI failed the strict inter-frame screen benchmark with a raw maximum channel delta of 36. Identical paint inputs and geometry do not turn that result into a pass. No pixel threshold was relaxed.

Independent four-engine advanced-feature groups passed 12/12, and retained regression groups passed 12/12; these are separate results, not a replacement full-matrix score. All five failed attempts retain their original reports and PNGs. The fifth immutable archive contains 71,325 files, with no missing or unparseable references in its independent verification.

Before contour integration, native checks passed 1,012 minimal-feature tests, 1,211 all-feature tests, 13 doctests, strict Clippy, Rust 1.95/WASM checks, consumer builds, 239 gallery scenes and 27 native smoke scenes. New root contour/density core checks passed 16 minimal-feature tests and 17 raster tests, including Rust 1.95. Formal browser integration is in progress. Isolated-stage browser results are not credited to the root build. GPU and worker execution remain deferred.

## Completed matrix — editable Brush and retained regressions

The final frozen run passed all five Chrome CLI groups and all 55 suites in
each of Firefox, Playwright WebKit and Edge, for 165/165. Independent post-freeze
Brush, ordinary Text and scale-profile runs passed in all four engines with
matching host WASM hashes before and after execution. The gallery contains
233 scenes; the 72 configured examples and three additional Brush modes are
checked separately.

The shared Rust Brush supports X/Y/XY selection, movement, eight resize handles,
modifier transitions, keyboard nudging, controlled updates and session retirement.
Each DPR1/2 context checks 134 SVG/Canvas trajectories and 506 steps, plus actual
ink, positive CSS transforms, viewport conversion, publication faults, lifecycle
and cleanup. The summary verifies 32 independent PNG files. Its original D3
oracle has 76 cases; the Rust equal-profile subset has 67 cases, 253 steps,
234 events and 1,648 visible handles. The bounded Alt-to-Space safety difference
is separate. Read-only native Chrome and four-engine event-exact reference
checks passed; event-exact replay is not device acceptance.

The first complete matrix passed 164/165. WebKit's relations keyboard test
failed when focus-induced scrolling moved a chord ribbon under the stationary
mouse. Native events showed Focus/Activated on the node followed by a trusted
hover on the edge; DOM focus stayed on the node button. Two mouse-parking calls
now isolate the keyboard phases. Removing those calls restores every original
verifier byte; all assertions, waits, pixel thresholds and the explicit
pointer-overrides-keyboard check remain. The original failure, causal trace,
comparison and byte proof are archived. Final scores come from a separate fresh
complete run, with all three relations suites passing.

Rust passed 986 minimal-feature tests, 1,178 all-feature tests and ten doctests.
Strict Clippy, consumer builds, Rust 1.95 native/minimal/WASM checks, WASM docs
and both portable READMEs passed. Native Apace headless passed four Brush cases
(DPR1/2 and camera1/2), alongside 34 ordinary-Text cases and 21 CPU/Glory smoke
scenes. Native raw modifiers require application forwarding. Trusted browser
touch taps and injected touch drags are reported separately; physical touch
dragging and screen-reader acceptance remain outside this evidence.

The summary checks 632 frozen source/build/runtime files, 22 actual HTTP
responses, all native and image evidence, original reference generators and
the first failed matrix. Results are in
`target/acceptance-summary-editable-brush.json` and
`target/gallery/cross-browser-editable-brush-report.json`. The final host WASM
SHA-256 is `c9b539177c1afcd34e09478de0b9faa1cc7d748f27edc67a80cf3f0f2bdad511`.
The sections below retain earlier verification checkpoints and their counts.

## Completed matrix — ordinary Text baselines and integrated scale profiles

Chrome passed all five CLI fixtures. Firefox 150.0.2, Playwright WebKit 26.0,
and Edge 157.0.4341.46 each passed all 54 suites in a fresh frozen run, for
162/162. The gallery contains 233 scenes; configured navigation covers 72
examples. Independent post-freeze runs also passed ordinary Text and all
twelve scale configurations in each of four engines, with matching host WASM
hashes before and after execution.

Ordinary HTML Text is compared with SVG and Canvas using actual font metrics,
three anchors, and separate DPR 1/2 contexts: 120 basic cases across engines.
The suite retains its pixel thresholds and eight-pixel negative control. It
also checks 32 positive scale/zoom contexts, 64 unsupported-transform
rejections, 16 Shadow DOM/slot probe groups, real ink collision, whitespace,
font readiness, stale measurement rejection and cleanup. Semantic Text nodes
remain ordinary Text nodes. Generic charts without a host measurer retain
their compatibility placement.

Six retained scale profiles cover the existing mappings plus signed Pow/Sqrt
and reciprocal branches. All original gesture, pixel, source-identity and
retained-node assertions remain. Three actual failed attempts are archived
separately: target selection restricted to a clipped series, an unsuitable
square-scale sample, and a reciprocal sample clipped during Firefox pan.
Only fixture candidates, two fixed samples and their independently checked
D3 coordinate expectations changed. The final run does not reuse previous
passing scores.

The final summary checks 611 frozen source/build/runtime files and 22 actual
HTTP responses. Rust passed 956 minimal-feature tests, 1,141 all-feature tests
and nine doctests; Rust 1.95 native/minimal/WASM checks and WASM docs passed.
Native Apace passed 34 real ordinary-Text cases, alongside 21 CPU PNG and
Glory vector/image smoke scenes. Both portable README files passed validation.

The report is `target/gallery/cross-browser-text-baseline-report.json`; the
combined evidence is `target/acceptance-summary-text-baseline.json`. Its final
host WASM SHA-256 is
`bc8a56ab758b6049cb14fbf6ac44010603f96bdf7ab215667b8bb58fb580d91b`.

## Discrete color scales — focused gallery and retained integration

Chromium 141.0.7390.37, Firefox 150.0.2, Playwright WebKit 26.0 and Edge
157.0.4341.46 each passed `gallery` and `colors`, for 8/8 suites. Each gallery
has 231 scenes. Quantile and threshold examples have independently checked
cell colors and nonuniform legend widths, alongside existing SVG/Canvas pixel
comparisons. The retained suite switches through five profiles, verifies real
pointer and keyboard input, preserved nodes, identical-frame zero mutations,
atomic rejection and cleanup.

The initial 229-scene count assertion rejected the two new examples. Its
failure remains in `target/discrete-colors-initial-count-mismatch.json`.
The corrected 231 count and additional geometry assertions passed in a fresh
run; pixel conditions were retained. Final results are in
`target/discrete-colors-final-browser-report.json` and
`target/acceptance-summary-discrete-colors.json`. This is an explicit subset,
not a new claim that the complete 53-suite matrix or physical devices passed.

## Completed matrix — shared font families, weights, and measurement

Chrome passed all five CLI fixtures. Firefox 150.0.2, Playwright WebKit 26.0,
and Edge 157.0.4341.46 each passed 53 suites in one fresh run against the same
frozen artifacts, for 159/159. The gallery contains 229 scenes and configured
navigation covers 72 examples. All preceding layout, clipping, color-font and
raster-widget assertions remain in the matrix.

The new typography suite loads actual authored Regular/Bold FontFaces and
compares SVG screenshots with Canvas ink. It checks 400/700 faces, CSS 500/600
weight matching, quoted names, host weight pollution, retained font-only
updates, font-aware semantic measurement, legend/tooltip inheritance,
rejection, hiding and cleanup. SVG advances remain tied to the independent
OpenType tables. Firefox Canvas advances are checked against independent
Regular-only/Bold-only FontFace aliases to account for observed engine
quantization, without relaxing ink thresholds.

The summary validates 600 frozen files and 22 actual HTTP response hashes,
including all consumer source/build/runtime files. A post-freeze four-engine
focused run records matching host WASM hashes before and after execution.
Native Apace independently checks actual ordinary Text face selection and
advance, plus painted-Glyph/CPU pixels, rollback and cleanup. Ordinary Text's
approximate baseline remains a recorded follow-up. The 17-scene smoke and
both font generators also pass. See `target/acceptance-summary-typography.json`
and `target/gallery/cross-browser-typography-report.json`.

## Completed matrix — radial labels, COLR paths, and raster widget slots

Chrome passed all five CLI fixtures. Firefox 150.0.2, Playwright WebKit 26.0,
and Edge 157.0.4341.46 each passed 52 suites against the same frozen artifacts
in one fresh run, for 156/156. The gallery contains 227 scenes and configured
navigation covers 72 examples.

Fourteen SVG/Canvas layout configurations now include Pie and RadialBar
ring-thickness filtering and validated polar anchors, measured Sunburst
placement, and text clipped to real padded and rounded arc masks. Dark text
pixels must stay inside those masks; removing the masks must increase ink.

The color-font suite checks two palettes through CPU PNG, semantic Glyph SVG,
standalone SVG, and Rust Canvas rendering, with holes, transparent foreground,
RGBA layers, retained nodes, failure/retry, hiding and owner cleanup. Native
FontFace/Canvas provides an independent default-palette reference with an
opaque foreground. Other palettes and foreground alpha use the independently
read CPAL values and source-over arithmetic. The raster-widget suite exercises
actual Glory Button state over decoded PNG, view/DPR mapping, partial input
clipping, atomic publication, hidden and stale callbacks, cleanup and remount.

The summary validates 574 frozen files, including consumer build configuration
and generated JS/snippets, and 22 actual HTTP response hashes. It checks the
complete Rust pipeline, MSRV, documentation, both READMEs, native Apace color
layers and the 15-scene smoke before publishing current results. See
`target/acceptance-summary-radial-color-fonts.json` and
`target/gallery/cross-browser-radial-color-fonts-report.json`.

## Completed matrix — custom Stream dots, measured labels, and typed raster

Chrome passed all five CLI fixtures. Firefox 150.0.2, Playwright WebKit 26.0,
and Edge 157.0.4341.46 each passed 50 suites in one fresh run against the same
frozen artifacts, for 150/150. The gallery contains 221 scenes and the configured
navigation suite covers 72 examples.

The layout suite now checks ten configurations through SVG and Canvas. New
cases verify offset Stream point ink, nearest anchors, measured rotated
hierarchy labels, displacement around obstacles, and controlled hiding. The
PNG consumer checks mixed-orientation vertical columns against independent
HarfBuzz metrics and decoded ink. It also checks a typed RasterSurface's
nullable rows, original tooltips, explicit host pointer coordinates, actual
keyboard activation, retained controls and image, rejected binding and image
publication, retry, and owner cleanup.

The first focused WebKit and Edge runs sampled keyboard state before their
event queue was processed. The fixture now waits for the actual selected datum
before asserting the unchanged source-row conditions. All four engines passed
the focused rerun and the final complete matrix. Initial diagnostic reports
remain separate from the final results.

The summary verifies 513 frozen source and artifact hashes, the four served
WASM files and gallery HTTP response, current Rust and native evidence, and the
final browser reports. See `target/acceptance-summary-labels-dots-vertical.json`
and `target/gallery/cross-browser-labels-dots-vertical-report.json`.

## Completed matrix — Chord, Bump, Stream, and nullable controls

Chrome passed all five CLI fixtures. Firefox, Playwright WebKit, and Edge each
passed 50 suites against the same frozen artifacts in one run, covering 219
gallery scenes and 72 configured examples.

The new suite checks two-direction Chord values and source dominance, separate
rim and ribbon geometry, Bump offset point ink and nearest anchors, Stream
area picking and sample slices, signed stacks, nullable control labels, and
rejected updates. Every configuration runs through SVG and Canvas with real
pointer and keyboard input. The fixture checks the rejection revision before
an explicit redraw and compares retained markup after the redraw.

See `target/gallery/cross-browser-flow-layouts-report.json` and
`target/acceptance-summary-flow-layouts.json`.

## Completed matrix — Funnel, Waffle, Sunburst, and typed Parallel coordinates

Chrome passed all five CLI fixtures. Firefox, Playwright WebKit, and Edge each
passed 49 suites in one run against the same frozen artifacts. The gallery
contains 212 scenes and the retained host includes 72 configured examples.

The added suite checks Funnel, Waffle cells and areas, and Sunburst in both SVG
and Canvas. It verifies typed values, exact pixels, pointer and keyboard input,
retained path identity, and cleanup. The Parallel suite now checks a rounded
source axis and a UTC dimension alongside the logarithmic and symmetric-log
axes. Rejected interaction metadata leaves the accepted frame intact.

See `target/gallery/cross-browser-layout-models-report.json` and
`target/acceptance-summary-layout-models.json`.

## Completed matrix — rounded radial bars, hierarchy focus, and vertical text

Chrome passed all five CLI fixtures. Firefox, Playwright WebKit, and Edge each
passed 48 suites in one run against the same frozen artifacts. The run covered
207 gallery scenes and 72 configured examples.

The SVG and Canvas checks cover the D3 tangent points for rounded radial bars
and tracks, the empty clipped corner, and picking. The hierarchy fixture checks
retained paths, sampled geometry and typed anchors, interrupted focus, and
settled output. The raster fixture decodes the actual Glory image and checks
two upright ideographs and the vertical punctuation placement.

See `target/gallery/cross-browser-radial-focus-vertical-native-report.json` and
`target/acceptance-summary-radial-focus-vertical-native.json`.

## Completed matrix — custom Line points, BoxPlot, measured labels, and path clips

Chrome passed all five CLI fixtures. Firefox, WebKit, and Edge each passed
48 portable suites on the same frozen artifacts, covering 203 gallery
scenes and 72 configured examples. The new retained suites checked exact
and nearest PointLine picking and actual AreaBump text measurements.
The plot widget suite now checks even-odd holes, camera mapping, instance
isolation, and native input for a fully enclosed widget.

See `target/gallery/cross-browser-line-boxplot-area-mask-report.json` and
`target/acceptance-summary-line-boxplot-area-mask.json`.

## Completed matrix — ordinal colors, custom Radar points, and Bullet scales

Chrome passed all five CLI fixtures. Firefox, WebKit, and Edge each passed
46 portable suites on the same frozen artifacts. The runs covered 200
gallery scenes and 72 configured examples. All four engines checked actual
SVG geometry and Canvas pixels for the five new scenes. The live custom
Radar suite checked exact and nearest picking, the angular slice after zoom,
atomic rejection, and stable retained nodes. The two Radar frames had zero
mean SVG and Canvas channel difference in each engine.

See `target/gallery/cross-browser-ordinal-radar-bullet-report.json` and
`target/acceptance-summary-ordinal-radar-bullet.json`.

## Completed matrix — Radar scales, color ramps, and retained chart features

Chrome passed all five CLI fixtures. Firefox, WebKit, and Edge each passed
45 portable suites on the same frozen artifacts, including 195 gallery
scenes and 72 configured examples. The final matrix and source stamp are
in `target/gallery` under `cross-browser-radar-color-browser-final`.
The build and browser summary is
`target/acceptance-summary-radar-color-browser-final.json`.

Chrome, Firefox, WebKit, and Edge passed the 195-scene gallery. The five new
scenes check logarithmic rings, the symmetric-log zero ring, both HCL hue
paths, and exact Viridis lookup boundaries. Their largest scene mean channel
error was 0.06144. See `target/gallery-radar-color-preview/full-gallery-report.json`.

The same four engines passed retained SVG and Canvas plot widgets and artistic
blend animation on host WASM
`dde0663264461963325ec44b1f898a8df8988188a9bf288819c4c39b044c2141`.
The blend checks exercised two motion profiles with 64 pixel comparisons at
DPR 2. A single live screenshot supplies both complete crops on the same
device pixel grid. Live crossfades had zero difference, and the largest mean
channel error across all comparisons was 0.05479 against the 0.5 threshold.
The widget checks included clipping,
focus, keyboard and pointer input, camera and domain changes, rejection, and
reentrant unmount. See `target/blend-motion-stage/canonical-dpr2-final-proof.json` and
`target/gallery/browser-plot-widgets-report.json`.

## Completed matrix — custom Scatter, blend modes, and semantic plot widgets

Firefox, WebKit, and Edge each passed all 43 portable suites on the same
frozen artifacts. The five Chrome CLI fixtures also passed. The run covered
190 gallery scenes, 72 configured browser
examples, live custom Scatter, 15 artistic blend modes, and measured semantic
Glory plot widgets. See `target/gallery/cross-browser-custom-blend-widgets-report.json`
and the adjacent source stamp. The host WASM SHA-256 was
`0c153f0472866acbddc8e329f8e00bc0abff41a2a8e676ad2e8cf442b5a0a736`.

### Staged custom Scatter and blend checks

The 189-scene custom Scatter checkpoint passed the static gallery in Chrome,
Firefox, WebKit, and Edge. The same four engines passed live SVG and Canvas
checks for the offset hollow path, true clipping, current data, and nearest
origin on an isolated host build. The largest scene mean channel errors were
0.02370 for the static example and 0.02660 for the live fixture. The reports
are in `target/gallery-custom-scatter-preview`.

The blended area snapshot separately passes a numeric Multiply pixel check
in all four engines. The independent expected overlap is `[135,125,131,255]`.
Both SVG and ScriptCanvas were within one channel unit, with exact backdrop
pixels and restored Canvas state. The report is
`target/gallery-blend-preview/blend-geometry-report.json`.

The expanded 190-scene static gallery subsequently passed the integrated
verifier in all four engines. The new Multiply scene's maximum mean channel
error was 0.35159 with the existing 3.5 gallery limit. The report is
`target/gallery-blend-preview/full-gallery-report.json`.

## Completed matrix — Bar, Swarm, Network, and Combo

Chrome 157, Firefox 150, WebKit 26, and Edge 157 passed the full 188-scene
static gallery, including six quantitative geometry checks for the new Bar,
Swarm, Network, and Combo scenes. The largest mean channel error among the
six new scenes was 0.05008334 with the existing gallery limit of 3.5. The
report is `target/gallery-combo-preview/gallery-report.json`.

All four engines passed separate live Swarm and Network glyph checks on
SVG and Canvas with staged WASM
`cac5fef33e5e2314d64b1373775208e42b88c761c5b316eaf158e86d796a2dca`.
The tests cover hollow geometry, current typed data, and retained IDs. Swarm
also verifies group visibility and rejected layouts. Network verifies capture,
CSS and camera mapping, resize rebase, release, and simulation replacement.
Their largest mean channel errors were 0.01672 and 0.05743, respectively,
with the existing live limit of 0.5. The reports are
`target/gallery-combo-preview/swarm-report.json` and
`target/gallery-combo-preview/network-glyph-report.json`.

The official pipeline passed 866 Rust tests, nine crate doctests, and 756
no-default tests. The official host WASM SHA256 is
`044be16c71069d6858eb5919a68ec62b8c359f63bab5a3393c1453ee4ab15b18`.
All five Chrome CLI checks passed with the official artifacts. The host check
includes 72 configured examples, the two new live suites, the parallel hover
fix, and all six Scatter cases and 12 cap modes. The three-engine matrix passed
all 120 suites in one run with the same frozen packages and 188-scene gallery.
Firefox 150.0.2, WebKit 26.0, and Edge 157.0.4341.46 each passed 40/40. The
report is `target/gallery/cross-browser-extended-layouts-report.json`.

## Previous batch — independent typed Scatter

The production layout is implemented and frozen. Its core eight all-feature /
eight no-default tests, independent nine / eight tests (one CPU raster test),
and three / three independent-Y-cap tests have passed. The reference executes
actual Nivo 0.99.0 compute/scale factories: nine profiles, 76 points and 18
domains. These are native/oracle evidence, not browser-suite results.

The 182-scene static gallery has independently passed Chrome and Firefox
150.0.2, including the six new quantitative Scatter assertions. Chrome's scene
mean is 0.03981625362484736 and maximum 0.33926888020833335; Firefox's are
0.17183551897321434 and 0.6044194878472222. Reports are
`target/gallery/scatter-gallery-chrome-report.json` and
`scatter-gallery-firefox-report.json`; the existing gallery limit is 3.5.

The repaired-source official pipeline (execution 19733) passed
**841 Rust tests + nine doctests = 850**, with **733 no-default tests** checked separately.
This adds 28 Rust tests to the previous 813-test baseline. All four consumers
passed formatting, strict checks, release/bindgen and native Apace execution.
The first repaired host WASM SHA256 was
`a54c7c1b0e66f77aa660f788597b5db6961161814286bc12f9c11b886e65610d`.
Rust 1.95 passed native all-targets checks with both the full and minimal
feature sets. The English and Chinese README each contain nine passing Rust
examples. The crate doctest total includes the English set.

The earlier pre-fix checkpoint was 842 including nine doctests, with 726
no-default checks. An independent audit found a real shared Circle-zero stroke
discrepancy: Butt had
no SVG/CPU ink but typed Exact hit it, and Round/Square had SVG/CPU ink while
ScriptCanvas omitted it. The diagnostic is
`target/scatter-circle-zero-audit-browser.json`. That production defect has been
fixed; the original diagnostic remains separate from repaired-source evidence.
The pre-fix result does not establish the corrected renderer/picking contract.

`target/scatter-zero-dash-oracle.json` retains 228 native observations per engine
(Chrome 157.0.8103.39 and Firefox 150.0.2); the engines disagree on degenerate
closed/dashed strokes, and native Canvas omits their caps. These observations
are not a common golden. Kartis normalizes actual single-point strokes through
one shared cap-footprint fill, following [SVG2 dash positions](https://www.w3.org/TR/SVG2/painting.html#StrokeDashing).
At zero length, `[3,3]` offsets 0/1/3/6 are on, 4/-1 off; `[0,3]` is on only
at multiples of 3. Empty/all-zero arrays mean solid. Butt/transparent/zero-width
have no ink, with the existing Scene width admission unchanged.

Admission is explicit: radius-zero Circle, Rect with both dimensions zero, or
a Path whose Move/Line/Cubic support points all coincide and which contains a
Line/Cubic or Close (`M Z` included, Move-only excluded). A zero-area bounding
box alone does not qualify. The repair has seven all-feature / six no-default
tests, plus one / one real semantic Widget test; its existing pure-dash expansion
already produced one correct cap and needed no production Widget change.

Independent canonical static-SVG / ScriptCanvas tests passed in Chrome and
Firefox: **60 rows per engine, three caps per row = 180 cap comparisons per
engine**, all mean zero. These rows are separate from the 228 native observations
per engine. The report is
`target/gallery/scatter-zero-circle-script-browser-report.json`.

Static and retained SVG, Script and typed Canvas, CPU and typed Exact use the
normalized paint representation. Scene geometry, business IDs/values, anchors,
clips and opacity remain intact; an exported SVG path may describe the real cap
footprint instead of the degenerate source commands. Ordinary nondegenerate
paths and bare-Scene legacy hit policy retain their documented semantics.
The live cap suite passed all 12 modes on the first repaired host.

The first repaired host passed all five Chrome CLI checks, including 72
configured entries and six live Scatter cases with two phases each.

The first complete portable matrix passed 113 of 114 suites. Firefox’s
dual-UTC Scatter scene exceeded the 0.5 live pixel limit at 1.74102 mean
channel error. WebKit and Edge each passed all 38 suites. The archived matrix
is `target/gallery/cross-browser-scatter-initial-report.json`.

The UTC example now uses daily date ticks on all four sides with wider
margins and compact coordinate labels that include hours and minutes.
The verifier checks the labels against the raw UTC values. An independent
build passed the full Scatter suite in Firefox, WebKit, and Chrome with
maximum mean channel errors of 0.27573, 0.00459, and 0.01546 respectively.
Each engine passed the 12 live cap modes. The unified package has the same
WASM SHA256, `c480d462744924126761c327a7466cd8c40bab854f83f63fa2e42f2f2e8783dc`.
It passed all five Chrome CLI checks, the 72 configured entries, and the live
Scatter and cap suites. The fresh matrix passed 113 of 114 checks with the
existing thresholds. Firefox and Edge passed 38/38 each. WebKit passed 37/38,
including Scatter. Its sole failure was `canvas parallel false: actual painted
dash hit missed`. The report is
`target/gallery/cross-browser-scatter-before-hover-owner-report.json`.

In the failed WebKit run, a trusted pointer move hit a painted Canvas dash
after leaving a data button. Glory delivered the old button's leave after
the plot update and cleared the new hover. The Browser now tracks the live
pointer region for control enters and keeps the plot's geometric hover until
its own leave. A separate build passed the original parallel verifier and
the new handoff regression on SVG and Canvas in all four engines. Its host
WASM SHA256 is
`16aad349e68c4ce79912ba1ce955c3faa5db94126acc7e6412c9938f5f034202`, and its
report is `target/gallery/parallel-hover-ownership-after-report.json`.

Scatter's current profile includes stable business node IDs and typed raw XY;
default linear min 0 / max auto / nice, and Circle diameter 9; distinct filled
area and stroked-D3 nominal units; actual Point Y top-to-bottom order; full
logical-site indexing with plot-bounded queries; real hollow/partial ink and
Path origins; canonical-domain relayout and independent Y ordinal zoom caps.
No arbitrary React node components, cross-backend blend mode, physical-platform
or GPU/worker equivalence follows from this batch.

The completed pre-Scatter baseline below is preserved independently at
`target/gallery/cross-browser-scatter-previous111-report.json`.

## Last completed round: shared scales and real glyphs

The archived `target/gallery/cross-browser-scatter-previous111-report.json` records **37 suites ×
3 engines = 111/111**, all passing in one fresh complete Windows run with
Playwright **1.56.1**, from **2026-11-10 15:33:25.063 UTC** to
**15:46:34.500 UTC**. Runtime source, fixture packages and verifier JavaScript
stayed frozen throughout that run. No earlier successful suite was merged in.

| Actual `browser.version()` | Suites | Gallery | Configured navigation |
| --- | ---: | ---: | ---: |
| Firefox 150.0.2 | 37/37 | 176 passed | 66 passed |
| WebKit 26.0 | 37/37 | 176 passed | 66 passed |
| Microsoft Edge 157.0.4341.46 | 37/37 | 176 passed | 66 passed |

The official native pipeline passed 813 Rust tests plus nine README doctests
(822 total), with 707 no-default tests checked separately. All five final Chrome
CLI checks passed, including the canonical 176-scene gallery, 66 configured
entries and 32 host extras. Native Rust 1.95 all-features and no-default
all-targets checks also passed; no Rust 1.95 WASM or test-execution claim follows.

| Maximum mean SVG/Canvas channel error, 0..255 | Firefox | WebKit | Edge |
| --- | ---: | ---: | ---: |
| Configured continuous/point scales | 0.17134180 | 0 | 0.01158691 |
| Factory glyphs, including physical animation/Fade/resize | 0.02981608 | 0 | 0.03239014 |
| Geographic glyphs, including resize | 0.12013166 | 0 | 0.03136692 |
| Gallery, mean over 176 scenes | 0.16521191 | 0.02499721 | 0.03909166 |
| Gallery, largest scene mean | 0.60441949 | 0.40265560 | 0.33926888 |

All three new host suites retain their original **0.5** limit; gallery retains
**3.5**. These distinct limits are not combined into a universal sub-0.5 claim.
The four-step Star/Circle/Wye/Triangle/Star Fade chain preserved focus,
selection and current data in each engine, with no ghost controls. Actual Exact
ink/gaps, explicit-origin Nearest, current-frame Path support anchors, nullable
Geo values, real projection masks, legend filtering, resize, atomic rejection,
retained nodes, zero no-op mutations and cleanup passed. This is evidence for
these profiles and builds, not arbitrary glyphs, platforms or pixel equality.

Final host WASM SHA256:
`d9cf74470ea2d245633c234e54bb969eda68f1367ba673efd76a045569808c8b`.
Final gallery verifier SHA256:
`04b948484e2a0f21b674fdb0120f3b266be27c6ed5cba88def47ff625dea91a7`.
Its incremental fixture-only host build followed the successful full pipeline:
unmounted/stopped Glyph and Geo snapshots return inactive rather than throwing
during composed navigation. Geo and all-family composition reports verify both
empty-state contracts on this package; production library code did not change.

The initial attempt was **incomplete**, archived separately as
`target/gallery/cross-browser-glyphs-initial-report.json`: 57 captured statuses,
56 passed and one known measurement failure (Firefox 36/37, WebKit 20/20 before
stopping, Edge not run). It was not a complete 110/111 result. Firefox returned
zero from `getBBox()` on a path inside `defs`; the identical path temporarily
attached to the original outer SVG returned `[225,70,270,270]`. The comparator
removes that hidden clone in `finally`; the independent probe also verifies restored child count. All
four original 0.01 coordinate checks and pixel limits remain unchanged.
Evidence is `target/gallery/globe-gallery-firefox-debug.json`; no clip geometry,
renderer, hit policy or threshold was changed to address this measurement.
The final 111 results came from a new complete run, not those diagnostic checks.

## Previous completed round: 164 scenes and 102 runs

The final source passed **34 suites × 3 engines = 102/102** in one fresh complete
Windows run with Playwright **1.56.1**, from **2026-11-10 13:47:57.677 UTC** to
**13:59:19.090 UTC**. The archived raw result is
`target/gallery/cross-browser-glyphs-previous102-report.json`; every suite and engine passed, with
no merging of results from the earlier 101/102 attempt. Runtime source, verifier
JavaScript and packages stayed frozen throughout this final run.

| Browser reported by `browser.version()` | Suites | Gallery | Configured navigation |
| --- | ---: | ---: | ---: |
| Firefox 150.0.2 | 34/34 | 164 passed | 56 passed |
| WebKit 26.0 | 34/34 | 164 passed | 56 passed |
| Microsoft Edge 157.0.4341.46 | 34/34 | 164 passed | 56 passed |

The previous thirty suites passed again, alongside Pie layout, Line crosshairs,
Radar polar slices and explicit graph relations. All thirteen new gallery
geometry checks passed in each engine. The live Pie fixture uses real font
measurements; the static gallery labels its thirteen new examples as interaction
state snapshots and does not pretend to measure text.

| Maximum mean SVG/Canvas channel error, 0..255 | Firefox | WebKit | Edge |
| --- | ---: | ---: | ---: |
| Pie layout, eight profiles | 0.09843970 | 0 | 0.01331543 |
| Line crosshairs, including resize | 0.03180382 | 0.00448032 | 0.01245605 |
| Radar polar slices, six profiles | 0.07650317 | 0 | 0.00596021 |
| Tree/Network/Chord/Sankey relations, ten profiles | 0.14205450 | 0.36352237 | 0.17021019 |
| Gallery, mean over 164 scenes | 0.15012447 | 0.02682627 | 0.03916217 |
| Gallery, largest scene mean | 0.60441949 | 0.40265560 | 0.33926888 |

The four new suites retain their original **0.5** limits; gallery retains **3.5**.
These different limits are not combined into a claim that every scene is below
0.5. Live SVG isolation keeps its separate ±2 interior-channel checks. Missing
engines, launch errors and page errors remain failures. This establishes these
profiles on these builds, not universal pixel equality or a performance ranking.

All engines also passed the new Line admission regressions: incompatible contour
configuration; 10000 old + 10000 disjoint new marks rejected before queueing in
both animation solvers; the 9999+9999+2 boundary and instant legal replacement;
and tiny dash rejection before either renderer claims DOM. The cumulative
150k-business-dash case rejects both a typed setter and resize result while
preserving accepted pixels/hover/binding/queue. Full decorated Scene admission
includes global dash expansion/work before Model/Cage commit. Target/union
pre-enqueue rejection keeps the old running motion/clock/model; a final frame
failure restores accepted Scene/hover/binding/focus but cancels animation and
live gestures/captures, avoiding repeated RAF failures or revived ended contacts.
The existing admissible Geometry Transform extents (min≥1, max≤100) are unchanged.

Native validation is separate: **769 Rust tests + 9 README doctests = 778**, with
**668 no-default tests** checked independently (43 new Rust tests). The full
pipeline, four consumers, native Apace, strict WASM/rustdoc and final five Chrome
CLIs all passed. Chrome host includes 29 extras and 56 actual navigation entries.
The final host WASM SHA256 is
`3f24ef7e1cc03d5159529c937e7a74d6513ff4dd299a8ce1f8c947447980877a`;
the Radar standalone sourceStamp matches it. Native counts, CPU checks and
browser results are not added together as independent tests. See
[verification](verification.md) for the oracle and pipeline details.

## Retained initial diagnostic (not merged into the final result)

The first complete attempt (2026-11-10 12:58:47.396–13:08:18.499 UTC)
finished **101/102** and is preserved separately at
`target/gallery/cross-browser-interactions-initial-report.json`. Firefox 150.0.2
passed 33/34; WebKit 26.0 and Edge 157.0.4341.46 passed 34/34. All four new suites
passed in every engine. The sole failure was the older HierarchyFocus Canvas
parent-pointer check. The evidence is retained in `hierarchy-firefox-pointer-debug.json`: the old
probe was outside the parent fill and depended on the existing stroke picking
slop. The requested radius distance was 64.8658 for radius 63.2377; the trusted
input shifted by (-0.78248, -0.67362) logical pixels and correctly missed at
65.7836, outside the radius+2.5 threshold of 65.7377. Scroll/box timing was stable.
Only the Hierarchy pointer fixture changed for this diagnosis: a real filled
Rect/Circle candidate must have
at least 1.5px fill inset and pass nine Scene hit tests at ±1.5px. The verifier,
production picking policy and pixel limits stayed unchanged. The new probe has
1.81860px inset (`hierarchy-firefox-newprobe.json`); the original Firefox suite
then passed all four cases (`hierarchy-focus-firefox-report.json`, with hashes).
These observed deltas are not a universal driver quantization guarantee. The final fresh
102/102 matrix above was run after this fixture-only correction and the new
production budget guards; none of the earlier 101 passes was reused.

Historical artifacts: the 151-scene / 90-run raw result is archived as
`target/gallery/cross-browser-previous90-report.json`. Older sections retain
paths used by those runs; canonical names without an archive suffix can be
overwritten by later runs and are not a snapshot of every historical matrix.

## Previous completed round: 151 scenes and 90 runs

The final source passed **30 suites × 3 engines = 90/90** in one fresh complete
run on Windows, using Playwright 1.56.1. It started at
**2026-11-10 11:36:21.414 UTC** and finished at **11:43:30.117 UTC**.
`target/gallery/cross-browser-previous90-report.json` is the raw result; no passes were
combined with an earlier attempt. Source, verifier JavaScript and packages stayed
frozen throughout this final run.

| Reported browser version | Suites | Gallery scenes | Configured navigation entries |
| --- | ---: | ---: | ---: |
| Firefox 150.0.2 | 30/30 | 151 passed | 41 passed |
| WebKit 26.0 | 30/30 | 151 passed | 41 passed |
| Microsoft Edge 157.0.4341.46 | 30/30 | 151 passed | 41 passed |

The existing twenty-seven suites all passed again, alongside the new independent
Marimekko, geographic-stream/graticule and Delaunay suites. These exercise actual
Rust APIs in SVG and Canvas, current raw input/tooltip/keyboard, hidden re-layout,
retained identity, zero unchanged-frame mutations, rejected-update atomicity and
cleanup. Missing engines, launch errors or page errors remain failures.

| Maximum mean SVG/Canvas channel error, 0..255 | Firefox | WebKit | Edge |
| --- | ---: | ---: | ---: |
| Marimekko, eight profiles | 0.02543677 | 0.26335327 | 0.01006567 |
| Geographic streams, six profiles plus hidden | 0.00010579 | 0.00001085 | 0.01922173 |
| Delaunay, six profiles | 0.02834961 | 0 | 0.00074924 |
| Gallery, mean over 151 scenes | 0.14213381 | 0.02497492 | 0.03783144 |
| Gallery, largest scene mean | 0.60441949 | 0.31094206 | 0.33926888 |

The three new suites retain their original **0.5** pixel-error limit. Gallery
limits are separate and unchanged. All eleven new gallery geometry invariants
passed; the minimum colored-pixel counts were 6618 / 6628 / 6694 respectively.
Three new-family gallery screenshots were visually checked. These measurements
establish these profiles on these browser builds, not universal pixel equality
or a cross-engine performance ranking.

The first complete attempt finished **88/90**, retained separately as
`target/gallery/cross-browser-p2-initial-report.json`: Firefox and WebKit each
passed 29/30; Edge passed 30/30. Both failures were an ambiguous Geo pointer probe.
Its requested point was only 0.069 logical pixels beyond a foreground symbol's
stroke/slop boundary. Firefox's observed shift of (-0.337, +0.129) pixels correctly
selected that symbol; the route also geometrically contained the point.
`target/gallery/geo-firefox-pointer-debug.json` retains actual coordinates,
bounds and hover. This was a measurement-context error, not a library failure.

Only the Rust fixture changed: it now chooses a real route midpoint whose full
±1 logical-pixel nine-point neighborhood is independently confirmed by
`Scene.hit` to select the route. Existing plot/paint checks, verifier JavaScript,
library code and thresholds did not change. The sole host workspace passed
strict Clippy/release/wasm-bindgen rebuilding; the other three consumer packages
were unchanged. The new probe survived an observed -0.8084px x shift in Firefox;
the earlier -0.337px was one observation, not a general quantization bound.
All three new Chrome suites and the full host CLI (25 extras, 41 navigation
entries) passed again on that package before the final **fresh 90/90** run.
The earlier 88/90 was not merged into the final result.

Native evidence is separate: **726 Rust tests + 9 README doctests = 735** passed,
with **629 no-default tests** checked separately and 37 Rust tests added.
The complete core pipeline, four consumers, native/WASM strict checks, strict
WASM rustdoc and final five Chrome CLIs passed. Marimekko has 328 actual Nivo-hook
cell oracles, Delaunay has 20 nondegenerate D3 cases plus independent integer
topology checks, and Geo has actual TransverseMercator/graticule/stream oracles
and five peer regressions. Native counts remain valid across the fixture-only
change. Default runtime dependencies include Spade 2.15.1 / robust 1.2.0;
development JavaScript references are outside the runtime. See
[verification](verification.md) and [third-party notices](third-party-notices.md).
GPU/worker and physical-platform coverage remain separate gaps.

## Previous completed round: 140 scenes and 81 runs

On 2026-11-10, the installed Firefox, WebKit and Microsoft Edge each passed all
twenty-seven portable acceptance suites. These are real headless browser runs on Windows,
not simulated browser APIs. The machine and build configuration are the same as
the [density measurement](browser-density-benchmark.md); this run makes no
cross-engine performance claim.

| Browser reported by `browser.version()` | Gallery | Widget | Outline | Paints | Contours | Raster PNG | Patterns | Clips | Combo | Dashes |
|---|---|---|---|---|---|---|---|---|---|---|
| Firefox 150.0.2 | 140 passed | Passed | Passed | Passed | Passed | Passed | Passed | Passed | Passed | Passed |
| WebKit 26.0 | 140 passed | Passed | Passed | Passed | Passed | Passed | Passed | Passed | Passed | Passed |
| Microsoft Edge 157.0.4341.46 | 140 passed | Passed | Passed | Passed | Passed | Passed | Passed | Passed | Passed | Passed |

| Browser | Tree styles | Parallel coordinates | Network drag/replacement | Example navigation |
|---|---|---|---|---|
| Firefox 150.0.2 | Passed | Passed | Passed | Passed |
| WebKit 26.0 | Passed | Passed | Passed | Passed |
| Microsoft Edge 157.0.4341.46 | Passed | Passed | Passed | Passed |

| Browser | Pie styles | Radar styles | Sankey styles | Line styles |
|---|---|---|---|---|
| Firefox 150.0.2 | Passed | Passed | Passed | Passed |
| WebKit 26.0 | Passed | Passed | Passed | Passed |
| Microsoft Edge 157.0.4341.46 | Passed | Passed | Passed | Passed |

| Browser | Data colors / legends | HeatMap | Bar styles | Stacked RadialBar | SVG instance isolation |
|---|---|---|---|---|---|
| Firefox 150.0.2 | Passed | Passed | Passed | Passed | Passed |
| WebKit 26.0 | Passed | Passed | Passed | Passed | Passed |
| Microsoft Edge 157.0.4341.46 | Passed | Passed | Passed | Passed | Passed |

| Browser | AreaBump | Hierarchy focus | Compact Calendar | BoxPlot / Bullet |
|---|---|---|---|---|
| Firefox 150.0.2 | Passed | Passed | Passed | Passed |
| WebKit 26.0 | Passed | Passed | Passed | Passed |
| Microsoft Edge 157.0.4341.46 | Passed | Passed | Passed | Passed |

All 81 suite runs passed without increasing the existing pixel thresholds.
The final complete run was 2026-11-10 10:29:52.439–10:35:47.079 UTC; it reran
all twenty-seven suites in all three engines after the measurement corrections below,
rather than combining earlier passing results. Each gallery result contains 140
scenes. The configured-example suite checks thirty-one actual style entry points, Change styles
through every phase, the five-mode HeatMap selector, same-mode remount and
cleanup on switching.
SVG image comparisons now preserve the live SVG's inherited language; removing
that context changed generic font fallback in Firefox and bundled Chromium 141.
Semantic dash opacity is compared exactly with an independent CSS group-opacity
control in the same engine. Firefox/WebKit produce `[255,126,126,255]`, whereas
Chrome/Edge produce `[255,127,127,255]`; stroke and overlapping zero-dash caps must
match that independent control channel for channel. Gap and oriented square-cap
pixel assertions, reactive updates, refusal and cleanup remain required.

The Network suite independently records trusted PointerEvent client coordinates
before the wrapper's listeners and checks the inverse world-space delta using
the original 0.01/0.02 tolerances. Firefox's actual device-coordinate quantization
differs from the requested Playwright mouse position; no implementation or
tolerance is changed to hide it. It also verifies rapid Up/Down ordering,
capture/lost capture, hidden-container suspension and same-size wake, current
topology/labels/selection, replacement refusal, late force failure, Error-callback
reentry, idle RAF, Stop/Drop and restoration of the DOM baseline.

The runner is [cross-browser-acceptance.mjs](../scripts/cross-browser-acceptance.mjs).
Its machine-readable output from this run is
`target/gallery/cross-browser-report.json`, containing each original verifier's
full result, actual browser version, executable selection and launch failures,
if any. Output is an artifact, not an embedded source fixture.

## Engines and launch paths

Playwright's installed Node package was **1.56.1**. Its expected Firefox 1495 and
WebKit 2215 bundles were absent. The runner used these existing newer bundles
through explicit `executablePath`, successfully exercising the browser protocol:

- Firefox: `C:\Users\chris\AppData\Local\ms-playwright\firefox-1522\firefox\firefox.exe`.
- WebKit: `C:\Users\chris\AppData\Local\ms-playwright\webkit-2311\Playwright.exe`.
- Edge: Playwright's system `msedge` channel; the installed executable was
  `C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe`, whose product
  version matched the version above.

The directory revision is not the browser version. A successful run with newer
bundles does not establish compatibility with every Playwright protocol version.
The runner records `usesDriverExpectedRevision: false` for these two engines.
No browser download, installation, cache relocation or environment rewrite was
needed. The runner first tries the driver's expected executable, then existing
bundles in the normal Playwright cache. Explicit `--firefox-path` and
`--webkit-path` arguments can pin an installed executable.

## Assertions and observed pixels

All twenty-seven verifiers are reused; there is no reduced cross-engine version
of their assertions. Each suite runs in a separate browser context at a
1440×1000 viewport and device scale factor 2. The gallery also tests its 390px
mobile layout and a real Canvas mouse hover.

The gallery compares rasterized SVG with Canvas **within each engine**, rather
than comparing screenshots from different engines. It checks all 140 scenes,
actual colored pixels, SVG/Canvas toggles, clipping, upgraded path geometry and
responsive layout. Its existing mean absolute RGBA-channel difference limit
remains **3.5** on the 0–255 channel scale. No font or antialiasing tolerance was
changed for this run.

| Measurement | Firefox | WebKit | Edge |
|---|---:|---:|---:|
| Mean of gallery SVG/Canvas mean channel differences | 0.123400 | 0.026937 | 0.037014 |
| Largest gallery SVG/Canvas mean channel difference | 0.604419 | 0.310942 | 0.339269 |
| Smallest gallery colored-pixel count | 6618 | 6628 | 6694 |
| Semantic outline dark pixels | 138 | 138 | 128 |
| Initial gradient SVG/Canvas mean channel difference | 0 | 0 | 0 |

The new chart-specific suites use their original, tighter limits: Pie **0.75**,
Radar/Sankey/Line and Colors/HeatMap/Bar/Stacked RadialBar and the four P1 suites **0.5**
on the same 0–255 channel scale. These limits are
separate from the gallery's 3.5 limit. Maxima include initial, updated and
hidden frames tested by the corresponding verifier:

| Largest mean channel difference | Firefox | WebKit | Edge |
|---|---:|---:|---:|
| Pie | 0.011152 | 0 | 0.020144 |
| Radar | 0.040625 | 0 | 0.022211 |
| Sankey | 0.099314 | 0.278716 | 0.056486 |
| Styled line | 0.122359 | 0.389667 | 0.066024 |
| Data colors / legends | 0.072662 | 0 | 0.022905 |
| HeatMap | 0.389747 | 0.354813 | 0.251786 |
| Bar styles | 0.034707 | 0 | 0.003022 |
| Stacked RadialBar | 0.027376 | 0.000295 | 0.067993 |
| AreaBump | 0.027090 | 0.000157 | 0.004415 |
| Hierarchy focus | 0.178521 | 0.196091 | 0.051971 |
| Compact Calendar | 0.063235 | 0.000648 | 0.098873 |
| BoxPlot / Bullet | 0.057243 | 0.032710 | 0.004187 |

Pie checks actual annular corner removal, holes, active radii, value/percent
labels, reverse winding, skip angles and nonselectable leaders. Radar checks
real periodic paths, grid/point shapes, independent fill/border alpha and
owned labels. Sankey checks horizontal/vertical DAG and circular layouts,
actual gradients, rounded nodes, widths/ports and source versus two-ended
visibility. Styled line checks area/line/point paint, missing-value connection,
original point IDs, transparent mesh sites and formatted labels. Every suite
also requires current pointer/keyboard interaction, keyed geometry retention,
zero DOM mutation for an unchanged frame, rejected-update atomicity, legend
ownership, definition retirement where applicable and Stop cleanup.

The P0 suites additionally check real Geo/Calendar color consumers and missing
swatches; nullable HeatMap cells with five exact relation sets and controlled
paint; raw/displayed bar labels and signed totals with actual hidden re-layout;
and ring-by-category angular stacks with public radial/circular axes, reverse
winding and full-turn seam deduplication. Shared ring labels/tracks survive
category filtering. Numeric legends and axes remain noninteractive decorations.

The four P1 suites use the actual typed layouts in both renderers. AreaBump
checks changing slice ranks/thickness, sparse Connect/Gap bands, current raw
endpoint data, controlled highlight and legend re-layout. Hierarchy checks
trusted pointer and keyboard activation into treemap/packing/icicle subtrees,
current ancestors, back/parent/root, deletion recovery, updated values and an
identity camera; this is data-space layout, not a camera transform. Calendar
checks compact/full-year, weekday and orientation changes, nullable dates,
real month outlines and inherited stroke-picking policy. BoxPlot/Bullet checks
sample summaries, current median/outlier scalar ownership, decorative whiskers,
row/subgroup re-layout, axis ownership and complete hidden recovery. All retain
keyed geometry, reject malformed updates without changing the accepted state,
and restore the DOM baseline at Stop.

The configured-example suite includes four hierarchy views whose native Enter
input automatically commits focus through a bounded event/RAF bridge; Back and
Root restore ancestors while the camera remains identity. It does not require a
permanent polling loop. The separate native packing regressions also recover
24 extreme profiles using parent-relative offsets, preserving whole-tree
padding; these numerical assertions are not represented as browser pixel tests.

The contour suite runs actual PropertyMotion RAF frames, retargets, applies
reduced motion, checks rejected mappings and drops an active controller. Both
SVG and Canvas retained exact final hole pixels `[255,255,255,255]` and ring
pixels `[30,90,160,255]` in every engine. It also checks two real contours in one
retained SVG mark and restoration of the exact cubic target.

The other suites exercise controlled legend changes, owned-label visibility,
mixed paint order, actual keyboard focus and selection, current-datum callbacks,
safe text, real shaped/rotated semantic glyph geometry, rejected-update atomicity,
keyed gradient definitions, actual PNG decoding and font ink, DPR redraw,
retirement and remount. Page errors fail each suite. This includes actual
cleanup assertions; it is not merely an initial-render smoke test.

Across these runs, four test-input or measurement portability issues were
corrected without weakening the rendering or pixel assertions:

1. The gallery's previous hover picked the first data circle at the left plot
   clip boundary. In Firefox, the dispatched CSS coordinate mapped to scene
   x=63.6279, outside clip x=64, so rejecting that hit was correct. The test now
   hovers the real interior `point-0-1` datum and still requires its North tooltip.
2. The contour test serialized a live SVG with percentage dimensions, then drew
   the decoded image at its implicit intrinsic size. Firefox decoded a
   340.4167×260 scene to a 300×229 image, moving the pixel probe away from the
   intended scene coordinate. The test now sets explicit logical dimensions on
   its clone and scales `drawImage` to that same viewport. Original exact RGB,
   winding, animation and cleanup assertions remain in force.

3. WebKit reported a height of `0.0000152587890625` for the bounding box of
   a zero-radius site whose Move and four Cubics are all at one identical point.
   The verifier now requires all 26 source coordinates to be exactly the same
   point, native `getTotalLength() === 0`, transparent fill and no stroke;
   the styled-line fixture also reports its exact Scene radius of zero.
   No bounding-box epsilon or pixel tolerance was added.
4. Firefox's styled-line test sent Escape while an external datum control,
   rather than the plot surface, held focus. That did not invoke the surface's
   Clear action and left selection highlighting active during the next check.
   It now sends `surface.press('Escape')`, executes the real Clear input, then
   checks the unchanged zero-site contract. The final 81-suite run includes
   both corrected gallery and styled-line assertions.

## Cross-instance SVG regression

The preceding P0 round fixed a real renderer defect, distinct from the test-input
corrections above. Two live BrowserChart SVGs using identical paint and mask
content shared five definition IDs. Hiding the first owner made the second
linear fill white `[255,255,255,255]` while Canvas stayed `[155,60,128,255]`.
The before-fix report and images are `target/gallery/svg-isolation-before-fix.json`,
`svg-isolation-before.png` and `svg-isolation-hidden.png`.

`render::to_svg_with_namespace` isolates generated resources for caller-owned
inline SVG exports. Retained SVG assigns a stable checked namespace per owner,
including gradient, pattern, mark clip and label clip definitions/references;
logical cache keys, raw datum IDs and user text remain unchanged. Native tests
also reject namespace injection/oversize values and preserve literal URL-like
user text. Independent `to_svg` remains deterministic for standalone documents.

The `svgIsolation` suite samples the **live composited SVG screenshot** and
Canvas in the same browser. Serializing the SVG as a separate image would hide
the document-wide resource lookup bug. Seven fixed interior points, clipped
text, first-owner hide/drop/stop/remount, local unique references, retirement,
retained path/text identity, zero unchanged mutations, input, invalid-update
atomicity and final cleanup all pass in all three engines. The largest checked
absolute RGBA-channel differences are **1 / 0 / 1** for Firefox/WebKit/Edge,
under the unchanged **±2** bound; text-clip exterior ink is zero.

Firefox's first radial-center comparison recorded a difference of three levels
at the distance function's cusp. The assertion now uses a smooth interior point
20 logical pixels from the center and 30 from the edge, while retaining the
original center RGBA, fractional surface/SVG CSS bounds and relative device
indices in `radialCenterDiagnostic`. No tolerance was enlarged. This records
live compositing/device-sampling limits rather than claiming every radial
pixel is exactly equal. The final complete 81-suite report uses this same
verifier; the latest Chrome host combination was rerun with it as well.

## Reproduction

Build the gallery and the `browser-widget`, `browser-host`, and `browser-raster`
consumers using their normal workspace configuration, then serve their outputs.
The default URLs are gallery `8811`, Widget `8813`, host `8814`, and raster
`8815`. Each consumer's README/build script describes its artifact directory.
With those servers running, execute from the kartis workspace:

```powershell
$target = (cargo metadata --no-deps --format-version 1 | ConvertFrom-Json).target_directory
node scripts/cross-browser-acceptance.mjs --browsers firefox,webkit,edge --output (Join-Path $target 'gallery/cross-browser-report.json')
```

The report directory must already exist. Use `--gallery`, `--widget`, `--host`
and `--raster` to supply other server URLs. An unavailable requested engine or
any failed assertion produces a failed report and nonzero exit status; it is
not silently skipped. This script does not build or install dependencies.

## Scope limits

- Playwright WebKit on Windows is not an installed Safari browser, iOS or macOS
  validation. Edge is another Chromium browser; Firefox and WebKit provide the
  independent engines here.
- Chrome CDP `Input.dispatchTouchEvent`, CDP-only screenshots/layer metrics and
  the combined host suite's CDP touch cases are not called by this runner.
  Their Chrome evidence remains separate; this is no cross-engine physical
  touch, OS input, native window or accessibility-tree certification.
- That historical run tests the listed twenty-seven suites. It does not claim all host viewport,
  navigation, layout, pinch, high-density or motion configurations on all
  browsers. No process RSS, cross-browser benchmark or 60 FPS claim is made.

## Development Funnel and Parallel filtering

Unreleased development APIs, not part of the published Kartis 0.1.0 package.

ParallelFilterController provides transactional raw-value AND filters and actual
scale inversion for pixel/Brush intervals. It renders from the full source,
preserving domain/palette/group legend/source indices while removing rejected
row ink, owned labels and interaction sites. This is a Rust extension that can
connect to D3 Brush, not an existing Nivo Parallel filter prop. Automatic live
host gesture wiring and categorical dimensions remain outside this acceptance.

Funnel stage factories replace one bounded body and explicitly preserve or
remove its original open-sided border. Exact picking follows custom ink;
Nearest keeps the original center. Default gradient inheritance preserves the
complete layout. Independent audit reproduced and fixed both default metadata
drift and a cumulative-budget false rejection; the final 16,384 gradient stops
and 200,000 command boundaries remain admitted with no raised limits.

Native acceptance: 1,440 minimal tests, 1,707 all-feature tests and 17 doctests;
native all-target Clippy, Rust 1.95 WASM Clippy and pinned nightly formatting
passed. Records: `target/development-funnel-parallel-validation/`.

The executable `funnel_custom` exports four directional/active frames, SVG,
Canvas scripts and CPU PNG. Chrome, Firefox, WebKit and Edge at DPR 1/2 compare
48 SVG-image/Canvas samples with CPU RGBA per context: 8 contexts and 384
comparisons. These samples verify the tested offset geometry and controlled
active expansion; they are not full-frame equality or retained-host/device
acceptance. Final artifacts: `target/funnel-custom-sealed/`. The first browser
run failed for a missing pinned Firefox executable; its log is retained, the
pinned browser installation completed, and the complete cohort was rerun.

Guides: [Funnel](funnel-custom.md), [Parallel filtering](parallel-filtering.md).
Arbitrary React/Widget components, automatic host Brush mounting, GPU/worker
and the remaining broader gap list are not closed by these results.

## Development measured text wrapping

Unreleased Rust extension, not a Nivo automatic-wrap prop and not included in
crates.io 0.1.0. Full-candidate real measurements drive word/grapheme breaking;
Scene replacement atomically remaps typography, ownership, clips and draw order.
True FontRegistry advances/ink and font-defined line metrics supply the CPU
wrapper. The primary face's line metrics are not HTML/CSS line-box metrics.

Core 6, independent audit 6 and raster 6 tests cover combining/emoji graphemes,
CRLF, ellipsis, non-monotonic advance, rotation, stale measurements, budgets,
family/weight, empty registry rejection and transactional metadata. The audit
reproduced and fixed discarded trailing-space mismeasurement and moved repeated
clip-cost admission before copying. Raster-only 5 tests also passed.

Complete regression: 1,452 minimal and 1,725 all-feature tests, 17 doctests;
native all-target Clippy, Rust 1.95 WASM Clippy and fixed-nightly format checks.
The first native Clippy run caught a new test's chunks_exact style; the corrected
test and strict Clippy were rerun, preserving the failed log.

The executable `text_wrapping` exports ordinary editable labels and separately
outlined text plus a CPU PNG. Four browsers at DPR1/2 (8 contexts) verify all
11 line strings, native column fit and SVG-path/Canvas-path ink bounding boxes
against CPU (within the declared 1.5 logical-pixel bound). This does not compare
entire RGBA images or establish ordinary SVG/Canvas text pixel equivalence.
The original exact-advance probe (0.1 px) failed on Firefox: maximum difference
0.38590502929687887 px. That failure is retained at
`target/text-wrapping-browser-first.log`; its cause was not isolated. The later
profile report separately records native column fit, advance differences and
outline placement; it does not relabel that exact-advance criterion as passed.
Final exports/reports: `target/text-wrapping-sealed/`.

The initial wrapping acceptance preceded an explicit host whitespace policy.
The subsequent development acceptance below supersedes that whitespace limitation;
it does not resolve the retained Firefox exact-advance failure. Full CSS/UAX14,
paragraph-wide bidi followed by breaking, automatic vertical wrapping and
automatic host reflow remain open gaps. See [guide](text-wrapping.md).

## Development explicit whitespace acceptance (2026-11-14)

`LabelWhitespace::Legacy` preserves existing behavior. Explicit `Preserve`
normalizes single-line control separators and retains repeated/leading/trailing
spaces in standalone SVG, retained SVG, browser Canvas, CPU raster, and actual
Glory ordinary Text. Scene wrapping marks every generated line Preserve.
Measurement identity includes the mode; mode-only changes request fresh metrics.
Filtering, painted-label conversion, motion, and managed-label placement preserve
or remap the sparse policy with the labels.

Complete regression passed: 1,468 minimal tests, 1,742 all-feature tests and
17 doctests; native all-target strict Clippy and Rust 1.95 WASM strict Clippy.
The public `SceneTypography` and `SemanticTextRequest` fields require updates to
complete struct literals. This remains Unreleased and is not in published 0.1.0.

The actual browser-widget fixture runs Legacy/Preserve/Legacy/Preserve with
identical source text in Chrome, Firefox, WebKit and Edge at DPR1/2: 8 contexts,
32 phases, unchanged retained text-node identity, restored Legacy styling,
fresh mode-dependent measurements, and real Glory Text content/styles.
Preserve SVG/HTML advance and DOM Range glyph widths agree within the declared
1 logical-pixel bound. Sixteen Preserve phases compare actual Canvas ink bounds
with serialized SVG-image ink bounds within 1.5 logical pixels. This is not
whole-image RGBA equality, arbitrary-font/device certification, or a solution to
the earlier CPU/native Firefox advance difference.

Reports: `target/whitespace-browser-after-clippy.json` and
`target/whitespace-final-validation/acceptance.json`. Initial fixture setup
failures are retained: missing bootstrap mount root, an incorrectly hidden
measurement host, and default Canvas selection where the retained-SVG fixture
required explicit SVG. The final fixture uses a normally sized offscreen
bootstrap host and explicit renderer selection; thresholds were not relaxed.
A later fixture-only strict Clippy check rejected a nested conditional; its
first failure is retained, the conditional was corrected, and the actual WASM
fixture and all 8 browser contexts were rebuilt and rerun successfully.
See the [policy guide](text-whitespace.md) and [中文指南](text-whitespace.zh-CN.md).


## Development Waffle cells and Calendar year labels

Unreleased additions, not included in the published 0.1.0 package. The borrowed
Waffle cell factory preserves physical slot identity and default whole-layout
output, styles real empty cells without creating typed data sites, and separates
custom ink picking from the original logical slot. In Areas mode only empty
cell marks invoke it; retained solid group unions count toward the final budget.
Seven core tests and five independent audit tests cover identity, traversal,
paint/site/source-color separation, dashed ink, extreme Rect round-off, animated
Path anchors, rejection recovery, and exact 200,000-command/16,384-stop budgets.

The Calendar entry composes FullYears year-label Before/After placement, signed
finite offsets and a borrowed text factory with existing day colors. Old entries,
panel allocation, dates/months and Scene.legend remain unchanged. Compact rejects
the new entry. Six minimal/seven raster tests cover negative offsets, rotation,
month spacing, text budgets, empty output and actual Lato raster ink.

Full regression passed 1,486 minimal tests, 1,761 all-feature tests and 17
doctests; Rust 1.95 WASM strict Clippy passed. Native strict Clippy first caught
new test-expression issues (fixed-size chunks and cloned singleton slices).
Those first failures and one failed test-expression migration compile are kept.
After correction, the affected Calendar 7 and Waffle audit 5 tests and native
all-target strict Clippy passed. Production library code did not change during
these test-only corrections.

Four engines (Chrome, Firefox, WebKit, Edge), each at DPR1/2, passed these fresh
export profiles:

- Waffle: 16 frames cover four directions, Cells/Areas and a hidden group.
  Every slot's logical origin and offset ink position is queried. Each context
  compares 384 SVG-image/Canvas samples against CPU RGBA within 1 channel unit;
  3,072 comparisons total. These samples are not whole-image equality.
- Calendar: 24 frames cover two orientations, Before/After, offsets -12/0/20 and
  month spacing 0/3, with two years per frame. The exact pinned original Nivo
  computeYearLegendPositions function independently verifies 48 placement
  anchors/rotations. The existing rotated size/3 alphabetic shift is removed
  only for this anchor comparison: Nivo Canvas uses middle baseline, which is
  not claimed equivalent. Supplied-font outlined SVG-image/Canvas ink bounding
  boxes agree with CPU within 1.5 logical pixels (48 comparisons/context, 384
  total). Ordinary editable chart SVGs are exported separately; outline checks
  do not prove ordinary native-font pixels or full Nivo layout equality.

Both scripts also support the bundled Chromium-only CI profile at DPR1/2;
the gallery workflow now generates these examples and retains their reports and
exports. The two-context CI profile is distinct from the eight-context manual
matrix. Guides: [Waffle](waffle-custom.md), [Calendar](calendar-year-legend.md).
Fresh reports live under `target/waffle-custom-v2/` and
`target/calendar-year-legend-v2/`; aggregate acceptance is
`target/waffle-calendar-final-validation/acceptance.json`. Initial 16-frame
Calendar results are retained separately and not reused for the expanded matrix.
GPU/worker, arbitrary components, broader platform input and other open gaps
remain outside this acceptance.


## Development Bullet part factories

Unreleased API, not included in the published 0.1.0 package. One borrowed typed
factory handles actual Range/Measure/Marker marks, including synthetic domain
ranges, while preserving raw intervals, source indices, row domains and track
clips. Rank includes omitted endpoints; source-row and visible-row indices stay
separate. Metadata geometry is updated after callbacks so every callback sees
original row snapshots. Default inheritance preserves the entire inner layout.

Ten minimal/eleven all-feature core tests and five independent audit tests cover
source/order identity, omitted and synthetic parts, actual offset ink versus
original logical centers, scale composition, track clipping, paint/geometry
rejection, late callback errors and exact 200,000-command/16,384-stop budgets.
Inspection found a transparent-fill/visible-stroke representative-color issue;
identity and custom ink now share the existing visibility-aware paint rule,
without rewriting original identity marks. A regression verifies the red stroke
site color and unchanged original transparent-fill layout.

Actual Glory Memory mounting initially rejected the legacy interaction binding.
The retained failed logs identify the initial host rejection; source inspection
confirmed that chart_layout requires explicit PaintedGeometry. The new
into_painted_scene_interaction conversion preflights that solid-ink profile.
The ordinary conversion retains Legacy Geometry/Nearest behavior. Tests mount a
real strict Glory chart and verify that strict Nearest and eligible gradient
fills reject, while legacy gradient SVG/Canvas exports remain valid. These
profiles are distinct; semantic Graphic/Text limits still apply.

Full regression passed: 1,501 minimal tests, 1,777 all-feature tests, 17 doctests,
native all-target strict Clippy and Rust 1.95 WASM strict Clippy. Formatting,
hidden-file spelling and diff checks passed. Initial development failures remain
retained: a new test counter's borrow lifetime, the initial host profile mismatch,
and a test-only incorrect FillPaint variant spelling. The full final regression
ran after all corrections, rather than reusing pre-correction suite results.

The executable bullet_custom checks 16 profiles: horizontal/vertical, reverse,
legacy/Linear/SymLog/Log, shared/per-row domains and a hidden row. Every profile
mounts its full Scene in real Glory Memory, verifies semantic Graphic and typed
control counts, and exercises Surface Exact/Nearest at custom ink and original
centers. There are 144 actual controls and callback invocations across 16 mounts.
Ink-only export clones remove ordinary labels before CPU/SVG/Canvas comparisons;
this does not test font rendering. Each of 8 browser contexts (Chrome, Firefox,
WebKit, Edge at DPR1/2) compares 432 SVG-image/Canvas samples with CPU RGBA within
1 channel unit, 3,456 comparisons total. Samples are not whole-image equivalence.
The separate bundled Chromium CI profile passed both DPRs (864 comparisons).

CI now generates and retains this export profile alongside Waffle and Calendar.
Original Nivo rangeComponent/measureComponent/markerComponent source contracts
were checked against commit 0dae2c32052a573f9f1d66ec1b453b572119b265; this is not a
React render oracle. Nivo removes zero endpoints and uses filtered sorted ranks;
Kartis's original zero/negative/source-index contract remains. Arbitrary React
components, animatedProps lifecycle, physical devices and broader parity remain
outside this acceptance. See [guide](bullet-custom.md) and [中文指南](bullet-custom.zh-CN.md).
Reports: target/bullet-custom-final/browser-report.json and
 target/bullet-custom-final-validation/acceptance.json.


## Development Bump endpoint text

Unreleased addition, absent from published 0.1.0. Independent Start/End text
factories compose with the existing point factory through BumpFactories. The
source series/label, legend, tooltip, geometry, style and logical identity stay
unchanged; only actual owned endpoint Label text is replaced. Default wrappers
preserve the complete layout. Contexts expose original source indices, completed
rows and both line and padded label positions. Source traversal remains explicit
with reversed X. Hidden/disabled/missing ends skip callbacks; empty and zero-ink
owned slots remain, with the shared 1,024-byte UTF-8 budget.

Core 6 and independent audit 6 tests passed in both minimal and all-feature
profiles, with the existing Bump 6 tests passing in both profiles. They cover
source borrowing, missing and nonmatching source order, reverse, actual Memory
Props::Text, owner/whitespace remapping, zero opacity/point size, callback order,
failure stops and byte boundaries. The audit's initial helper missed an Ok
wrapper required by the old point-factory signature; the compile failure was
retained and corrected. No production-source correction was required.

Full regression passed: 1,513 minimal tests, 1,789 all-feature tests and 17
doctests; native all-target strict Clippy and Rust 1.95 WASM strict Clippy.
Afterward, the export fixture was extended to execute ordinary Canvas text as
well as outlined Canvas; it was rebuilt and its example-specific strict Clippy
passed. Production library/tests did not change during this fixture extension.
Formatting, hidden-file spelling and diff checks passed.

The real-font fixture exercises 16 reverse/hidden/start/end/disabled profiles,
combined rectangular point geometry, missing endpoints and an empty start label.
It mounts the full Scene in 16 actual Glory Memory apps and checks 24 ordinary
Text nodes, including eight empty slots, alongside 88 point and 24 endpoint
callback invocations. Text-only export clones retain the accepted label data.
Lato font bytes are explicitly bound for CPU and loaded in the real browser.

Chrome, Firefox, WebKit and Edge at DPR1/2 passed eight contexts. Each checks all
24 editable SVG labels/anchors and all 24 actual ordinary Canvas fillText calls
(text, transformed anchor, alignment, alphabetic baseline and requested font),
including eight empty calls. Native Canvas blank/nonblank states also match the
expected six blank frames. No native-text advance or pixel equality is claimed.
Separately, outlined SVG-image and Canvas bounds match actual CPU text ink within
1.5 logical pixels: 32 checks/context, 256 total, including exact blank-state
checks. Whole-image RGBA and ordinary native-font pixel equivalence remain outside
this result. Bundled Chromium at both DPRs also passed the separate CI profile.

The original Nivo startLabel/endLabel source contract was checked at commit
0dae2c32052a573f9f1d66ec1b453b572119b265. Its position-separated hooks call the
accessor before excluding missing endpoints; Rust's actual-label, series-first
point→Start→End order differs deliberately. This is not a React execution or
rendering oracle. Automatic label collision, arbitrary component lifecycle,
physical-device acceptance and the broader gap list remain open.

Guides: [English](bump-endpoint-labels.md), [中文](bump-endpoint-labels.zh-CN.md).
Fresh exports and reports: target/bump-endpoint-labels-v2/;
aggregate receipt: target/bump-endpoint-final-validation/acceptance.json.
The first outline-only fixture profile remains separately retained. CI now
creates the extended profile and uploads its exports/reports; its bundled
Chromium profile was exercised directly, without claiming a new remote CI run.


## Development Albers USA primitives and planar rectangle postclip

**Foundation-stage acceptance only; full Albers USA integration remains open.**
The Unreleased `GeoAlbersUsaProjection` point/inverse kernel and planar rectangle
postclip APIs are implemented. They do not yet supply an Albers USA option to
`GeoMapOptions`, fitted polygon maps, geographic line/graticule streams,
GeoSymbol/glyph/label consumers or geographic host-coordinate adapters. This
record does not close that feature gap or establish a complete USA map renderer.
Published 0.1.0 does not contain these additions.

The pinned original d3-geo 3.1.1 reference covers 9 scale/translation states,
5,778 forward source-point records, 792 inverse records and 414 region-specific
inverse records, including each child's window and visibility. The final kernel
suite passed 6 minimal/7 all-feature tests. Forward clipping uses the original
1e-6-final-pixel inset and exact window comparisons; no forward clip tolerance
was added. Official serde_json float_roundtrip preserves the source binary64
values; no custom JSON lexer remains.

Inverse admission changed explicitly during development. The final contract
checks the requested pixel's exact selected-region rectangle, checked inverse
admission, the native child representative and an unclipped same-region residual.
A recovered source may round one ULP outside strict forward visibility at an
exact edge; forward point-stream visibility is therefore not a second inverse
domain check. The original D3 inverse remains an unrestricted algebraic result;
Rust's input-domain admission is a derived policy, not a claim that arbitrary
far-away native inverse pixels are accepted. Schema 2 revises 44 derived strict
expectations from None to a finite inverse whose candidate forward stream is
not visible. Native forward/native inverse/region-inverse records, scales,
translations and clip windows remain unchanged against the retained v1 data.
The policy change is recorded in target/geo-usa-inverse-policy-change.json;
target/geo-usa-kernel-v1-reference/ and the failed intermediate kernel logs are
retained. Old-policy failures are not relabelled as passes of that old contract.

The rectangle clipper uses native stream buffering, planar line clipping,
boundary ordering and contour rejoining. It preserves native winding, closing
duplicates and degenerate contacts, without assigning exterior/hole roles from
list position. Seven minimal/eight all-feature tests cover concave splitting,
holes through the window, whole-window coverage/removal, independent reentry,
coincident edges, point contacts, finite arithmetic and resource limits. Its
383-case original D3 differential includes the initial 63 cases plus 128 simple
star-shaped rings, 128 folding open lines and 64 two-hole profiles. Contour and
point counts/order agree, with each coordinate difference below 1e-8. The initial
63-case reference and acceptance remain separately retained; expanding the
reference required no production or comparison-threshold changes.

An independent audit reproduced a work-budget bypass: empty input line headers
previously consumed no work. The failing test is retained at
target/geo-rect-clip-empty-header-before.log. Per-line/per-ring and winding-ring
headers now consume work, while point-count semantics remain unchanged. Final
native output points are bounded by max_points, intermediate buffering by twice
that allowance, and scanning/copy/sort/traversal by max_work. Finite crossing
endpoints follow D3's +/-1e9 numerical clamp; unrepresentable polygon winding
arithmetic returns an error. Expanded acceptance is recorded in
target/geo-rect-clip-expanded-acceptance.json.

The final combined regression passed 1,526 minimal tests, 1,804 all-feature
tests and 17 doctests. All five commands in
 target/geo-usa-primitives-final-validation/state.json exited zero, including
native all-target strict Clippy and Rust 1.95 WASM strict Clippy. The focused kernel and rectangle suites also passed their owned-source
formatting checks.

The executable geo_usa_primitives exports 10 diagnostic frames: six polygon
clip cases, one open-line reentry case and three manually authored regional
circle/rectangle-clip cases. Its 37 authored probes check Scene hit results and
CPU samples. Chrome, Firefox, WebKit and Edge at DPR1/2 passed 8 contexts, each
comparing 74 actual SVG-image/Canvas samples against CPU RGBA within one channel
unit: 592 comparisons total. A distinct bundled Chromium profile passed 2
contexts and 148 comparisons. These are sample checks, not whole-image equality.
The three regional circles are manual Mark.clip diagnostics using kernel
roundtrip coordinates; they are **not GeoSymbol/glyph/label pipeline acceptance**.
There is no ordinary text, physical-device or integrated geographic host-input
claim in this browser result.

Fresh browser reports and exports: target/geo-usa-primitives-final/.
Full regression logs: target/geo-usa-primitives-final-validation/.
See [English integration guide](geo-albers-usa.md) and
[中文说明](geo-albers-usa.zh-CN.md) for the remaining consumers and completion
conditions. GPU/worker and the broader feature-gap list remain deferred/open.


## Development Albers USA fitted maps and selected-region masks

The separate `geo_usa_maps` export tests actual dedicated World/Data map entries
and retained-context circle/glyph overlays: 20 frames, 168 probes, 336 actual
SVG-image/Canvas versus CPU comparisons per context. Chrome, Firefox, WebKit and
Edge each pass DPR1/2: eight contexts and 2,688 comparisons, within one RGBA
channel unit. A distinct bundled Chromium CI profile passes two contexts and
672 comparisons. Reports and exported SVG/PNG/Canvas HTML are retained under
`target/geo-usa-maps-v5/`. Scene hit assertions agree with the authored probes.

Each inset uses its own rectangle, including full original strokes; the broad
Lower48 rectangle is not used as a union substitute. Final probes have
12-pixel fill-interior and four-pixel clip-edge clearance. Initial fixed-radius
Data-fit fixtures lacked visible spill, and earlier fill/stroke-edge probes
showed a two-channel cross-engine difference. Those first failures and exports
remain retained; no comparison threshold was increased and no earlier edge
result was relabeled. Final sample count is explicitly 168 rather than the
earlier 180. These checks establish sampled map/glyph interiors and region
isolation, not whole-image edge equality, native browser editable text, new
browser-host inverse runtime, or physical devices. Native CPU text clipping and
Glory typed-host tests are separately described in
[the combined verification](verification.md#development-albers-usa-fitted-maps-and-region-overlays).

Original D3 multiplexed polygon/fit fixtures independently pass 32,890 coverage
queries (22 explicitly retained fixed-band exclusions), 192 projection queries,
fit bounds, contour signs and areas. This differs from the earlier manual
primitive-circle diagnostic and is recorded as its own acceptance batch. USA
component lines/graticules and their canonical ownership remain open; the
complete Albers USA feature is not marked done. The Chromium export step is in
the CI definition and was run locally; no remote CI run is claimed.


The accompanying full regression passes 1,537 minimal tests, 1,821 all-feature
tests and 17 doctests, with strict native/MSRV WASM Clippy, pinned formatting,
typos and diff checks. Its logs and scoped acceptance are in
`target/geo-usa-maps-final-validation/`.


## Development Albers USA streams and browser hosts

The fresh native-rectangle line/graticule export passes 37 frames/240 probes in
Chrome, Firefox, WebKit and Edge at DPR1/2: eight contexts, 480 actual SVG/Canvas
versus CPU samples each, 3,840 comparisons total. A distinct bundled Chromium
profile passes another 960 comparisons. Solid/dashed paint and all three caps
keep their original ink under independent region/plot rectangles. Full physical
picking supplies expectations; probes exclude other strokes' AA fringes as well
as clip edges. Earlier legacy-cap and crossing-AA failures are retained, not
reclassified. Source/exports/reports: target/geo-usa-streams-native-rect/.

The actual WASM host suite also passes eight contexts. Across them it verifies
224 successful and 64 None native-pointer queries, actual paint bounds, region
provenance and original D3 default/child inverses at 1e-7 for received event
coordinates; requested pointer quantization is recorded separately. It covers
trusted wheel/pan and keyboard, CSS scaling, stop/release, and 96 physical line
parts sharing canonical controls through nullable/42 updates and rejected
layouts. Thirty-two native editable-label checks inspect real SVG text, actual
Canvas fillText calls and magenta pixels before/after regional clipping. No
outlined surrogate establishes this claim. Reports are in
 target/geo-usa-host-final/report.json and, for the separate Chromium CI profile,
 target/geo-usa-host-ci-profile-final/report.json.

Default mask profiles also mount actual Glory Memory and native Raster hosts;
no clip_projection:false reduction substitutes for the default. The audit
fixed unsupported generic rectangle masks, duplicate values, nullable tooltip
fallback to live scalars, and metadata-only invalidation. The earlier 20 map
frames were rebuilt and pass all 2,688 comparisons with native rectangles.
Original D3 line/graticule geometry separately passes 6,520 bidirectional and
18,002 coverage probes, with three recorded band exclusions, interleaved native
events and tiny corner-contact differences explicitly retained. See
[the combined record](verification.md#development-albers-usa-streams-and-host-integration)
for contract details, source inventories and first failures. This is bounded
profile acceptance, not exact native degenerate-contact/edge-image equivalence,
physical-device testing, or completion of the broader Nivo gap list.

The accompanying regression passes 1,549 minimal tests, 1,838 all-feature tests
and 17 doctests; strict native/MSRV WASM Clippy, browser-host MSRV Clippy, pinned
formatting, typos and diff checks pass. CI includes both focused Chromium gates,
executed locally; no remote CI execution is claimed. New acceptance and source
hashes are in target/geo-usa-streams-final-validation/.


## Development Scene-order Widget composition

Chrome, Firefox, WebKit and Edge at DPR1/2 verify actual interleaved SVG/Canvas
and Glory Widget output: 256 screenshots, 320 native input/hover checks,
128 rejection records and 80 lifecycle records. The exact contract, independent
native color controls, received-event coordinate checks and retained first
failures are described in [the verification record](verification.md#development-scene-order-live-widgets).
The final script/WASM/glue hashes are checked unchanged during each run.
This is bounded browser acceptance using Canvas-alpha hit proxies, including
for SVG; it does not establish native same-layer UI input or exact SVG subpixel
coverage. GPU/worker remains deferred.


## Development HeatMap and Waffle cell Widgets

Both role suites pass Chrome, Firefox, WebKit and Edge at DPR1/2 on one
unchanged WASM/glue artifact. HeatMap verifies 208 frames, actual nullable
metadata/tooltip behavior, 656 DOM placements and 96 rejected candidates;
Waffle verifies 208 frames, 4,896 RGBA samples, real empty controls and
112 rejected candidates. The wider original Chrome host CLI also passes its
original strict piecewise screen gates. Historical matrix failures remain
separate, and the complete catalog has not been reclassified for other engines.
[Scope, raw evidence and limitations](verification.md#development-cell-widgets-and-complete-chrome-host-cli).

## Development CPU image Chart input

The actual Glory web Image/View input suite passes Chrome, Firefox, WebKit and
Edge at DPR1/2 on one unchanged artifact. Its 72 accepted frames and 120 trusted
pointer groups cover absolute image cameras, source aliases, nullable controls,
real Widget state, nonuniform viewport scaling and atomic PNG publication
rollback. All 72 received-coordinate inverse checks have zero error. The
Chromium gate is included in CI; four injected failures verify that the suite
rejects incorrect pixels, nullable values, identities and publication behavior.
Earlier aborted attempts remain separate. This does not establish native OS
input or rerun the complete SVG/Canvas catalog.
[Evidence and harness diagnostics](verification.md#development-cpu-image-chart-bindings).

## Development Stream dot components

Chrome, Firefox, WebKit and Edge at DPR1/2 pass the scoped dot-component suite
on one artifact: 208 frames, 64 trusted input groups and 112 complete candidate
rollbacks. The 1,248 gated Widget/source-interior/restored-dot RGBA samples
retain the channel limit of one. Sixteen disabled zero-height area-boundary
probes are separate diagnostics, including one unresolved strict Firefox DPR2
AA mismatch; this is not full source-edge pixel equivalence. Frozen original
Nivo function replay and the Chromium component gate are part of CI.
[Scope and evidence](verification.md#development-stream-dot-components-and-bound-chart-anchors).


## Waffle group regions: current focused matrix

Chrome, Firefox, WebKit and Edge passed DPR 1/2 for both SVG and Canvas in
`scripts/waffle-regions-acceptance.mjs`: 128 frames, 160 trusted input groups,
64 rejected candidates and 400 interior RGBA samples (maximum channel error 0,
allowed ±1). The independent CI subset repeats Chromium at both DPRs with the
same driver/helper/WASM/glue fingerprints.

The matrix distinguishes full-slot business selection from actual painted hits,
checks fresh per-click activation, exact keyboard group identity, hidden groups,
transparent cells, physical aliases, custom offset Path interiors and raw
state/DOM/focus/PNG rollback. It does not assert React lifecycle, OS transport,
Nivo shared-edge equivalence, arbitrary Widget export or the whole catalog.

Firefox blank-page input controls reproduce native request quantization up to
0.817 CSS px. The driver admits at most 1 CSS px for that screen delivery step,
then recomputes business membership from received coordinates. Actual physical
Scene positions retain a 1/32 logical-pixel gate; screenshot channels retain ±1.
This does not alter production picking or any prior failed raster-edge report.
See [verification](verification.md#development-waffle-occupied-group-event-regions)
and the [group area guide](waffle-regions.md).


## Scatter node components: current focused matrix

The current Scatter driver passes Chrome, Firefox, WebKit and Edge at DPR1/2:
176 SVG/Canvas frames, 48 trusted input groups, 112 rejected candidates and
10,416 RGBA samples (maximum channel error 1, allowed ±1). Real DOM button
bounds retain a separate 1/32 logical-pixel gate. Original typed XY and owned
series data, state/focus retention, legitimate revision rebuild, exact active
stop cleanup, source restoration and custom offset Path ink are checked.
The CI subset independently repeats Chromium at both DPRs with the same protected
driver/helper/WASM/glue. Four mutated-driver controls fail at intended gates.

The initial WebKit rollback witness failure remains archived. Its revision
increment reproduced without any candidate; corrected pointer settling and
screenshot-first observation pass the WebKit control and the new complete matrix.
The exact raw frame/DOM/focus/PNG comparison is retained. Screen request
quantization allowance (1 CSS px) is separate from DOM geometry and RGBA gates.
This result does not establish React/spring runtime equivalence, native OS input,
arbitrary Widget exports or whole-catalog compositor acceptance.
See [verification](verification.md#development-scatter-live-node-components) and
[the node component guide](scatter-widgets.md).


## Bar live components: focused solid-color matrix

Chrome, Firefox, WebKit and Edge pass DPR1/2 in SVG/Canvas: 240 paint frames,
48 label-metadata frames, 48 trusted input groups, 112 rejected candidates and
14,736 gated RGBA samples (maximum channel error0, allowed ±1). Actual Button
bounds retain a 1/32 logical-pixel gate. Source snapshots, original rectangles,
stack/percent intervals, zero/hidden/cropped roles, live label data and dimmed
input state, revisions, exact active stop cleanup and source-ink restoration are
checked. The independent Chromium CI subset uses the same protected artifact;
four mutated-driver controls fail at their intended data, pixel, DOM and full
rollback gates.

The fixture explicitly uses solid-color Buttons with font-size0 while preserving
DOM text for data/state checks. Original font8 text in a specific actual Canvas
compositor has an unaccepted two-unit green-channel diagnostic. Font8/0/8 toggles
98/100/98 with identical state/revision/bounds; a blank native Button does not
reproduce it. No strict text-equivalence or general font-error claim is made,
and the old failed glyph report remains failed. An earlier baseline-position
error was corrected through explicit display:block in the application fixture.
Production geometry/input and pixel/DOM gates were unchanged.
See [verification](verification.md#development-bar-live-component-roles) and
[the Bar component guide](bar-widgets.md). React/spring runtime, arbitrary Widget
exports, native OS transport and whole-catalog compositor acceptance remain
outside this result.

## Exact viewport screenshot sampling

The native-only sampling matrix passes Chrome, Firefox, WebKit and Edge at
DPR1/2: 48 frames, 80 exact hard RGBA samples, 24 independent native-marker
coordinate/index observations, 120 rejection controls and eight padded-element
crop counterexamples. Chromium separately passes the two-context CI subset.
Four mutant copies fail their intended index, scale, geometry-race and offscreen
checks. These results validate the new full-viewport sampler within its positive
axis-aligned contract; they do not reclassify the historical catalog reports.

The scrolled fixed-Canvas palette is explicitly ungated: Firefox fractional
scroll changes its native raster phase, while independently laid-out marker
indices and broad cyan CSS colors retain their hard gates. Fractional-origin
unscaled CSS rows at DPR2 are hard gates; other fine rows are diagnostic. No nearby expected pixel or wider tolerance is
used. The Bar font8 failure still reproduces with correct viewport sampling;
scriptless native DOM plus a transparent activated Canvas can reproduce it.
A separate diagnostic non-LCD native text profile matches the local glyph
region, without changing production flags or claiming a text fix.
See [the sampling guide](viewport-sampling.md) and
[the detailed evidence](verification.md#development-exact-viewport-sampling-and-native-text-diagnosis).

## Bump live point components: focused matrix

Chrome, Firefox, WebKit and Edge pass DPR1/2 in SVG/Canvas: 192 frames,
48 trusted input groups, 112 rejected candidates and 11,856 gated RGBA samples
(maximum channel error0, allowed ±1). An independent real-DOM geometry gate
remains 1/32 logical pixel. The new read-only viewport sampler maps actual
logical coordinates to physical PNG indices without fitting padded element crops.

Owned raw series/categories/ranks, original -0 bits, gap insertion, hidden
series, zero-size/transparent points, reverse scales, ordinary/offset Path paint,
source restoration and retained child state are checked. Legitimate revisions
rebuild children; stop cleans actual active children exactly once. Four mutated
drivers fail the intended color, independently checked data, real geometry and
full rejection-witness gates. The separate Chromium subset uses the same bound
package. This solid-color font0 fixture does not establish arbitrary text parity,
React/Spring runtime behavior, native OS input or full-catalog acceptance.
See [verification](verification.md#development-bump-live-point-components) and
[the guide](bump-widgets.md). New work remains Unreleased.

## Live hierarchy nodes: three-family matrix

Chrome, Firefox, WebKit and Edge pass DPR1/2 for Treemap, CirclePacking and
Icicle in separate fresh contexts: 24 cases, 624 SVG/Canvas frames, 144 trusted
input groups, 336 rejected candidates and 20,832 gated RGBA samples (maximum
channel error0, allowed ±1). Actual Button geometry retains a separate 1/32
logical-pixel gate. The full-viewport sampler maps actual DOM geometry and DPR
with one physical floor; all input remains native trusted browser input.

Owned full-tree raw weights/indices, computed leaf values, parent/focus metadata,
zero-to-positive insertion, hidden/focused roles, transparency, layout variants,
identity/state retention and exact stop cleanup are checked. Raw-bit-only -0/+0
updates also refresh actual components with identical source SVG/mark geometry.
The separate Chromium subset repeats all three families at both DPRs on the
same protected artifact. Four independently mutated drivers fail intended
color, raw-data, real-DOM and complete rejection-witness gates. Native parent
raw/aggregate roles and managed labels are covered separately by Rust/Memory;
the browser matrix primarily exercises leaf components with source labels off.

This font0 solid-color fixture does not establish arbitrary glyph equivalence,
React/Spring/D3 layout runtime parity, automatic sampled-motion components,
native OS input or whole-catalog acceptance. Nivo's zero/invisible component and
label-ownership rules remain distinct from Rust's actual-Mark model.
See [verification](verification.md#development-live-hierarchy-node-components)
and [the guide](hierarchy-widgets.md). New work remains Unreleased.

## Original-value updates: separate functional oracle

The new source-values driver checks exact original bits in actual retained role
children, source renderer eligibility, real DOM geometry, foreground colors and
complete accepted-state rollback. Rejection witnesses compare complete root DOM,
computed SVG styles, native Canvas backing bytes and viewport geometry exactly.
Business error reasons are checked; TypeError and WASM traps fail the run.

Whole-viewport PNG hashes remain diagnostic in this distinct driver. The earlier
strict whole-viewport run remains failed. Independent no-candidate and script-free
native carriers reproduced one changing SVG antialias pixel; the prescribed
integer alignment did not fix it. No arbitrary glyph or full-catalog acceptance
is inferred. See [the paired guide](widget-source-values.md) and
[verification](verification.md#development-original-values-in-retained-widgets).

Final results: four browsers × two DPRs × six roles pass 48 contexts, 480 frames,
96 trusted input groups, 480 rejected candidates and 4320 foreground samples.
The same-artifact Chromium subset passes 12 contexts, 120 frames and 1080 samples.
Five negative controls plus a separate deep-witness diagnostic reject intended
faults, including a hidden Canvas backing edit that leaves the full screenshot
unchanged. Two whole-viewport hash mismatches are retained as diagnostics in the
full matrix; none occur in the CI subset. This does not pass the old strict gate.

## Live Swarm nodes: packed source metadata and component matrix

Chrome/Firefox/WebKit/Edge pass DPR1/2 SVG/Canvas: 8 contexts, 272 frames,
48 trusted input groups, 112 rejected candidates, 10,800 foreground and
32 restored-source RGBA samples. Maximum channel error is zero (allowed ±1);
actual DOM geometry keeps its separate 1/32 logical-pixel gate. The same-artifact
Chromium subset passes 2 contexts and 68 frames. Independent PNG decoding
recomputes exact viewport/DPR indices for both reports.

Full source data, raw signed-zero changes, reorder/resize, empty/hidden/cull/skip/
zero/transparent roles, original packed anchors for offset Paths, revision
rebuilds, source picking and stop cleanup are covered. Five mutations and a
separate deep-witness variant reject independently forged data, real colors,
smaller actual Buttons and accepted updates disguised as errors. Hidden Canvas
backing corruption is detected even when all foreground/full PNG bytes agree.

State/DOM/SVG styles/Canvas backing/geometry rollback is exact; whole-viewport
PNG equality is diagnostic. Zero diagnostic mismatches here do not resolve
historical strict compositor/glyph failures. This font-zero fixture does not
establish arbitrary text, OS input, React/Spring/force or full-catalog parity.
See [verification](verification.md#development-live-swarm-node-components) and
[the guide](swarm-widgets.md). This work remains Unreleased.

## Live Network nodes: actual force/Scene/component matrix

The final single-cohort Chrome/Firefox/WebKit/Edge matrix passes DPR1/2 and four
ordered/top-layer, replace/keep profiles: 32 contexts,
1216 SVG/Canvas frames and 31,680 RGBA samples. The same-artifact
Chromium subset passes 8 contexts/304 frames/7,920 samples.
Independent Pillow readback checks all 1,520 saved frame PNGs.
Maximum channel error is 0 (allowed ±1); actual DOM and world mapping retain
the separate 1/32 logical-pixel gate.

Actual ticks, trusted node dragging, CSS scale/resize, graph replacement,
heat/revision and coherent retained owner/Bond snapshots are exercised. Accepted
state/DOM/SVG paint/Canvas backing/geometry rejection is exact, including active
drag and cold heat failure. Stop/cleanup and idle-hover termination are checked.
Six negative controls reject joint data forgery, wrong color, smaller actual DOM,
accepted updates disguised as errors and hidden one-pixel Canvas corruption.

The earlier thin-link RED stays recorded; this fixture explicitly authors 6px
for a solid interior link sample. Firefox pointer expectations use trusted
delivered endpoints and preserved grab offset, not the ideal requested delta;
an independent plain DOM control reproduces the delivered coordinates. Neither
the pixel allowance nor world/DOM geometry tolerance was widened.

Whole-viewport equality remains diagnostic, with
19 full-matrix and 0 CI mismatches.
No arbitrary glyph/OS input/force-React-Spring/full-catalog or historical strict
compositor acceptance is inferred. See
[verification](verification.md#development-live-network-node-components) and
[the guide](network-widgets.md). This work remains Unreleased.

## Live Network links and one-stage node/link composition

The final strict script passes Chrome/Firefox/WebKit/Edge at DPR1/2 with four
ordered/top and replace/keep profiles: 32 contexts, 832 SVG/Canvas
frames and 32,448 samples. Chromium CI adds 8 contexts,
208 frames and 8,112 samples. Independent Python/Pillow reconstructs
the authored source/Widget order for all 1,040 saved frame PNGs;
max channel difference is 0 (allowed ±1), with a separate 1/32
logical-pixel DOM/world gate. Source ink can cover link Widgets and block native
activation in ordered mode; exposed points and the top-layer profile are tested.

Complete current source bits, original midpoint versus offset Path bounds,
parallel/self/zero/transparent roles, live ticks, real dragging, resize,
replacement/index-local reorder and revision are checked. Full accepted state,
DOM, SVG styles, Canvas backing and geometry rollback are exact. Eight target
controls reject forged metadata, real paint/DOM changes, committed graph updates
disguised as errors, hidden backing corruption and native exceptions after stop.
The original node-only API separately passes 8 contexts/304 frames on the same WASM.

Whole-viewport hashes remain diagnostic: 29 full-matrix and
0 CI mismatches are preserved. No arbitrary glyph, OS input,
React/Spring/force or historical strict compositor acceptance is inferred.
See [verification](verification.md#development-live-network-link-and-mixed-node-components)
and [the guide](network-link-widgets.md). This work remains Unreleased.

## Literal Funnel SVG layers

The dedicated SVG owner passes Chrome, Edge, Chromium, Firefox and WebKit at
DPR1/2: 40 direction/interpolation profiles, 1,610 accepted frames, 560 rejected
transactions, 20 actual DOM paint/camera negative controls and 2,800 saved PNGs.
RGBA tolerance remains one channel unit, separate from the 1/30 + 0.0001 CSS-pixel
geometry gate. Retained source/Cage cohorts, literal source layers, actual input,
failed UI isolation, Glory batch refresh and explicit component reconstruction
are checked. A plain SVG control preserves WebKit's missing window-exit events
after stationary path replacement; the delivered-exit check samples the accepted
body with a real Move first. This remaining input limitation is explicit.

Native and same-artifact regression scope, upstream adapters, remaining gaps and
local CI verification are recorded in [verification](verification.md#development-funnel-literal-svg-layers).
This work remains Unreleased and does not claim full Nivo, React/Spring or OS input
compatibility. GPU/worker remains deferred.


## Funnel native hooks and retained tooltips

The dedicated SVG owner passes five configurations (Chrome, Edge, Chromium,
Firefox and WebKit) at DPR1/2: 40 direction/interpolation profiles, 2,000 accepted
frames, 160 rejected transactions, 20 paint/placement negative controls and 2,360
saved PNGs. Original MouseEvent identity, Window bubble phase, immutable admission data,
independent held Moves, stable occurrence replacement, registration lifecycle,
reentrant callbacks and cleanup are covered. Standard literal text and retained
Glory tooltips cover nine natural-size anchors, state and keyboard focus,
accepted-source rebinding, joint source/layer/tooltip adoption and failed-UI latching.

RGBA tolerance is one channel unit and geometry tolerance is 1/30 + 0.0001 CSS
pixels. A fixed plain-HTML calibration chooses one percent-reference model per
context and verifies 36 separate holdouts before product frames. Independent
readback reconstructs expected geometry and source-over pixels from archived
records. Rejected transaction viewport PNGs remain exactly equal. Chromium-family
contexts explicitly disable GPU; earlier GPU-enabled one-pixel antialias failures
remain preserved diagnostics. Hardware compositor behavior is outside this result.

The same package passes the prior full literal Funnel SVG suite and Chromium
DPR1/2 Tree node/link tooltip regressions. Hover is primary/non-touch and click
excludes touch. Stationary hovered-path replacement can leave WebKit without a
delivered window-exit event; no missing Enter/Leave is fabricated. Current tooltip
data rebases to accepted source while callback frames retain admission provenance.
No React/Spring/OS input, rotated/skewed CSS or arbitrary item-widget equivalence
is inferred. See [verification](verification.md#development-funnel-native-events-and-tooltips),
[events](funnel-events.md) and [tooltips](funnel-tooltip.md). This work is Unreleased;
GPU/worker remains deferred.

## Funnel bound annotations (Unreleased)

One frozen source-bound consumer and driver pass Chrome, Edge, bundled Chromium,
Firefox and WebKit at DPR1/2 across vertical/horizontal and linear/smooth profiles:
40 profiles, 900 frames, 120 rejected candidates, 20 actual DOM negative controls
and 40 glyph-stroke paint-contribution controls. The original matrix terminal
exits 0 and the complete source/package fence stays unchanged. Independent
readback decodes every original PNG, including isolated native reference pages.

The same two geometrically selected dot cells compare at RGBA1 to the committed
original Nivo native-circle component at each authored pose. Reference inputs
contain no product paths or pixels; source, compiler, Node executable, HTML,
DOM, PNG and worker terminals are archived. The earlier Firefox pure-fill
expectation was disproved in plain native SVG as well as foreignObject, with
failed attempts preserved. This correction does not change the authored
geometry, sample cells or tolerances, and proves no whole-primitive antialias
or compositor equivalence. Chromium contexts explicitly disable GPU.

Source, layer and tooltip adoption, actual business input occlusion, repeated
and omitted layers, camera/CSS, hidden sources, raw-only values, reordering,
custom notes, local state and cleanup remain part of the same full matrix.
The same package passes Chromium DPR1/2 legacy Funnel literal/event/tooltip,
Tree Canvas/Widgets and SVG Tree motion regressions. GPU/worker, annotation
Spring/exit, per-note retained Widgets and arbitrary live UI export remain
separate. See [verification](verification.md#development-funnel-bound-annotations)
and the [bilingual guide](funnel-annotations.zh-CN.md).

## Funnel owned application data (Unreleased)

The same frozen package and authored driver pass Chrome, Edge, bundled Chromium,
Firefox and WebKit at DPR1/2 for vertical/horizontal and linear/smooth Funnel
profiles. All 40 profiles, 2,010 frames, 360 rejected candidates and ten actual
DOM paint negatives pass. Independent offline readback decodes all 2,020 original
PNGs and verifies 240 synchronous presentation proofs, raw fields and numeric
bits, native admission provenance, retained component cohorts, focus, cleanup
and accepted-source recovery. Original matrix and reader processes both exit 0.

The deliberately traitless application datum includes nested business fields,
signed zero and Rc/Cell. Raw-only updates change actual Glory paint while keeping
the projection unchanged; historical events retain the admitted source and
tooltip selection rebases. A capture-to-bubble source replacement drops the
stale native event. Source, layers and tooltip share one accepted complete frame.
Incremental native logging preserves original sequence conservation, including
explicit boundary records. Presentation-only commands are measured synchronously;
later delivered native Enter/Leave remains a separate source transaction.

The same package passes five selected legacy Funnel and Tree tooltip lanes.
Chromium-family contexts explicitly disable GPU. The typed dedicated owner is
SVG-only; existing pure Scene and role APIs serve Canvas/raster. Spring/exit,
item mixing, arbitrary live UI export and GPU/worker remain separate. See
[verification](verification.md#development-funnel-owned-application-data),
[English guide](funnel-data.md) and [中文指南](funnel-data.zh-CN.md).

## Funnel Scene-role property springs (Unreleased)

One frozen source-bound package and driver pass Chrome, Edge, bundled Chromium,
Firefox and WebKit at DPR1/2 across vertical/horizontal and linear/smooth
profiles. The original coordinator and five browser lane processes all exit 0:
40 profiles, 1,930 frames, 200 rejected candidates and ten real DOM paint
negatives. Independent offline readback also exits 0, decoding all 2,340
original PNGs and verifying 2,472 full lossless event batches, 13,018 original
entries and 11,858 unique accepted owner observations.

Exact Manual intervals verify independent physical values and velocities,
previous-target path resets, changed command topology, labels, separators and
bound annotations. Actual native path/text attributes, retained Glory paint,
local state, focus and same-Cage peers are checked. Real sample-only cells admit
trusted input, target-only cells do not, and capture-to-bubble physical
publication changes invalidate the old observation. All 40 matched-hidden
samples and 120 actual native-node stability observations pass. A real DOM
failure after leaf placement restores the whole viewport, attributes, child
nodes, positions and focused application component.

Automatic clocks advance relative serials, pause at zero BCR and synchronize
system motion preferences after hidden/settled restoration. Original numeric
process terminals, source/package/helper fences, PNG bytes and complete event
batches remain bound to the evidence. Earlier Clippy and JSON-size failures
are retained and do not supply partial profiles to this accepted matrix.

That matrix verifies the earlier captured Scene-role spring scope. Dormant Part
memory is covered by the extension below; complete React component lifetime
equivalence is not asserted.
The dedicated owner is SVG-only; Canvas is a direct sampled-Scene preview.
Chromium-family contexts explicitly disable GPU. Mutable D3 generators,
item-widget mixing, arbitrary live UI export and GPU/worker remain separate.
See [verification](verification.md#development-funnel-property-springs),
[English guide](funnel-motion.md) and [中文指南](funnel-motion.zh-CN.md).

## Development: dormant Funnel role memory

The focused extension passes five engines, two DPRs and four profiles:
**40 profiles / 560 frames / 880 operations**. Independent Python spring algebra
and original D3 templates verify default and collapsed geometry, previous-target
path resets, style velocity, source removal/reentry, native path membership and
40 real no-body input negatives. Its 560 original PNGs are SHA checked and
decoded for structure; they are diagnostics, with no standalone pixel claim.

A fresh complete ordinary-motion matrix on the same source-bound package also
passes **40 profiles / 1930 frames / 2340 original PNGs**, including 5720 RGBA
checks, 200 rejected transactions, 10 DOM negative controls, 120 retained-node
observations and 40 physical-input pairs. Independent streamed readback checks
2432 complete gzip event batches containing 11,525 entries, including all 11,005
effects. All five original engine processes and the coordinator exit 0. The
independent ordinary reader and numeric replay retain their previously frozen
assertions byte for byte.

The first ordinary attempt is retained. The revised hidden-clock precondition
hides before retargeting and requires an unsettled hidden target; the remaining
pause/resume gates are unchanged. No successful profiles are spliced from that
attempt. Current evidence is retained in
`artifacts/verification/funnel-dormant-browser-matrix-first`,
`artifacts/verification/funnel-dormant-motion-regression-second`,
`artifacts/verification/funnel-dormant-independent-focused-final` and
`artifacts/verification/funnel-dormant-independent-motion-final`.

Twelve native tests cover custom all-source capture, uncalled legacy factories,
typed raw data, transparent explicit colors, invalid latent paint and budgets.
Default/all-feature/doctest counts are 2077/2422/18. This remains Unreleased;
the dedicated owner is SVG, pure sampled Scenes render through Canvas/raster,
and GPU/worker remains deferred. See [verification](verification.md#development-funnel-dormant-source-role-motion),
[English](funnel-dormant-motion.md) and [中文](funnel-dormant-motion.zh-CN.md).
