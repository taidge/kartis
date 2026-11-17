#!/usr/bin/env python3
"""Reproduce one explicit closing-coordinate repair without changing the source."""
import argparse
import copy
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
SOURCE_SHA = "bb261743fc2aa5e09ca70a2823e307bfe084fbb812df066a55fb2bf898248e17"


def normalized():
    source = (HERE / "world_countries.json").read_bytes()
    if hashlib.sha256(source).hexdigest() != SOURCE_SHA:
        raise ValueError("The original source changed; review its provenance before normalizing")
    original = json.loads(source)
    output = copy.deepcopy(original)
    changes = []
    assert original["type"] == "FeatureCollection" and len(original["features"]) == 176
    assert len({feature["id"] for feature in original["features"]}) == 176
    for feature in output["features"]:
        geometry = feature["geometry"]
        polygons = geometry["coordinates"] if geometry["type"] == "MultiPolygon" else [geometry["coordinates"]]
        for pi, polygon in enumerate(polygons):
            for ri, ring in enumerate(polygon):
                if ring and ring[0] != ring[-1]:
                    assert (feature["id"], pi, ri) == ("KOR", 0, 0)
                    assert len(ring) == 62
                    changes.append({"featureID": feature["id"], "polygonIndex": pi, "ringIndex": ri,
                                    "originalPositions": len(ring), "appendedPosition": ring[0]})
                    ring.append(copy.deepcopy(ring[0]))
    assert len(changes) == 1
    encoded = (json.dumps(output, ensure_ascii=False, separators=(",", ":")) + "\n").encode("utf-8")
    return encoded, changes


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="Compare the derived bytes without rewriting them")
    args = parser.parse_args()
    encoded, changes = normalized()
    target = HERE / "world_countries.closed.geojson"
    if args.check:
        assert target.read_bytes() == encoded, "Derived world data is stale"
    else:
        target.write_bytes(encoded)
    print(json.dumps({"originalFeatureCount": 176, "derivedFeatureCount": 176,
                      "originalPositions": 10586, "derivedPositions": 10587,
                      "sourceSHA256": SOURCE_SHA, "derivedSHA256": hashlib.sha256(encoded).hexdigest(),
                      "changes": changes}, indent=2))


if __name__ == "__main__":
    main()
