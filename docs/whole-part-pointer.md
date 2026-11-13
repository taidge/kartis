# Whole-part pointer picking

[简体中文](whole-part-pointer.zh-CN.md)

Use `BoxPlotInteraction::whole_parts_scene_interaction()` to opt into `PickingProfile::PaintedGeometry`. It retains one logical body site per computed box and maps each emitted border, median, whisker, cap, and outlier to that business identity. The existing `scene_interaction()` and `chart(scene)` entries keep their legacy body-site behavior.

A `SurfacePhysicalHit` distinguishes `physical_mark_id` from `logical_site_id`. Its optional `source_data` is an owned snapshot of an original outlier's ID and value. A box median is the logical site's value; it must not be substituted for an outlier's raw value. Do not reconstruct source data from an animated `Mark.value`, an ID suffix, or a later controller lookup.

The following complete program exercises the renderer-independent Surface protocol and mounts the original semantic Graphic chart in Glory's memory host. With the `raster` feature it also mounts the CPU-backed image and controls in a memory host. These operations do not simulate a physical device or launch a native window.

```rust
use kartis::{
    surface::{SurfaceEvent, SurfaceInput, SurfaceModel, SurfaceOptions},
    *,
};
use glory::render::{GraphicStroke, StrokeCap};
use std::{cell::RefCell, rc::Rc};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let config = ChartConfig {
        width: 480.,
        height: 300.,
        margin: Margin {
            left: 40.,
            right: 40.,
            top: 24.,
            bottom: 24.,
        },
        ..Default::default()
    };
    let mut data: Vec<_> = [0., 2., 4., 6., 8., 10.]
        .into_iter()
        .enumerate()
        .map(|(i, value)| BoxObservation::new(format!("sample-{i}"), "A", "x", Some(value)))
        .collect();
    let stroke = GraphicStroke {
        color: Color(20, 40, 60, 255),
        width: 6.,
        cap: StrokeCap::Butt,
    };
    let options = BoxPlotLayoutOptions {
        domain: Some([-2., 12.]),
        border: Some(stroke),
        median: stroke,
        whisker: stroke,
        cap_size: 0.5,
        outlier_radius: 5.,
        legend: false,
        value_axis: AxisOptions {
            visible: false,
            grid: false,
            ..Default::default()
        },
        group_axis: CategoryAxisOptions {
            visible: false,
            grid: false,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut controller = BoxPlotInteraction::new(
        &config,
        &data,
        &options,
        &BoxPlotInteractionOptions::default(),
    )?;
    let frame = controller.whole_parts_scene_interaction()?;
    let binding = frame.interaction.as_ref().expect("whole-parts binding");
    assert_eq!(binding.picking_profile, PickingProfile::PaintedGeometry);
    assert_eq!(binding.sites.len(), 1); // Parts do not become extra datums.
    let body_id = binding.sites[0].mark_id.clone();
    let physical_id = binding
        .part_data
        .iter()
        .find(|(_, datum)| datum.source_id == "sample-0")
        .expect("the original zero observation is an emitted outlier")
        .0
        .clone();
    let part = frame
        .scene
        .marks
        .iter()
        .find(|mark| mark.id == physical_id)
        .unwrap();
    let point = match part.geometry {
        Geometry::Circle { x, y, .. } => (x, y),
        _ => return Err(ChartError("expected the configured circle outlier".into()).into()),
    };
    let mut surface = SurfaceModel::new_with_interaction(
        frame.scene.clone(),
        SurfaceOptions {
            plot: frame.plot,
            zoom: false,
            pan: false,
            pinch: false,
            brush: false,
            ..Default::default()
        },
        frame.interaction.clone(),
    )?;
    let events = surface.apply(SurfaceInput::Move { id: 1, point })?;
    let old_hit = surface
        .hovered_physical_hit()
        .cloned()
        .expect("actual outlier ink");
    assert!(matches!(
        events.last(),
        Some(SurfaceEvent::PhysicalHover(Some(_)))
    ));
    assert_eq!(old_hit.physical_mark_id, physical_id);
    assert_eq!(old_hit.logical_site_id, body_id);
    assert_eq!(surface.hovered(), std::slice::from_ref(&body_id));
    assert_eq!(surface.physical_hit_at(point)?.as_ref(), Some(&old_hit));
    let original = old_hit
        .source_data
        .as_ref()
        .expect("accepted original source");
    assert_eq!(original.source_id, "sample-0");
    assert_eq!(original.value, 0.); // Never inferred from a live Mark.value.

    // Direct Surface input: contact lifecycle and explicit terminal cleanup.
    surface.apply(SurfaceInput::TouchStart { id: 2, point })?;
    surface.apply(SurfaceInput::TouchMove { id: 2, point })?;
    surface.apply(SurfaceInput::TouchEnd { id: 2, point })?;
    surface.apply(SurfaceInput::Leave)?;
    assert_eq!(surface.touch_count(), 0);
    assert!(!surface.dragging());
    assert!(surface.hovered().is_empty());
    assert!(surface.hovered_physical_hit().is_none());
    assert_eq!(surface.selected(), Some(body_id.as_str()));
    surface.apply(SurfaceInput::TouchStart { id: 3, point })?;
    surface.apply(SurfaceInput::TouchCancel { id: 3 })?;
    surface.apply(SurfaceInput::Leave)?;
    assert_eq!(surface.touch_count(), 0);

    data[0].value = Some(-1.);
    controller.replace_source(&config, &data, &options)?;
    let next = controller.whole_parts_scene_interaction()?;
    assert_eq!(
        next.interaction.as_ref().unwrap().part_data[&physical_id].value,
        -1.
    );
    surface.set_layout(next.clone())?;
    assert!(!surface.is_current_hit(&old_hit));
    assert_eq!(old_hit.source_data.as_ref().unwrap().value, 0.);
    let token = surface.frame_token();
    let mut invalid = next.clone();
    invalid
        .interaction
        .as_mut()
        .unwrap()
        .ink_owners
        .insert(physical_id, "unknown-owner".into());
    assert!(surface.set_layout(invalid).is_err());
    assert_eq!(surface.frame_token(), token);

    // This is a real semantic Graphic chart, with one atomic layout Cage.
    let app = glory::App::memory();
    let layout = app.scope().cage(next.clone());
    let observed = Rc::new(RefCell::new(Vec::<Option<SurfacePhysicalHit>>::new()));
    let pending = observed.clone();
    app.mount(
        chart_layout(layout)
            .highlight(false)
            .data_control_pointer(false)
            .on_physical_hover(move |hit| pending.borrow_mut().push(hit)),
    )?;
    app.flush()?;
    // Applications drain accepted hits outside the callback, update the
    // controller if the active owner changed, then publish a complete layout.

    #[cfg(feature = "raster")]
    {
        let raster_app = glory::App::memory();
        let raster = RasterSurface::mount_layout(
            raster_app.scope(),
            next,
            render::raster::FontRegistry::new(),
            RasterSurfaceOptions {
                highlight: false,
                ..Default::default()
            },
        )?;
        raster_app.mount(raster.widget())?;
        raster_app.flush()?;
        assert_eq!(raster.handle().frame()?.width, 480);
    }
    Ok(())
}
```

