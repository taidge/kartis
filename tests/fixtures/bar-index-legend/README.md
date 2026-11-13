# Original Nivo Bar/index legend reference

The producer executes the unmodified Nivo 0.99.0 grouped and stacked bar generator bodies, common helpers and getLegendDataForIndexes with actual released d3-scale4.0.2/d3-interpolate3.0.1/d3-array3.2.5/d3-shape3.2.0. Original source bytes and Nivo MIT license are preserved under vendor and pinned in source-pins.json; a D3-shape license is included. D3 implementations are supplied externally and are not copied here; existing repository bar-category-band D3 license notices also apply.

From a project containing these files:

    node scripts/bar-index-legend-reference.mjs <scales-node_modules> <shape-node_modules> --check

Without --check it regenerates values.txt/native-unusual.json/reference.json. The producer resolves source/outputs relative to its own location and imports pinned released modules from explicit arguments. It checks source bytes and module versions; only linear/band dispatcher branches are exercised. Type erasure/import wiring plus flat property and first-string-uniq adapters are explicit. It does not run React or claim complete lodash/CSS accessor/component parity.

32 profiles preserve both authored keys=[z,a] and lexical keys=[a,z], grouped/stacked, horizontal/vertical, reverse/round and hidden-z cases. 212 native source bars include106 null bars;128 index legend rows are retained. Native headers retain authored inputs; L rows contain original legend outputs; B rows contain original raw-null/value, full geometry and numeric palette CSS. Rust tests use the explicit complete series_order for actual authored geometry and fixed1e-12 comparisons. They preserve the existing Rust checked distinction that null source bars do not become drawn marks.

The native unusual supplement preserves original empty-index and absent-color black fallbacks. Rust Bar source IDs are nonempty checked strings and always resolve a typed Color, so those native records are not silently rewritten into accepted Rust expectations. Rust percent/window/crop/role semantics are separately tested extensions; they are not claimed as additional native factory branches. Index colors use original pre-custom palette, not prepared custom-paint representative colors.
