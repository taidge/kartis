# SVG and Canvas browser controller fixture

This independent consumer workspace mounts two real `BrowserChart` controllers,
with application-owned surfaces and real Glory legend/data controls. Both scenes
include a 601-command decorative path, beyond the semantic Graphic path budget.

```powershell
./build.ps1
python -m http.server 8814 --bind 127.0.0.1
```

The build reads this workspace's `cargo metadata` and uses the actual artifact
directory. It inherits normal Cargo and temporary paths; generated WASM/browser
bindings live in ignored `pkg/`. Open `http://localhost:8814`.

The page has mode buttons for whole-scene transforms, fixed-axis data navigation,
category windows, independent X/Y axes, explicit shape morphs, custom legend/
tooltip styling and tooltip obstacle placement, plus animation and view reset. The time-zone
samples panel is computed by the actual WASM formatter. This consumer explicitly
enables `web` and `timezone`; applications can choose them independently.

For consumer applications, enable Kartis's `web` feature and mount only on
`wasm32`. Reserve an empty DOM element exclusively for the controller, for
example `<div id="chart"></div>`. Give that element a measurable width. A root
already claimed by another Glory host cannot be reused for this mount.

`BrowserChart` owns the chart's lifetime. Keep the returned guard; a temporary
such as `BrowserChart::mount(root, scene, options)?;` immediately drops the chart
at the end of the statement. `BrowserHandle` is weak: cloning or storing handles
does not retain the mounted chart. This complete single-chart ownership pattern
also handles replacing a previously mounted chart:

```rust
use kartis::{ChartConfig, Color, Geometry, Margin, Mark, Result, Scene};
use kartis::browser::{BrowserChart, BrowserHandle, BrowserOptions, BrowserRenderer};
use std::cell::RefCell;

thread_local! {
    static CHART: RefCell<Option<BrowserChart>> = const { RefCell::new(None) };
}

fn layout(width: f64, height: f64) -> Result<Scene> {
    let mut scene = ChartConfig {
        width,
        height,
        margin: Margin {
            top: height * 0.05,
            right: width * 0.05,
            bottom: height * 0.10,
            left: width * 0.05,
        },
        ..Default::default()
    }.scene()?;
    let mut mark = Mark::new(
        "sales",
        Geometry::Rect {
            x: width * 0.20, y: height * 0.20,
            width: width * 0.60, height: height * 0.50,
        },
        Color(65, 120, 185, 255),
    );
    mark.series = "sales".into();
    mark.label = "Sales".into();
    mark.value = 42.;
    scene.marks.push(mark);
    Ok(scene)
}

fn mount_chart(root: web_sys::Element) -> Result<BrowserHandle> {
    unmount_chart();
    let chart = BrowserChart::mount(root, layout(640., 300.)?, BrowserOptions {
        renderer: BrowserRenderer::Canvas, // BrowserRenderer::Svg also works.
        height: Some(300.),
        ..Default::default()
    })?;
    chart.on_resize(layout);
    let handle = chart.handle();
    CHART.with(|slot| *slot.borrow_mut() = Some(chart));
    Ok(handle)
}

fn unmount_chart() {
    let previous = CHART.with(|slot| slot.borrow_mut().take());
    drop(previous); // Drop outside the storage borrow.
}
```

An existing Glory owner can use `scope.retain(chart)?` instead of thread-local
storage. Keep a cloned weak handle for later commands; disposing that owner
drops the guard and cancels the controller. The chart's DOM mount must still be
reserved for this controller, rather than simultaneously managed as another
host's child content.

Register `on_event` and `on_resize` before transferring the guard to its owner.
Each registration replaces the previous callback. Callbacks run in a frame,
outside native listeners and controller state borrows. Capture a cloned
`BrowserHandle` in callbacks; capturing a strong `Rc<BrowserChart>` can create an
ownership cycle. Stopping through a weak handle from a callback is supported.
Other callback-driven updates should use the queued handle methods; calling
`flush` from inside a frame callback is rejected as reentrant.

`on_resize` requests an initial layout pass and subsequent passes when the plot's
CSS size changes. Its width and height describe the plot only, excluding Glory
controls, and are independent of device pixel ratio. Returning a newly computed
scene makes ticks and geometry responsive; without this callback the original
scene is scaled to the available surface. `height: Some(300.)` fixes the plot's
CSS height; `None` uses the initial scene's aspect ratio. The callback should
read current application data. Data changes alone do not trigger a resize:
compute a new scene and call `handle.set_scene(scene)` or
`handle.animate_to(scene)`. Successful relayout cancels any old animation.

Handle setters enqueue work and return admission errors; `Ok(())` does not mean
the next frame has already painted. `snapshot()` reads the last processed model.
Later layout, rendering or animation failures arrive through
`BrowserEvent::Error`. `animate_to` drives RAF automatically and honors
`BrowserOptions.motion` (system preference by default); no application polling
loop is required. The default `BrowserAnimation::Properties(...)` uses independent
physical springs, preserving compatible property velocities when retargeted.
`BrowserAnimation::Scene` retains shared-progress scene springs. Both modes
animate label rotation; these fixtures rotate captions from -15 to 30 degrees.
`snapshot().animation_kind` identifies a running engine.
`flush` is available for explicit host turns and verification.
Programmatic `SurfaceInput` points use logical viewport coordinates before the
view transform, not CSS client coordinates or already unprojected data points.