`Move` uses viewport logical coordinates. `physical_hit_at` is a pure query; it does not create a hover event. Strict picking checks the actual owned solid RGBA fill or stroke, stroke width, caps, dash runs, clipping, hidden series, and effective group opacity. A transparent body can retain a logical site when its median or other owned part paints. A completely transparent group has no physical hit, while its one logical keyboard control can remain available. No transparent hit mark or fabricated rectangular area is added.

Only `Picking::Geometry` is admitted for this profile. Nearest, slice, and polar interaction are rejected. Eligible physical marks with gradient or pattern fills are not supported by this first solid-paint profile. Curves use bounded stroke flattening, and clip paths retain their own checked approximation limits; this is not a claim of analytic picking for arbitrary vector paths.

Successful source, hidden, or view replacement invalidates earlier hit tokens. A rejected candidate retains the accepted frame and token. `source_data` in an old hit remains its old immutable snapshot, even if the replacement reuses the same physical ID. `SurfaceModel::is_current_hit` checks that model's accepted token. Applications must retire queued observations when they replace their own source, rather than applying an old same-ID observation to a new controller source.

For semantic charts, pass one `Cage<SceneInteraction>` to `chart_layout`; publish the Scene and binding together. This entry paints the original Graphic tree and requires the explicit strict binding. Its logical data buttons have pointer events disabled but remain keyboard/focus controls. The controls-only Scene projection is never painted. `Chart::data_control_pointer` also accepts a dynamic `Vary<bool>` for adapters that switch profiles; its legacy default is true. Physical observations arrive through `on_physical_hover`, including transitions between two parts of the same box. Keep callbacks short: queue immutable observations, then update `set_hovered_mark` and publish a complete layout outside the callback only when the active owner changes. The physical event is delivered after the operation's logical events.

BrowserChart accepts the same layout through its layout APIs and reports `SurfaceEvent::PhysicalHover` inside `BrowserEvent::Interaction`. RasterSurface uses `mount_layout`/`set_layout` and its existing `on_event` callback. Raster coordinates come from the actual image-host View, excluding the separate controls. The image, controls, binding and callback source are accepted together after validation and successful image publication. Strict Raster plus `editable_brush` is explicitly unsupported and rejects before publication; the existing legacy brush path is unchanged.

The Glory coordinate channel preserves fractional View-local coordinates and the same observation's viewport extent. Adapters map these once to Scene dimensions without DPR rounding. Mouse and pen use move/down/up; touch uses contact start/move/end/cancel, with terminal cleanup instead of persistent mouse hover. The direct Surface example spells out its terminal `Leave`. Keyboard activation still uses the logical site. Applications should select `highlight(false)`, `BrowserHighlight::Controlled`, or raster `highlight: false` when BoxPlot's controller owns association opacity; use reduced browser motion for immediate updates.

Interactive hosts must provide the finite `PlotPointer` capability. Static output records interest but cannot generate input. The Web channel supports positive axis-aligned 2D scale and translation; observations under rotation, skew, perspective or unsupported transform chains reject. The Apace channel currently rejects observations under any computed transform ancestor. Neither adapter guesses a transform inverse. Owner disposal retires the channel and callback; source updates do not turn stale detached nodes into new datums.

This guide describes bounded pointer input and controlled association. It does not claim physical-device certification, arbitrary platform transforms, deterministic compositor screenshots, GPU or worker execution. GPU and worker work remains deferred.
