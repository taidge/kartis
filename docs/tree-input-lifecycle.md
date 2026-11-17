# Tree pointer input lifecycle

[中文](tree-input-lifecycle.zh-CN.md)

This development API is not included in published Kartis 0.1.0. See the
[Tree activity guide](tree-activity.md) for the complete application and the
four activity modifiers.

`BrowserTree` reserves a primary mouse/pen press before processing chart input.
When movement and pressing arrive before the same animation frame, the latest
automatic activity request waits until the native click has finished. The old
source, pointer target and accepted typed children stay together during that
press. Chart Down/Move/Up events and ordinary Glory UI events continue to run.
No layout factories are replayed merely because the button is still held.

The shared chart also postpones automatic series/crosshair painting that would
replace a pending click target. Real source panning and brushing can continue
painting after the movement threshold is crossed; this is not a pause of the
whole UI or gesture model. Widget presses retain their native parents until
their click finishes. A post-input task releases an Up without a generated
click, and a generation token prevents an old task from ending a newer press.

A quick click can therefore complete against the previous activity frame,
followed by the latest deferred current-node update. Nearest activity remains
separate from strict painted chart picking. Programmatic chart dispatch does
not pretend to establish a native press or a nearest-node move.

## Explicit requests and errors

Source, configuration, Tree options and Widget revision changes remain explicit
candidate requests. Once such a request is accepted, the previous press and its
deferred hover are cancelled. An old Widget click cannot activate newly accepted
data. A new primary press starts a new sequence; keyboard activation remains an
independent input.

Static validation, builder, child construction and precommit paint failures keep
the old accepted frame. A still-captured source contact can finish its original
click after a rejected candidate. An already released/cancelled contact cannot
be revived by rollback. Desired requests are retained: an explicit refresh or a
later activity request can retry them, without an idle retry loop.

This boundary preserves accepted chart data and click ownership. Ordinary UI
presentation can still update: rollback may redisplay the previous source's
hover tooltip or repaint that source with its current shared Series highlight.
The latter is checked against a separately mounted old Scene with the same
public highlight policy and logical hover. Complete DOM or backing-image bytes
can therefore differ from the earlier held presentation.

After typed data has been committed, errors from ordinary dynamic child UI
bindings are reported on that accepted frame. They do not undo data that user
effects have already observed. Overlay and source-order hosts use this same
boundary. See [composition](widget-composition.md) and
[source-order composition](ordered-widgets.md).

## Lifetime and policy

Matching cancellation, capture loss and window blur retire the active contact.
One inactive pointer/generation identity can remain to suppress that cancelled
contact's late click; it owns no DOM references or active animation loop.
Matching Up, the next real primary press or stop removes it. Stop/drop clears
all listeners, release tasks, pending intent and chart resources.

`BrowserTreeOptions::default()` uses `BrowserHighlight::Controlled`. Explicit
`Series` policy is honored. `BrowserTree::highlight()` and the corresponding
handle method return the effective shared-chart policy.

The input domain includes the exact owner's Surface and its sibling overlay
Widget layer. Moving between them is not an owner leave. A nested chart or
another mounted chart owns its own input; a class name alone does not confer
ownership.

Focus delivered synchronously during an Engine frame preserves the latest
native pointer state, including focus restored while retained children move.
The delivery boundary is captured before the queued event is processed.
Outside-frame Widget focus keeps its existing shared-chart hover behavior.
Programmatic focus remains separate from Tree current-node input.

Revision replacement may synchronously blur an old focused child while its
DOM node is being removed. Such a retiring scope is no longer an accepted row,
even if that blur callback briefly observes it as connected. Its revision and
prior identity distinguish it from accepted children; the new accepted rows
must still agree with the owner. There is no guarantee that every synchronous
DOM observer sees only the final number of connected buttons.

During same-revision adoption, a synchronous focus event can also observe old
DOM text before ordinary UI bindings update it. Actual typed Frame Bonds
already contain the adopted data. Settled DOM reads and live Bond reads are
verified separately; every native event is not a DOM-text transaction boundary.

Touch does not drive Tree current-node activity. DOM-injected cancel/blur tests
are protocol evidence, not proof of OS touch or window-focus delivery. These
finite input guarantees do not establish arbitrary browser/OS event scheduling,
full-image equality, automatic Spring/exit animation or live Widget export.
GPU/worker remain deferred.
