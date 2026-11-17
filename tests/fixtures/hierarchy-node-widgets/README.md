# Hierarchy component contracts

This reference preserves 29 complete Nivo 0.99.0 source files and the complete
MIT licenses for Treemap, CirclePacking, Icicle, Rects and Theming. It records
all source hashes and the pinned existing Playwright TSX compiler hash.

The three component protocols are executed separately:

| Family | Original expressions exercised | Scope |
| --- | --- | --- |
| Treemap | Leaves/descendants selector, interactive-node hook, transition keys/phases, `createElement` argument mapping, default node JSX and leaf/parent label conditions | 16 profiles, 280 creation-argument calls |
| CirclePacking | Leaves/descendants selector, full `Circles` component recipe, transition helper and radius projection, interaction gates | 4 profiles, 14 update-pose creation-argument calls; enter/update/leave helpers recorded separately |
| Icicle | `IcicleNodes` → shared `RectNodes` → `RectNodeWrapper` JSX, rectangle transition recipes, event gates, invisible-node focus and keyboard callbacks | 18 profiles, 90 custom-node JSX creation calls |

The source TSX is compiled by the already installed Playwright Babel bundle.
The JSX runtime only records arguments and synchronously invokes the original
wrapper recipes. Hooks, hierarchy selector methods, color/property accessors
and scalar `.to` operations use explicit probes. These are not real React
components, Spring values, DOM focus transport, or lifecycle tests.

Computed input geometry is authored. This reference does **not** execute or
compare D3 hierarchy sums, packing, treemap tiling, partition coordinates,
zoom geometry, or sorting. Raw-versus-aggregate and ID/path conventions are
also documented from the preserved source. Existing numeric layout references
remain separate. No new runtime dependency is required.

Treemap keys are root-to-node dot paths; repeated own IDs in separate branches
remain distinguishable. CirclePacking creation uses own IDs and stores paths
in own-to-root order; its raw `data` retains nested children. Icicle uses
`hierarchy.path` as transition UID and may retain invisible nodes for navigation.
These are not equivalent to Kartis's flat, globally identified source tree.

Zero-area/radius computed inputs are not filtered by these creation components.
Treemap opacity applies to rectangle fill; its leaf and parent Text labels use
their separate animated opacity. With skip-size zero, a zero-area leaf can
still have a label; positive skip sizes use strict `min(width,height) > skip`.
CirclePacking and Icicle labels are separate layers. Kartis's actual-Mark
eligibility, source opacity, focus clipping and Scene-owned labels must be
described using their Rust contracts rather than assumed identical to Nivo.

After installing pinned project packages, use Node 24 or newer:

```sh
node scripts/hierarchy-node-widget-reference.mjs --frozen
```

An optional fixture directory follows `--frozen`. Verification reads the frozen
files, checks source/license/compiler hashes, reexecutes the original recipes,
and compares every result exactly. It invokes no Git checkout and writes no
files. Regeneration is separate:

```sh
node scripts/hierarchy-node-widget-reference.mjs <nivo-checkout> [output-directory]
```
