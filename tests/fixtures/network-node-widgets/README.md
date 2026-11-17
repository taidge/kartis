# Network node component contracts

This fixture preserves eight complete Nivo 0.99.0 source files, its package
manifest and complete MIT license. Verification checks source, producer and
the existing pinned TSX compiler hashes before executing original recipes.

The original raw-node preparation and computed-style hook are executed with
explicit dependency probes. Four normal/active/inactive style profiles and
default style values are recorded. Twenty component profiles cover animation,
interaction and five transition poses, producing 80 creation configurations.
The original default node JSX, four mouse handlers and tooltip/active-node
ordering are executed. Eight additional cases record default Canvas commands.

The original preparation shallow-copies raw data, initializes transient
position/velocity to zero, and later receives D3 simulation mutations. This
reference authors the resulting coordinates, velocities and indices. It does
not execute force simulation, React lifecycle, Spring scheduling, browser
event transport, actual Canvas color parsing or framebuffer equivalence.

The parent nodes layer requires a non-null node array; an empty array produces
no components. Zero size, transparent color and off-viewport computed nodes
are not filtered by component creation. Interaction only gates four handlers.
Initial/enter/update use scale and opacity one; from/leave use zero while size
is retained. Default radius is size/2 with no Swarm-style nonnegative clamp.
Mouse enter shows tooltip `{node}`, sets active IDs, then calls the consumer;
leave hides the tooltip, clears active IDs, then calls the consumer.

Kartis supplies current `ForceNodeState` snapshots rather than arbitrary
original Nivo `InputNode` objects. Physics radius, fixed coordinates, ticks and
heat belong to its Rust API and remain separate from visual glyph size/bounds.
Empty-label/no-ink Widget roles do not imply additional canonical datum
controls. Scene-owned labels and graph relation behavior remain separate.

After installing pinned project packages, verification requires no checkout,
Git, network or filesystem writes:

```sh
node scripts/network-node-widget-reference.mjs --frozen
```

An optional fixture directory follows `--frozen`. Regeneration is separate:

```sh
node scripts/network-node-widget-reference.mjs <nivo-checkout> [output-directory]
```
