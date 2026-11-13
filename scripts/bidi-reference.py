"""Freeze the raster Bidi font subsets and Unicode/HarfBuzz test fixtures.

Install fonttools==4.67.0 and uharfbuzz==0.52.0 in a tooling environment,
then run from the Kartis workspace: python scripts/bidi-reference.py --check
Remove --check to regenerate. Downloaded source files stay in Cargo's
workspace target_directory. Rust tests consume only checked-in artifacts.
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
GOOGLE = f"https://raw.githubusercontent.com/google/fonts/{COMMIT}/ofl"
UNICODE = "https://www.unicode.org/Public/16.0.0/ucd/BidiCharacterTest.txt"
UNICODE_LICENSE = "https://www.unicode.org/license.txt"
SOURCES = {
    "Arabic": "63111b5b2e074dd48cc67692e0a2726d86ee94c1c37fe8598257b7b4e87e869e",
    "Hebrew": "7ef36a2c3593758cdb622e1bdef4f84523e92fbc3ccc667438dd80ff54c2de88",
}


def source(cache, name, url, sha=None):
    path = cache / name
    if not path.exists():
        with urlopen(url, timeout=45) as response:
            path.write_bytes(response.read())
    data = path.read_bytes()
    if sha is not None and hashlib.sha256(data).hexdigest() != sha:
        raise ValueError(f"Unexpected source SHA-256 for {name}")
    return data


def font_subset(data, script):
    font = TTFont(io.BytesIO(data), recalcTimestamp=False)
    font = instantiateVariableFont(font, {"wdth": 100, "wght": 400}, inplace=True)
    # Include Hebrew letters and marks, and the Arabic joining/mark fixture's
    # alphabet. No runtime system font access is needed by the Rust tests.
    if script == "Hebrew":
        chars = list(range(0x591, 0x5EB))
    else:
        chars = sorted(set(map(ord, "سلام السَّلَام العربية مرحبا ابج")))
    options = subset.Options()
    options.recalc_timestamp = False
    sub = subset.Subsetter(options)
    sub.populate(unicodes=chars + list(range(32, 127)) + [0x25CC])
    sub.subset(font)
    family = f"Kartis Fixture {script}"
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


def hexdump(text):
    return " ".join(f"{ord(c):X}" for c in text)


def harfbuzz_cases(font_data):
    faces = [hb.Face(data) for data in font_data]
    fonts = [hb.Font(face) for face in faces]
    # Parts listed in expected left-to-right visual run order. Within each
    # part, pass original logical bytes to the independent native shaper.
    cases = [
        ("auto", "abc אבג 123", [(0, "abc ", 0, "Latn", "ltr"), (8, "123", 0, "Zyyy", "ltr"), (7, " ", 0, "Hebr", "rtl"), (4, "אבג", 2, "Hebr", "rtl")]),
        ("rtl", "אבג (123) abc", [(10, "abc", 0, "Latn", "ltr"), (8, ") ", 0, "Zyyy", "rtl"), (5, "123", 0, "Zyyy", "ltr"), (3, " (", 0, "Hebr", "rtl"), (0, "אבג", 2, "Hebr", "rtl")]),
        ("auto", "سلام", [(0, "سلام", 3, "Arab", "rtl")]),
        ("rtl", "السَّلَام", [(0, "السَّلَام", 3, "Arab", "rtl")]),
        ("auto", "س\u200cلام", [(0, "س\u200cلام", 3, "Arab", "rtl")]),
        ("auto", "\u2067אבג\u2069 xyz", [(1, "אבג", 2, "Hebr", "rtl"), (5, " xyz", 0, "Latn", "ltr")]),
        ("ltr", "A\u0323B", [(0, "A\u0323", 1, "Latn", "ltr"), (2, "B", 0, "Latn", "ltr")]),
        ("ltr", "A\u2029אב", [(0, "A ", 0, "Latn", "ltr"), (2, "אב", 2, "Hebr", "rtl")]),
        ("ltr", "abc \u202eabc\u202c 12", [(0, "abc ", 0, "Latn", "ltr"), (10, "12", 0, "Zyyy", "ltr"), (9, " ", 0, "Latn", "rtl"), (5, "abc", 0, "Latn", "rtl")]),
        ("auto", "אָב שָׁלוֹם", [(4, "שָׁלוֹם", 2, "Hebr", "rtl"), (3, " ", 0, "Hebr", "rtl"), (0, "אָב", 2, "Hebr", "rtl")]),
    ]
    rows = [
        "# HarfBuzz 12.1.0 through uharfbuzz 0.52.0",
        "# base;text codepoints;advance in em;glyphs (font,gid,UTF-8 cluster,x em,y em)",
    ]
    for base, text, parts in cases:
        glyphs = []
        advance = 0.0
        for offset, part, font, script, direction in parts:
            assert part == text[offset:offset + len(part)] or part == "A "
            buffer = hb.Buffer()
            buffer.add_utf8(part.encode())
            buffer.script = script
            buffer.direction = direction
            buffer.flags = hb.BufferFlags.REMOVE_DEFAULT_IGNORABLES
            hb.shape(fonts[font], buffer)
            units = faces[font].upem
            start = len(text[:offset].encode())
            for glyph, position in zip(buffer.glyph_infos, buffer.glyph_positions):
                x = advance + position.x_offset / units
                y = -position.y_offset / units
                glyphs.append(f"{font},{glyph.codepoint},{start + glyph.cluster},{x:.16g},{y:.16g}")
                advance += position.x_advance / units
        rows.append(f"{base};{hexdump(text)};{advance:.16g};{' '.join(glyphs)}")
    return ("\n".join(rows) + "\n").encode()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    check = parser.parse_args().check
    assert fontTools.__version__ == "4.67.0"
    assert hb.__version__ == "0.52.0" and hb.version_string() == "12.1.0"
    metadata = subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"], cwd=ROOT
    )
    cache = Path(json.loads(metadata)["target_directory"]) / "bidi-reference"
    cache.mkdir(exist_ok=True, parents=True)

    def emit(relative, data):
        path = ROOT / relative
        if check:
            # Checkout line ending normalization applies to text fixtures.
            stored = path.read_bytes()
            if path.suffix != ".ttf":
                stored = stored.replace(b"\r\n", b"\n")
            assert stored == data, f"Fixture drift: {relative}"
        else:
            path.write_bytes(data)
        print(f"{hashlib.sha256(data).hexdigest()}  {relative}")

    fixtures = {}
    for script, digest in SOURCES.items():
        family = f"notosans{script.lower()}"
        data = source(cache, f"{script}.ttf",
                      f"{GOOGLE}/{family}/NotoSans{script}%5Bwdth%2Cwght%5D.ttf", digest)
        fixtures[script] = font_subset(data, script)
        emit(f"tests/fonts/KartisFixture{script}-Regular.ttf", fixtures[script])
        license_data = source(cache, f"{family}-OFL.txt", f"{GOOGLE}/{family}/OFL.txt")
        emit(f"tests/fonts/KartisFixture{script}-OFL.txt", license_data.replace(b"\r\n", b"\n"))
    names = ["ABeeZee-Regular.ttf", "Lato-Regular.ttf"]
    fonts = [(ROOT / "tests/fonts" / name).read_bytes() for name in names]
    emit("tests/fixtures/bidi-harfbuzz.txt",
         harfbuzz_cases(fonts + [fixtures["Hebrew"], fixtures["Arabic"]]))
    ucd = source(cache, "BidiCharacterTest.txt", UNICODE).decode()
    rows = [line for line in ucd.splitlines() if line and line[0] != "#"]
    selected = ["# Unicode 16.0.0 BidiCharacterTest excerpt, copyright 2024 Unicode, Inc.",
                "# Licensed under Unicode License v3 (bidi-unicode-LICENSE.txt)."]
    selected.extend(rows[:96])
    emit("tests/fixtures/bidi-unicode-16.txt", ("\n".join(selected) + "\n").encode())
    license_data = source(cache, "unicode-LICENSE.txt", UNICODE_LICENSE)
    emit("tests/fixtures/bidi-unicode-LICENSE.txt", license_data.replace(b"\r\n", b"\n"))


if __name__ == "__main__":
    main()
