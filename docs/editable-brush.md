# Editable brush

[简体中文](editable-brush.zh-CN.md)

`EditableBrush` owns a persistent selection in logical viewport pixels.
`EditableBrushOptions` selects X, Y, or XY and gives an explicit normalized
extent and handle size. The default extent is a unit rectangle; applications
must configure the intended plot extent. X/Y selections fill the inactive
dimension of that extent. `selection()` distinguishes no selection from the
temporary zero-span rectangle during creation.

The overlay creates a selection, the selection moves it, and its eight handles
resize the corresponding edges. Crossing an edge flips the active direction.
Alt adjusts about the center, Shift locks the dominant direction when both
axes are active, Meta forces a new selection, and Space temporarily moves the
current selection. Ctrl filters a new gesture. Modifiers can be disabled.

Begin returns its session through `session()`. Every Move, End, Cancel, and
modifier update carries that session; pointer events also carry their pointer
ID. An old session cannot continue after a programmatic set, reconfiguration,
clear, or a new controller. `Nudge` and `Clear` are checked user commands for
keyboard controls. Explicit cancellation restores the selection before Begin.

Events distinguish Start, Change, End, and Cancel, with user/programmatic
source, mode, selection, and session. Program updates retire the active
baseline before accepting a new value. An idle repeated value is a no-op,
which also lets callbacks submit an unchanged value without a notification
loop. Illegal coordinates, handles, or configuration preserve the old state.

`SurfaceModel::configure_editable_brush` enables the editor independently of
legacy Shift-drag creation when the editor is absent. While configured, the
editor owns creation inside its extent and outside gestures use ordinary
pan/click, avoiding an invisible second brush. The extent lies inside the plot.
`SurfaceEvent::EditableBrush` reports viewport selection and current data IDs.
IDs use accepted typed anchors, clipping, visibility, and one scene projection;
they are not computed from a custom Path's bounding-box center.
`editable_brush_viewport_request()` subtracts the plot origin once and chooses
the active axes for a plot-local domain request; no camera inverse is applied.
Browser handles can queue this with `request_editable_brush_viewport()`.

`BrushReconfigure::Preserve` rejects a selection outside the new extent;
`Clamp` constrains it, and `Clear` clears it. Plot shrink/movement clamps the
configured pixel extent and retires an affected gesture. Plot expansion does
not scale pixels or silently expand the extent; reconfigure explicitly. A
relayout may preserve the session only when its plot and dimensions remain
unchanged. Scene replacement and reset cancel it.
Changing series visibility also cancels an active session before a later
pointer-up can finish it; repeating the same visibility value is a no-op.

Browser SVG and Canvas use `BrowserOptions::editable_brush` and the corresponding
handle configuration/selection methods. Raster hosts use the same Surface
controller and explicitly supplied host bounds. Host capture, focus, keyboard
controls, publication, and cleanup are separate integration responsibilities;
the Rust state machine itself installs no listeners.

The Glory raster bridge provides named drag-capture regions and ordinary
Tab/Enter buttons for selecting a control, nudging, resetting, and clearing.
Applications can forward raw modifier changes through
`EditableBrushInput::Modifiers` when their native event source exposes them;
the browser adapter installs its modifier listeners. A native image-boundary
change during a drag cancels that gesture instead of reusing its old offset.

Recoverable browser DOM write failures restore the last accepted selection
and paint by restoring the model and republishing its complete DOM state.
A contact that is still down can retain its session; an already-ended
physical contact is retired while keeping the accepted selection. Explicit
Escape/owner cancellation instead follows the Begin-origin rollback policy.
If the host also prevents that recovery write, the adapter reports the host
error; it cannot guarantee physical DOM recovery from a persistently failing
DOM API.
This recovery covers the adapter's owned chart and brush DOM. User-supplied
Plot Widget factories and their external side effects keep their separate
error and ownership contract.

The geometry/modifier rules follow original d3-brush 3.0.0. This bounded Rust
profile deliberately differs for idle no-op notifications, session retirement,
and a zero inactive extent in single-axis mode. It also keeps the actual
clamped size when Alt is followed by Space: the original D3 edge case expands
an X selection beyond its extent. Ordinary equivalence fixtures and this
independently observed safety difference are recorded separately. The original
JavaScript oracle and complete third-party licenses are development fixtures,
not chart runtime dependencies. See [verification](verification.md) and
[third-party notices](third-party-notices.md).
