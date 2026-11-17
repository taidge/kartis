# Original Tree activity reference

`reference.json` records executed recipes from Nivo 0.99.0. `original/` contains complete upstream source files, package manifests, and four complete MIT notices. The reference is compact JSON without omitted fields.

Verify the frozen reference using the pinned project dependencies:

```sh
npm ci
node scripts/tree-activity-reference.mjs --frozen
```

Verification reads the frozen files and existing compiler package. It does not require an upstream checkout, Git, a network request, or fixture writes. The optional fixture-directory argument supports isolated integrity checks. Regeneration uses `node scripts/tree-activity-reference.mjs <nivo-checkout> <output-directory>`.

The recipes execute the original current-node setter, node and link hooks, four modifiers, fixed-depth color inheritance, label computation, node mouse handlers, mesh event handlers, and SVG/Canvas mesh transport expressions. They cover every combination of the four highlight flags, neutral/focused/cleared state, function/scalar/unspecified modifiers, zero results, and original numeric threshold boundaries.

React hooks execute as explicit synchronous dependency probes. Hierarchy traversal and coordinates, cursor positions, and Delaunay matches are authored inputs. The JSX compiler records creation arguments; no React, Spring, browser event loop, Voronoi matching, or D3 numeric-layout equivalence is claimed.

A neutral state has `isActive: null` and calls neither modifier branch. With any focused node, links use their own independent active-link set, including an empty set that makes every link inactive. Fixed-depth inheritance skips the deeper node's color callback. The default node handler changes current state on enter and leave; its move handler does not. The mesh move handler does update current state. Keyboard focus and Rust flat identifiers are separate host/profile contracts.
