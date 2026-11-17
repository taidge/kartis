# World country polygons

[English](README.md) | [简体中文](README.zh-CN.md)

`world_countries.json` is the unmodified 176-feature GeoJSON used by Nivo's geographic examples. It contains Polygon and MultiPolygon features with stable country IDs and country names.

Source: https://github.com/plouc/nivo/blob/0dae2c32052a573f9f1d66ec1b453b572119b265/website/src/data/components/geo/world_countries.json

The original bytes, commit and SHA-256 are recorded in `source-index.json`. `NIVO-LICENSE.md` contains the full upstream MIT license. `world_countries.closed.geojson` is a reproducible derived file: the original KOR polygon 0 / ring 0 is not closed. `normalize_world.py` preserves all 62 original coordinates and appends its first coordinate. This is the only ring edit; the original source bytes remain unchanged. Run `python site/wasm/data/normalize_world.py --check` to verify the derived bytes. The manifest records both SHA-256 values and the adaptation.

Kartis strictly parses the complete derived 176-feature collection with its public `geodata::parse_geojson` API. The two gallery maps explicitly render the **175 non-Antarctic regions**: ATA (Antarctica) is excluded because the current planar projection profile rejects pole-enclosing spherical-cap topology. No other feature is skipped or simplified; library topology and budget validation remain enabled. The author-data panel exposes `sourceFeatureCount`, `renderedFeatureCount`, `excluded`, the normalization and attribution. Demonstration values are assigned by stable country ID. The values are illustrative, not observed country statistics. Borders and names are retained from the upstream dataset and do not express a position on territorial status.
