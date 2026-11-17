# Funnel property springs (Unreleased)

[简体中文](funnel-motion.zh-CN.md)

Funnel motion keeps the accepted business source separate from its displayed
Scene. `FunnelMotion` supplies an explicit clock for SVG, Canvas and raster;
the dedicated SVG browser owner adds automatic RAF and retained Glory component
subscriptions. Existing mounts keep their immediate behavior until motion is enabled.

## Pure motion and rendering

```rust
use kartis::*;

pub fn sample() -> Result<String> {
    let config = ChartConfig::default();
    let options = FunnelLayoutOptions::default();
    let first = FunnelSvgFrame::new(&config,
        &[FunnelDatum::new("a", 8.), FunnelDatum::new("b", 4.)], &options)?;
    let next = FunnelSvgFrame::new(&config,
        &[FunnelDatum::new("a", 5.), FunnelDatum::new("b", 7.)], &options)?;
    let mut motion = FunnelMotion::new(first, FunnelMotionOptions::default())?;
    motion.retarget(next)?;
    motion.tick(0.08)?;
    render::to_svg(motion.frame().scene())
}
```

Render `frame.scene()` for the actual sample; `frame.target().scene()` remains
the immutable target. Existing `render::draw`, `draw_canvas` and raster APIs can
consume that sample. Role and item mappings retain the target Scene slots.
Frames are owned and equality identifies one capture; historical samples remain
readable. The model retains no application factories and does not grant input
eligibility on its own. `tick` accepts finite intervals in 0..=60 seconds.
Options, budgets, retargeting and samples validate before replacing the model.

## Browser clocks and components

```rust
use kartis::*;
use kartis::browser::*;

pub fn mount(root: web_sys::Element) -> Result<BrowserFunnel> {
    let owner = BrowserFunnel::mount_with_svg_layers(root,
        vec![FunnelDatum::new("a", 8.), FunnelDatum::new("b", 4.)],
        ChartConfig::default(), FunnelLayoutOptions::default(),
        BrowserFunnelSvgProgram::defaults())?;
    owner.enable_motion(FunnelMotionOptions::default())?;
    owner.flush()?;
    Ok(owner)
}

pub fn deterministic_step(owner: &BrowserFunnel) -> Result<()> {
    owner.set_motion_clock(BrowserFunnelMotionClock::Manual)?;
    owner.flush()?;
    owner.step_motion(0.016)
}
```

Automatic RAF is the default when enabled. Manual stepping uses the supplied
seconds exactly and rejects invocation during an owner callback. Source builds
advance source generation; physics samples advance publication without replaying
business factories. View, literal layer order and hidden-series commands keep
the same accepted source. Hidden or zero-size surfaces suspend automatic time;
restoration starts a fresh wall-clock baseline rather than catching up.
`set_motion_options(None)` queues target presentation for the next successful
flush/adoption. Motion preference
can follow the system, request reduced motion, or explicitly animate.

```rust
use glory::Widget;
use kartis::*;
use kartis::browser::*;

pub fn add_status(program: BrowserFunnelSvgProgram) -> Result<BrowserFunnelSvgProgram> {
    program.custom_animated("status", 0, |context, motion| {
        Ok(glory::ui::widget(move |scope| {
            glory::ui::text().content(scope.bond(move || {
                let source = context.frame.get();
                let pose = motion.get();
                format!("source {}, sample {}",
                    source.as_ref().map_or(0, |f| f.source_generation),
                    pose.as_ref().map_or(0, |f| f.data.sample_serial()))
            })).show_in(scope)?;
            Ok(())
        }))
    })
}
```

Add a matching custom layer occurrence before using that registry. Its existing
source Bond and extra motion Bond derive from the same complete Cage.
`set_tooltip_component_with_motion` supplies the equivalent pair for retained
tooltips. `set_mouse_handlers_with_motion` adds the physical presentation captured
at native admission. Typed `BrowserFunnelData<D>` supports the same capabilities;
D stays on its accepted target frame and gains no new trait requirements.

The actual displayed body paths receive native input. A publication change
between capture and bubble invalidates that observation. Earlier event data and
motion certificates remain historical; tooltip selection follows the eligible
accepted source. Failed precommit candidates retain the accepted model, paint,
component state and focus. Application callback side effects remain external;
backend failures after adoption use the existing UI error isolation policy.

## Nivo behavior and boundaries

Body and border paths use separate 0→1 springs with D3 string interpolation:
on an interrupted path change, the old endpoint is the previous committed target,
not the currently displayed sample. The new path supplies command topology;
numeric tokens pair in order, with unmatched new tokens held at their target.
Style channels preserve their independent velocity. Part fill/border opacity,
formatted text and target border visibility update immediately.

Parts and labels retain typed business keys. Separators distinguish before,
after and terminal roles. Bound annotations use their binding index: outline
geometry, connector progress and built-in note position animate independently.
Custom note Scenes and unowned extras remain target-authored. New source parts
start at target and removed source parts disappear immediately. Removing and
readding a source key creates fresh tracks, with no exit transition.

Body and border tracks survive while their source key remains, including omitted
zero-width/zero-opacity borders and genuinely collapsed bodies. Dormant tracks
continue to receive targets, tick, settle and obey reduced motion. Restored paint
uses their retained style velocity; a changed path starts from its previous
committed target, including the captured collapsed target. No dormant track adds
a Scene Mark or input target. An isolated zero-size part with nonzero neighbors
need not be fully collapsed. See [dormant roles and custom capture](funnel-dormant-motion.md).

The Rust integrator uses analytic damped springs with explicit rest thresholds;
it does not run React Spring. Geometry and color are admitted to Scene limits,
and custom captured ink is animated directly rather than reconstructed from the
default profile. Arbitrary mutable D3 generators, item-widget mixing, arbitrary
live UI export and GPU/worker remain separate. Verification scope is recorded
in the [verification record](verification.md).
