"""Generate authored COLR fixtures and an independent native FontFace page.

Use fonttools==4.67.0 and run python scripts/colr-font-reference.py --check.
Remove --check to regenerate. All shapes are original fixture artwork and
are distributed under the repository's Apache-2.0 license.
"""
import argparse
import io
import json
from pathlib import Path

import fontTools
from fontTools.colorLib.builder import buildCOLR, buildCPAL
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import TTFont

ROOT = Path(__file__).resolve().parent.parent
PALETTES = [
    [(255, 0, 0, 255), (0, 0, 255, 128), (0, 0, 0, 0)],
    [(0, 180, 0, 255), (255, 255, 0, 128), (0, 0, 0, 0)],
]


def contour(pen, points):
    pen.moveTo(points[0])
    for point in points[1:]:
        pen.lineTo(point)
    pen.closePath()


def rectangle(pen, x0, y0, x1, y1, reverse=False):
    points = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
    contour(pen, list(reversed(points)) if reverse else points)


def glyph(name):
    pen = TTGlyphPen(None)
    if name in {"color", "color.vert", "color2"}:
        rectangle(pen, 0, 0, 100, 100)
    elif name == "A":
        contour(pen, [(50, 0), (500, 1000), (950, 0)])
    elif name == "ring":
        rectangle(pen, 0, 0, 1000, 1000)
        rectangle(pen, 200, 200, 800, 800, reverse=True)
    elif name == "blue":
        rectangle(pen, 400, 100, 900, 900)
    elif name == "foreground":
        rectangle(pen, 0, 0, 200, 300)
    elif name == "transparent":
        rectangle(pen, -200, -200, 1200, 1200)
    return pen.glyph()


def build(variant="normal"):
    order = [".notdef", "space", "A", "color", "ring", "blue", "foreground",
             "transparent", "color.vert", "color2"]
    builder = FontBuilder(1000, isTTF=True)
    builder.setupGlyphOrder(order)
    builder.setupCharacterMap({32: "space", 65: "A", 66: "color", 0xE000: "color", 0xE001: "color2"})
    glyphs = {name: glyph(name) for name in order}
    builder.setupGlyf(glyphs)
    builder.setupHorizontalMetrics({name: (1000, getattr(value, "xMin", 0)) for name, value in glyphs.items()})
    builder.setupHorizontalHeader(ascent=800, descent=-200)
    builder.setupVerticalMetrics({name: (1000, 1000 - getattr(value, "yMax", 0)) for name, value in glyphs.items()})
    builder.setupVerticalHeader(ascent=500, descent=-500)
    builder.setupOS2(sTypoAscender=800, sTypoDescender=-200, usWinAscent=1200, usWinDescent=200)
    family = "Kartis Fixture COLR V0 " + variant
    builder.setupNameTable({"familyName": family, "styleName": "Regular",
        "uniqueFontIdentifier": family, "fullName": family,
        "psName": family.replace(" ", ""), "version": "Version 1.000",
        "copyright": "Original Kartis fixture artwork. Apache-2.0.",
        "licenseDescription": "Licensed under the Apache License, Version 2.0.",
        "licenseInfoURL": "https://www.apache.org/licenses/LICENSE-2.0"})
    builder.setupPost()
    font = builder.font
    addOpenTypeFeaturesFromString(font, "feature vert { sub color by color.vert; } vert;")
    layers = [("ring", 0), ("blue", 1), ("foreground", 0xFFFF), ("transparent", 2)]
    other = [("ring", 0), ("foreground", 0xFFFF)]
    if variant == "bad-palette":
        other = [("ring", 3)]  # Crosses into the next palette, within the global color array.
    if variant == "too-many-layers":
        other = [("ring", 0)] * 257
    font["COLR"] = buildCOLR({"color": layers, "color.vert": [("ring", 1)], "color2": other},
                             version=0, glyphMap=font.getReverseGlyphMap())
    if variant == "version-one":
        font["COLR"] = buildCOLR({"color": {"Format": 10, "Glyph": "ring",
            "Paint": {"Format": 2, "PaletteIndex": 0, "Alpha": 1.0}}},
            version=1, glyphMap=font.getReverseGlyphMap())
    font["CPAL"] = buildCPAL([[tuple(channel / 255 for channel in color) for color in palette]
                              for palette in PALETTES])
    font.recalcTimestamp = False
    font["head"].created = font["head"].modified = 3406620150
    output = io.BytesIO()
    font.save(output, reorderTables=False)
    return output.getvalue()


def reference(data):
    # Reparse serialized font tables independently of the Rust font parser.
    font = TTFont(io.BytesIO(data))
    palettes = [[[color.red, color.green, color.blue, color.alpha] for color in palette]
                for palette in font["CPAL"].palettes]
    layers = {name: [[font.getGlyphID(layer.name), layer.colorID] for layer in records]
              for name, records in font["COLR"].ColorLayers.items()}
    return (json.dumps({"units_per_em": font["head"].unitsPerEm,
                        "palettes": palettes, "layers": layers}, indent=2) + "\n").encode()


HTML = """<!doctype html>
<html lang="en"><meta charset="utf-8"><title>COLR v0 native font reference</title>
<canvas id="reference" width="240" height="180"></canvas>
<script type="module">
const font = new FontFace('KartisColrReference', 'url(../fonts/KartisFixtureColrV0.ttf)');
document.fonts.add(await font.load());
const canvas = document.getElementById('reference');
const context = canvas.getContext('2d');
context.fillStyle = 'white'; context.fillRect(0, 0, canvas.width, canvas.height);
context.font = '100px KartisColrReference'; context.textBaseline = 'alphabetic';
context.fillStyle = 'rgb(0,255,0)'; context.fillText('B', 40, 140);
window.colrNativeReference = {
  ready: true,
  pixel: (x, y) => [...context.getImageData(x, y, 1, 1).data],
  pixels: () => [...context.getImageData(0, 0, canvas.width, canvas.height).data],
};
</script></html>
"""


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if fontTools.version != "4.67.0":
        raise ValueError("Use fonttools==4.67.0")
    font = build()
    outputs = {
        "tests/fonts/KartisFixtureColrV0.ttf": font,
        "tests/fonts/KartisFixtureColrBadPalette.ttf": build("bad-palette"),
        "tests/fonts/KartisFixtureColrTooManyLayers.ttf": build("too-many-layers"),
        "tests/fonts/KartisFixtureColrV1.ttf": build("version-one"),
        "tests/fonts/KartisFixtureColr-LICENSE.txt": (ROOT / "LICENSE").read_bytes(),
        "tests/fixtures/colr-font-reference.json": reference(font),
        "tests/fixtures/colr-native-reference.html": HTML.encode(),
    }
    for name, data in outputs.items():
        path = ROOT / name
        if args.check:
            if path.read_bytes() != data:
                raise ValueError(f"Fixture differs from {name}")
        else:
            path.write_bytes(data)


if __name__ == "__main__":
    main()
