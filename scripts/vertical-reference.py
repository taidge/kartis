"""Freeze a licensed CJK subset and its native HarfBuzz vertical layout.

Install fonttools==4.67.0 and uharfbuzz==0.52.0 in a tooling environment,
then run python scripts/vertical-reference.py --check from the workspace.
Remove --check to regenerate. Rust tests use only the checked-in artifacts.
"""

import argparse
import hashlib
import io
import json
from pathlib import Path
import subprocess
from urllib.request import urlopen

import fontTools
from fontTools import subset
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont
import uharfbuzz as hb


ROOT = Path(__file__).resolve().parent.parent
COMMIT = "8b0a1d0f5983c89bc2b93f1b5fb55f9e252744b5"
SOURCE = f"https://raw.githubusercontent.com/google/fonts/{COMMIT}/ofl/notosansjp"
CASES = [
    [("日本（漢字）。", "Hani")],
    [("カタカナー", "Kana")],
    [("日本", "Hani"), ("A", "Latn"), ("。漢字", "Hani")],
    [("。", "Zyyy")],
]


def source(cache, name, filename, sha256):
    path = cache / name
    if not path.exists():
        with urlopen(f"{SOURCE}/{filename}", timeout=45) as response:
            path.write_bytes(response.read())
    data = path.read_bytes()
    if hashlib.sha256(data).hexdigest() != sha256:
        raise ValueError(f"Unexpected source SHA-256 for {name}")
    return data


def font_subset(data, vertical_alternates=True):
    font = TTFont(io.BytesIO(data), recalcTimestamp=False)
    options = subset.Options()
    options.recalc_timestamp = False
    options.layout_features = ["*"]
    if not vertical_alternates:
        options.layout_features = sorted({record.FeatureTag
            for record in font["GSUB"].table.FeatureList.FeatureRecord
            if record.FeatureTag not in {"vert", "vrt2"}})
    sub = subset.Subsetter(options)
    sub.populate(text="天地 ABfi()é\u0301\u20dd（）ー。" + "".join(text for case in CASES for text, _ in case))
    sub.subset(font)
    font = instantiateVariableFont(font, {"wght": 400}, inplace=True)
    family = "Kartis Fixture Vertical" if vertical_alternates else "Kartis Fixture Vertical Fallback"
    for field, name in {
        1: family, 2: "Regular", 3: family + " Regular",
        4: family + " Regular", 6: family.replace(" ", "") + "-Regular",
        16: family, 17: "Regular",
    }.items():
        font["name"].removeNames(nameID=field)
        font["name"].setName(name, field, 3, 1, 0x409)
    out = io.BytesIO()
    font.save(out, reorderTables=False)
    return out.getvalue()


def reference(data):
    font = hb.Font(hb.Face(data))
    lines = [
        "# Noto Sans JP subset, HarfBuzz 12.1.0 through uharfbuzz 0.52.0",
        "# Hex scalars; vertical advance; glyph, UTF-8 cluster, screen x, screen y",
    ]
    for case in CASES:
        x = y = offset = 0
        glyphs = []
        for text, script in case:
            buffer = hb.Buffer()
            encoded = text.encode()
            buffer.add_utf8(encoded)
            buffer.script = script
            buffer.direction = "ttb"
            buffer.flags = hb.BufferFlags.REMOVE_DEFAULT_IGNORABLES
            hb.shape(font, buffer)
            for glyph, position in zip(buffer.glyph_infos, buffer.glyph_positions):
                glyphs.append(f"{glyph.codepoint},{offset + glyph.cluster},"
                              f"{x + position.x_offset},{y - position.y_offset}")
                x += position.x_advance
                y -= position.y_advance
            offset += len(encoded)
        scalars = " ".join(f"{ord(c):X}" for text, _ in case for c in text)
        lines.append(f"{scalars};{y};{' '.join(glyphs)}")
    return ("\n".join(lines) + "\n").encode()


