# Exact viewport screenshot sampling

`scripts/acceptance-viewport.mjs` provides read-only capture and sampling for
browser verification. It captures the whole configured viewport, records the
surface's actual DOM rectangle and DPR, and maps logical surface points to
physical PNG indices with one floor operation.

```javascript
import {captureViewport,rgbaAtLogical} from '../scripts/acceptance-viewport.mjs';
await surface.scrollIntoViewIfNeeded();
const capture = await captureViewport(page, surface);
const sample = rgbaAtLogical(capture, {
    point: [120.5, 80.25], // after the application's accepted paint camera
    size: [600, 260],     // logical surface viewport dimensions
});
console.log(sample.client, sample.index, sample.rgba);
```

The helper does not scroll, change CSS, hide a caret, select a nearby pixel or
change application state. It requires unchanged DOM rectangle, viewport,
scroll and visual-viewport metadata around capture. PNG dimensions must match
configured viewport dimensions multiplied by DPR exactly. Logical points outside
their surface, and points outside the captured viewport, are rejected. Partial
offscreen surfaces are allowed only for queries that are actually visible.
Visual viewport zoom or offsets are rejected. Callers supply a positive
axis-aligned mapping; a bounding rectangle cannot explain rotation, skew or
perspective. Stable geometry alone does not prove stable ink or an application
frame; callers also control their accepted frame and animation state.

Element screenshots can include integer padding around fractional CSS bounds.
For example, a 260px surface can yield a 261px-high element PNG at DPR1.
Treating 261/260 as a content scale can select an adjacent row. The existing
`acceptance-png.mjs::pixel` remains a fitted-image sampler for its original
callers. Historical reports are preserved; this tool does not retroactively
validate or invalidate every older sample.

The standalone `scripts/viewport-capture-acceptance.mjs` verifies fractional
origins, positive nonuniform scaling and scroll without Kartis rendering.
It uses native CSS colors, a physical-pixel-coded Canvas and a separately laid
out native point marker. Unscrolled coordinates also have fixed independent
expected indices. Actual browser scroll is recorded rather than assumed to
equal the requested scroll. A real screenshot-then-DOM-move control must be
rejected. CI uses `--chromium-only`; the full matrix covers Chrome, Firefox,
WebKit and Edge at DPR1/2.

The scrolled fixed-Canvas tag palette remains a diagnostic: Firefox's fractional
scroll can shift its raster phase by one or two pixels in this native fixture.
Its DOM-marker coordinate/index checks remain strict. Hard palette comparisons
cover the unscrolled fractional and scaled profiles; broad cyan CSS panels
add independent visible-color checks while scrolling. No neighboring color is
selected as an expected value and no pixel tolerance is widened.

The Bar font8 case is separate. Correct full-viewport sampling still finds its
original G98 sample, so this coordinate correction does not resolve that glyph
comparison. Native DOM with an activated transparent Canvas reproduces it, and
an independent non-LCD text profile matches the local glyph region. These
diagnostics establish a browser compositing/text path explanation for that
carrier, not uniform arbitrary text equality or a change to production flags.
See [verification](verification.md).

`node scripts/native-text-compositor-diagnostic.mjs` runs a portable native-only
probe with explicit English document language, fixed synthetic text and geometry.
It records default and diagnostic non-LCD profiles separately, including font0
and black-text controls. It does not load the chart host or declare a universal
passing text color. Language is part of the carrier: identical computed CSS
font-family alone does not establish identical text layout.

Skia's [rasterization design notes](https://docs.skia.org/docs/dev/design/raster_tragedy/)
discuss distinctions between full-pixel/subpixel text and gamma/contrast
adjustments. They provide background; the native experiments establish this
specific reproduction and do not identify a unique Skia implementation branch.