The surface accepts two touch pointers, including a nonprimary second finger.
Pinch preserves the world point under the initial centroid while its movement
pans the view. Lifting one finger rebases single-finger pan; that gesture never
activates a data point as a tap. A third contact is ignored. Set
`BrowserOptions.interaction.pinch` to false to keep single-touch behavior;
`zoom` and `pan` continue controlling those operations separately.
Lost capture, cancel, viewport resize (including rejected/no layout callbacks),
hiding, stop and Drop retire contacts and release captures. Snapshots expose
`touches`, `pinching` and `captured_touches` for host diagnostics.

`stop()` is idempotent and removes owned DOM, listeners and pending frames. It
does not resume the same controller. After stop or Drop, use
`handle.is_active()` to check liveness; `snapshot()` and update methods return an
error rather than an inactive snapshot. Mount a fresh guard on the emptied root
and obtain a new handle when remounting.

The page exposes `window.hostFixture` after WASM startup and sets
`window.hostReady`. Hooks accept `"svg"` or `"canvas"`:

- `snapshot`, `events`: JSON inspection of view coordinates, current selection,
  hidden series, resize calls, active animation, frame revision, bounded input
  queue and events.
- `set_phase(id, phase, animate)`: publish or animate between two layouts.
- `reset_view`, `flush`, `reject_scene`: exercise queued updates and rejection.
- `stop`, `drop_chart`, `remount`: verify ownership and listener retirement.
- `remount_scene_animation`: select the legacy shared-progress engine for
  explicit mode acceptance; ordinary remounts use default property springs.
- `arm_stop_on_hover`, `arm_stop_on_resize`: stop through weak handles inside
  application callbacks, without retaining the controller.
- `overflow_actions`, `clear_events`, `install_replacing_layout`: exercise
  bounded admission, event inspection and reentrant callback replacement.
- `dispatch_control`: exercise typed programmatic input and animation
  interruption without bypassing the controller's input queue.

`scripts/host-browser-acceptance.js` is a reusable Playwright page function for
real pointer, wheel, keyboard, brush, resize, animation, and cleanup acceptance.
It also verifies failed RAF admission, lost capture, hidden-host cancellation,
multiple primary pointer types, malformed wheel input and dynamic reduced motion.
The fixture uses only local deterministic data and requires no network services.

The CLI also loads `scripts/touch-browser-acceptance.js` as its third function
argument. It uses Chrome's touch injection to verify real primary/nonprimary
PointerEvents, both captures, pinch anchors, handoff to one finger, tap
suppression, third contacts, resize failure paths and active stop/Drop cleanup.
This is browser-level emulation, not physical touchscreen hardware acceptance.

The fourth argument is `scripts/viewport-browser-acceptance.js`. It exercises
actual domain relayout, fixed axes, hit testing, resize, rejected final Up/TouchEnd,
callback replacement/stop, presentation CSS and IANA samples.
Use `BrowserHandle::viewport_controller()` to copy the accepted controller when
replacing a callback; reconstructing it from rounded pixels can lose a last bit.
`refresh_viewport()` requests the current view with fresh application data.
The Rust data-layout callback is in `src/lib.rs` (`viewport_scene` and
`install_viewport_binding`).

The fifth argument is `scripts/svg-browser-acceptance.js`. It verifies retained
SVG root/mark/label identity through animation, zero subtree mutations for an
unchanged frame, path preservation across clip/topology changes, paint order,
retirement and remount. BrowserChart no longer replaces its SVG on every frame.
The time-zone panel also runs actual local-calendar ticks in WASM, including a
23-hour day and two distinct UTC instants for the repeated 01:00 hour.

The example navigation also exposes **Tree styles**, **Parallel axes** (both
orientations), and **Drag network nodes**. Switching examples stops their prior
owners and restores the ordinary panels on return. The Network demo uses the
actual `BrowserNetwork` adapter; drag a node, pan empty space or wheel to zoom.
Select the example again to restart its initial graph.

Independent Rust fixtures live in `src/browser/{tree,parallel,network_drag}.rs`.
The CLI `--host` includes their pixel/input/lifecycle verifiers plus actual
navigation checks; the portable cross-browser runner reuses the same assertions.
Network replacement uses `BrowserNetworkHandle::replace_simulation` and preserves
the accepted common node identity without remounting. Input is a bounded FIFO,
physics publication is transactional, and zero-size containers pause and resume.

For separate OS/WASM capacity sampling, see
[`memory-measurement.md`](../../docs/memory-measurement.md). The fixture exposes
the actual initialized WASM memory to the measurement script; this does not add
a production memory API or treat retained linear capacity as live Rust heap.