def mixed_reference(font_data):
    # These spans are explicitly authored orientation/script expectations.
    # Native HarfBuzz supplies glyph selection, advances, positions and boxes.
    upright = lambda text, script="Hani": (text, script, "ttb", False)
    sideways = lambda text, script="Latn": (text, script, "ltr", True)
    definitions = [
        (0, "RightToLeft", [[upright("日本"), sideways("ABfi"), upright("。", "Zyyy"),
                            upright("（", "Latn"), sideways("A"), upright("）", "Latn")]]),
        (0, "RightToLeft", [[upright("（）ー。", "Zyyy")]]),
        (1, "RightToLeft", [[sideways("（）ー", "Zyyy"), upright("。", "Zyyy")]]),
        (0, "RightToLeft", [[sideways("A\u0301B")]]),
        (0, "RightToLeft", [[upright("A\u20dd", "Latn")]]),
        (0, "RightToLeft", [[upright("日本"), sideways("AB")],
                            [upright("（", "Latn"), sideways("A"), upright("）", "Latn"), upright("。", "Zyyy")]]),
        (0, "LeftToRight", [[upright("日本"), sideways("AB")],
                            [upright("（", "Latn"), sideways("A"), upright("）", "Latn"), upright("。", "Zyyy")]]),
        (0, "RightToLeft", [[upright("日")], [], [upright("本")], []]),
    ]
    cases = []
    for font_index, flow, columns in definitions:
        font = hb.Font(hb.Face(font_data[font_index]))
        extents = font.get_font_extents("ltr")
        center = (extents.ascender + extents.descender) / 2
        glyphs = []
        measured_columns = []
        ink_points = []
        offset = 0
        for index, spans in enumerate(columns):
            column_x = index * 1500 * (-1 if flow == "RightToLeft" else 1)
            x, y = column_x, 0
            for text, script, direction, rotated in spans:
                buffer = hb.Buffer()
                encoded = text.encode()
                buffer.add_utf8(encoded)
                buffer.script = script
                buffer.direction = direction
                buffer.flags = hb.BufferFlags.REMOVE_DEFAULT_IGNORABLES
                hb.shape(font, buffer)
                for glyph, position in zip(buffer.glyph_infos, buffer.glyph_positions):
                    if rotated:
                        gx, gy = x - center + position.y_offset, y + position.x_offset
                    else:
                        gx, gy = x + position.x_offset, y - position.y_offset
                    glyphs.append([glyph.codepoint, offset + glyph.cluster, gx, gy, rotated])
                    bounds = font.get_glyph_extents(glyph.codepoint)
                    if bounds is not None and (bounds.width or bounds.height):
                        for a, b in [(bounds.x_bearing, bounds.y_bearing),
                                     (bounds.x_bearing + bounds.width, bounds.y_bearing + bounds.height)]:
                            ink_points.append((gx + b, gy + a) if rotated else (gx + a, gy - b))
                    if rotated:
                        x += position.y_advance
                        y += position.x_advance
                    else:
                        x += position.x_advance
                        y -= position.y_advance
                offset += len(encoded)
            measured_columns.append([column_x, y])
            offset += 1  # Explicit LF between columns, including empty columns.
        left, right = min(point[0] for point in ink_points), max(point[0] for point in ink_points)
        top, bottom = min(point[1] for point in ink_points), max(point[1] for point in ink_points)
        cases.append({
            "font": font_index, "columns": flow,
            "text": "\n".join("".join(part[0] for part in spans) for spans in columns),
            "column_positions": measured_columns, "glyphs": glyphs,
            "ink_bounds": [left, top, right - left, bottom - top],
        })
    json_data = (json.dumps({"harfbuzz": "12.1.0", "units_per_em": 1000, "cases": cases},
                           ensure_ascii=True, indent=2) + "\n").encode()
    rust = ["// Native HarfBuzz 12.1.0 oracle, generated by scripts/vertical-reference.py.", "&["]
    for case in cases:
        rust.append(f"    ({case['font']}usize, {str(case['columns'] == 'RightToLeft').lower()},")
        rust.append("     &[" + ", ".join(f"0x{ord(c):X}u32" for c in case["text"]) + "] as &[u32],")
        rust.append("     &[" + ", ".join(f"({float(x)}, {float(y)})" for x, y in case["column_positions"]) + "] as &[(f64, f64)],")
        rust.append("     &[")
        for glyph, cluster, x, y, rotated in case["glyphs"]:
            rust.append(f"         ({glyph}u16, {cluster}u32, {float(x)}, {float(y)}, {str(rotated).lower()}),")
        rust.append("     ] as &[(u16, u32, f64, f64, bool)],")
        rust.append("     [" + ", ".join(str(float(value)) for value in case["ink_bounds"]) + "]),")
    rust.append("]")
    return json_data, ("\n".join(rust) + "\n").encode()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if (fontTools.version, hb.__version__, hb.version_string()) != ("4.67.0", "0.52.0", "12.1.0"):
        raise ValueError("Use the pinned fontTools and uharfbuzz versions")
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"], cwd=ROOT,
    ))
    cache = Path(metadata["target_directory"]) / "vertical-reference"
    cache.mkdir(parents=True, exist_ok=True)
    raw = source(cache, "NotoSansJP.ttf", "NotoSansJP%5Bwght%5D.ttf",
                 "c2f3b4d463500a2ddcd3849cded1fceeb9fd6d1c32e6cbecd568453ba50fc68f")
    license_data = source(cache, "NotoSansJP-OFL.txt", "OFL.txt",
                          "1c05c68c34f9708415aada51f17e1b0092d2cea709bf4a94cd38114f9e73d7d9")
    font = font_subset(raw)
    fallback = font_subset(raw, vertical_alternates=False)
    mixed_json, mixed_rust = mixed_reference([font, fallback])
    for path, data in {
        "tests/fonts/KartisFixtureVertical-Regular.ttf": font,
        "tests/fonts/KartisFixtureVertical-OFL.txt": license_data,
        "tests/fonts/KartisFixtureVerticalFallback-Regular.ttf": fallback,
        "tests/fonts/KartisFixtureVerticalFallback-OFL.txt": license_data,
        "tests/fixtures/vertical-harfbuzz.txt": reference(font),
        "tests/fixtures/vertical-mixed-harfbuzz.json": mixed_json,
        "tests/fixtures/vertical-mixed-harfbuzz.rs": mixed_rust,
    }.items():
        output = ROOT / path
        if args.check:
            if output.read_bytes() != data:
                raise ValueError(f"Fixture differs from {path}")
        else:
            output.write_bytes(data)


if __name__ == "__main__":
    main()
