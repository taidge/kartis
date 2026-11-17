# Funnel tooltips (Kartis 0.2.0)

[简体中文](funnel-tooltip.zh-CN.md)

This SVG API is available in Kartis 0.2.0 and has passed its native-events and tooltip browser
acceptance: 10 configurations, 40 profiles, 2,000 frames, 160 rejections and 20
real DOM negative controls. Artifacts and the fixed Chromium `--disable-gpu`
test configuration are in the
[verification record](verification.md#development-funnel-native-events-and-tooltips).
It extends the dedicated `BrowserFunnel` owner, not generic Canvas charts or
every Nivo Funnel feature.

## Explicit configuration

`BrowserFunnel` and `BrowserFunnelHandle` provide `set_tooltip_options`,
`set_tooltip_component`, `clear_tooltip_component`, `reset_tooltip` and the
accepted `tooltip` getter. Keep the owner alive: its handle is weak. Setters
enqueue changes; `flush()` processes a shared source/layer/tooltip transaction.
Flushing from a native mouse callback is rejected; queued work runs afterward.

```rust
use kartis::browser::{
    BrowserFunnelHandle, BrowserFunnelTooltipAnchor, BrowserFunnelTooltipOptions,
};

fn enable_tooltip(owner: &BrowserFunnelHandle) -> kartis::Result<()> {
    owner.set_tooltip_options(BrowserFunnelTooltipOptions {
        enabled: true,
        anchor: BrowserFunnelTooltipAnchor::Top,
        gap: 14.,
    })?;
    owner.flush()
}
```

Existing mounts have no managed tooltip until configured. Setting a component
also enables default options. Options alone show safe text with a color chip,
the author's label, a colon and the part's already formatted value. An empty
label displays `: value`; it never falls back to a business ID. Text is inserted
through DOM text nodes, including HTML characters. `FunnelDatum.label` is a
`String`; this API does not preserve a raw object's missing-versus-empty field.
Clearing the component retains options and returns to standard text. Resetting
removes management and clears its selection. `enabled: false` hides presentation
while a configured component may retain its scope.

## Complete typed data and a real component

`FunnelTooltipFrame` contains the immutable `FunnelSvgFrame`, `part_index`,
`source_index`, accepted `source_generation` and `publication`, actual Parts
`occurrence` (key/index/publication), `pointer_client`, logical
`viewport_anchor`, `requested_anchor` and resolved `anchor`. Checked `part()`,
`datum()` and `style()` accessors expose computed metadata, original source and
captured constructor style. Current Scene ink remains a separate view; cloning
or reading the tooltip does not replay layout, color or formatting functions.

```rust
use glory::Widget;
use kartis::browser::BrowserFunnelHandle;

fn install_component(owner: &BrowserFunnelHandle) -> kartis::Result<()> {
    owner.set_tooltip_component(1, |context| {
        let frame = context.frame;
        Ok(glory::ui::widget(move |scope| {
            glory::ui::text()
                .content(scope.bond(move || {
                    frame.get().map_or_else(String::new, |frame| {
                        frame.text().unwrap_or_else(|error| error.to_string())
                    })
                }))
                .show_in(scope)
        }))
    })?;
    owner.flush()
}
```

This is a retained Glory Widget with a `Bond<Option<FunnelTooltipFrame>>`.
Construction starts with `None`; a constructor is not an accepted-source
notification. The common adoption boundary updates source/layer and tooltip
Cages before pumping their runtimes. Each runtime owns its whole Cage; reading
another runtime's Bond for diagnostics requires untracked reads rather than
subscribing an effect across runtimes. The same component revision retains its
scope and local state, even if a new factory closure is supplied. A new revision
requests a replacement.

## Native selection and placement

Actual owned body Enter/Move reserves tooltip selection before the application
hook. Leave clears only the matching occurrence/body lease; Click does not
establish or move a tooltip. No public dummy mouse handler is required. A DOM
event's `isTrusted` value is observed as delivered; connected owned synthetic
DOM events are not rejected solely for being untrusted.

Tooltip configuration selects the native-component activity policy unless the
application explicitly selected Owner policy in its mouse handlers. Under that
policy Enter queues current and presentation, Move updates only presentation,
and Leave queues clearing both. Manual current and tooltip selection are
independent. No fake scalar site, companion Mark or component click is promoted
to a native body hit.

Omitted Parts, removed bodies and hidden series cannot retain automatic
presentation. Literal repetitions report the actual occurrence. Native SVG
`visiblePainted` determines the original event target: this is not a solid-only
or nonzero-alpha pixel oracle, and transparent SVG paint may remain a native
pointer target. Borders, separators and native labels stay passive. Custom
layers above a body can intercept input. A source/program commit revalidates the
selection against the actual candidate plan and refreshes its generation and
publication. Pointer-only Move retains the SVG publication and source cohort.

Placement follows the actual cursor. Defaults are Top and a 14 local CSS-pixel
gap. The shared nine-anchor vocabulary includes Center and four fixed corners.
Requested Left/Right resolve to a side using the shared provider's horizontal
half-container rule; Top, Bottom, Center and corners stay fixed. This is not
viewport collision avoidance. Coordinates map each physical CSS axis back to
the parent independently, then round local coordinates and use natural-size
percentage translation. This supports axis-aligned nonuniform CSS scaling;
arbitrary rotated/skewed ancestors are outside this profile. The overlay is
separate from the clipped SVG content.

## Failure and lifecycle boundary

Source/layer and tooltip preparation, owned carrier placement and accepted data
share one transaction. A rejected builder, factory or precommit owned DOM write
keeps the preceding accepted configuration, source and tooltip; a failed desired
configuration must not poison subsequent native Move/Leave. Application callback
side effects cannot be rolled back.

Standard text uses no Glory application runtime. Custom tooltip runtimes share
the program's component budget with live, pending and retiring custom layers.
A retained component is counted once; replacing it temporarily counts both
records. After common data adoption, a managed backend/effect failure is a
postcommit UI error. The failed tooltip is hidden and inert, and remains latched
until replacement by a new revision or remount; accepted source data is not
represented as a rolled-back native backend transaction. Stop retires owned
presentation and weak handles reject further work.

This feature does not add Canvas tooltips, source-item components, annotations,
Spring animation, generic raw datum types or arbitrary mutable D3 generators.
