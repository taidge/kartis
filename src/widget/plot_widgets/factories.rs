//! Bridges the legacy Mark-only factories and real mixed Scene factories.
use super::*;

fn mark_context(context: &PlotWidgetBuildContext) -> Result<PlotWidgetContext> {
    // Reject a Label before invoking an old user factory. The retained record
    // kind guard keeps this derived Bond Mark-only for its entire lifetime.
    context.base.frame.with(|frame| frame.mark_frame())?;
    let source = context.base.frame;
    let frame = context.scope.bond(move || {
        source.with(|frame| {
            frame
                .mark_frame()
                .expect("validated Mark component retains its source kind")
        })
    });
    Ok(PlotWidgetContext {
        mark_id: context.base.key.clone(),
        revision: context.base.revision,
        frame,
    })
}

pub(crate) fn plain_factory<W: Widget>(
    mut factory: impl FnMut(PlotWidgetContext) -> Result<W> + 'static,
) -> PlotWidgetFactory {
    Box::new(move |context| Ok(Box::new(factory(mark_context(&context)?)?)))
}

pub(crate) fn typed_factory<T: Clone + PartialEq + 'static, W: Widget>(
    mut factory: impl FnMut(TypedPlotWidgetContext<T>) -> Result<W> + 'static,
) -> PlotWidgetFactory {
    Box::new(move |context| {
        context.payload.with(|payload| payload.get::<T>())?;
        let base = mark_context(&context)?;
        let payload = context.payload;
        let frame = context.scope.bond(move || TypedPlotWidgetFrame {
            frame: base.frame.get(),
            data: payload
                .with(|payload| payload.get::<T>())
                .expect("validated accepted plot Widget payload"),
        });
        Ok(Box::new(factory(TypedPlotWidgetContext {
            mark_id: base.mark_id,
            revision: base.revision,
            frame,
        })?))
    })
}

pub(crate) fn composition_factory<T: Clone + PartialEq + 'static, W: Widget>(
    mut factory: impl FnMut(PlotWidgetCompositionContext<T>) -> Result<W> + 'static,
) -> PlotWidgetFactory {
    Box::new(move |context| {
        context.payload.with(|payload| payload.get::<T>())?;
        let base = mark_context(&context)?;
        let payload = context.payload;
        let frame = context.scope.bond(move || TypedPlotWidgetFrame {
            frame: base.frame.get(),
            data: payload
                .with(|payload| payload.get::<T>())
                .expect("validated composition payload"),
        });
        Ok(Box::new(factory(PlotWidgetCompositionContext {
            mark_id: base.mark_id,
            revision: base.revision,
            frame,
        })?))
    })
}

pub(crate) fn scene_composition_factory<T: Clone + PartialEq + 'static, W: Widget>(
    mut factory: impl FnMut(crate::SceneWidgetCompositionContext<T>) -> Result<W> + 'static,
) -> PlotWidgetFactory {
    Box::new(move |context| {
        context.payload.with(|payload| payload.scene_get::<T>())?;
        let base = context.base;
        let payload = context.payload;
        let frame = context.scope.bond(move || crate::TypedSceneWidgetFrame {
            frame: base.frame.get(),
            data: payload
                .with(|payload| payload.scene_get::<T>())
                .expect("validated Scene composition payload"),
        });
        Ok(Box::new(factory(crate::SceneWidgetCompositionContext {
            key: base.key,
            revision: base.revision,
            frame,
        })?))
    })
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use glory::render::{Event, EventKind, Props};

    use super::*;

    fn label_frame(text: &str, index: usize) -> crate::SceneWidgetFrame {
        let rect = LayoutRect {
            x: 20.,
            y: 30.,
            width: 80.,
            height: 20.,
        };
        crate::SceneWidgetFrame::Label(crate::SceneLabelWidgetFrame {
            label_index: index,
            label: crate::Label {
                rotation: 0.,
                x: 20.,
                y: 45.,
                text: text.into(),
                color: crate::Color(20, 40, 80, 255),
                size: 14.,
                anchor: crate::TextAnchor::Start,
            },
            owner: Some("original-node".into()),
            font: crate::ResolvedFont::default(),
            whitespace: crate::LabelWhitespace::default(),
            clip: None,
            rect,
            visible: Some(rect),
            interactive: true,
            transform: None,
        })
    }

    fn context(
        app: &glory::App,
        frame: Bond<crate::SceneWidgetFrame>,
        payload: Bond<SlotPayload>,
    ) -> PlotWidgetBuildContext {
        PlotWidgetBuildContext {
            base: crate::SceneWidgetContext {
                key: "label:original-node".into(),
                revision: 0,
                frame,
            },
            payload,
            scope: app.scope().clone(),
        }
    }

    #[test]
    fn old_mark_factories_reject_real_label_frame_before_application_callback() {
        let app = glory::App::memory();
        let source = app.scope().cage(label_frame("real label", 0));
        let frame = app.scope().bond(move || source.get());
        let data = app.scope().cage(SlotPayload::new(7u32));
        let payload = app.scope().bond(move || data.get());
        let calls = Rc::new(Cell::new(0));
        let plain_calls = calls.clone();
        let typed_calls = calls.clone();
        let composition_calls = calls.clone();
        let factories = [
            plain_factory(move |_| {
                plain_calls.set(plain_calls.get() + 1);
                Ok(glory::ui::button())
            }),
            typed_factory(move |_: TypedPlotWidgetContext<u32>| {
                typed_calls.set(typed_calls.get() + 1);
                Ok(glory::ui::button())
            }),
            composition_factory(move |_: PlotWidgetCompositionContext<u32>| {
                composition_calls.set(composition_calls.get() + 1);
                Ok(glory::ui::button())
            }),
        ];
        for mut factory in factories {
            let error = match factory(context(&app, frame, payload)) {
                Ok(_) => panic!("a Mark-only factory accepted a Label source"),
                Err(error) => error,
            };
            assert_eq!(error.0, "Mark component received a label target");
        }
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn scene_payload_envelope_is_not_interchangeable_with_legacy_typed_payload() {
        let app = glory::App::memory();
        let source = app.scope().cage(label_frame("actual label", 0));
        let frame = app.scope().bond(move || source.get());
        let old = app.scope().cage(SlotPayload::new(7u32));
        let old_payload = app.scope().bond(move || old.get());
        let scene = app.scope().cage(SlotPayload::scene_new(7u32));
        let scene_payload = app.scope().bond(move || scene.get());
        let calls = Rc::new(Cell::new(0));
        let seen = calls.clone();
        let mut factory =
            scene_composition_factory(move |_: crate::SceneWidgetCompositionContext<u32>| {
                seen.set(seen.get() + 1);
                Ok(glory::ui::button())
            });
        let wrong_scene = match factory(context(&app, frame, old_payload)) {
            Ok(_) => panic!("Scene factory accepted a legacy payload envelope"),
            Err(error) => error,
        };
        assert!(wrong_scene.0.contains("metadata type mismatch"));
        let seen = calls.clone();
        let mut old_factory = typed_factory(move |_: TypedPlotWidgetContext<u32>| {
            seen.set(seen.get() + 1);
            Ok(glory::ui::button())
        });
        let wrong_legacy = match old_factory(context(&app, frame, scene_payload)) {
            Ok(_) => panic!("legacy factory accepted a Scene payload envelope"),
            Err(error) => error,
        };
        assert!(wrong_legacy.0.contains("metadata type mismatch"));
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn scene_factory_real_child_reads_live_label_and_payload_without_remounting() {
        let app = glory::App::memory();
        let source = app.scope().cage(label_frame("first", 0));
        let frame = app.scope().bond(move || source.get());
        let data = app.scope().cage(SlotPayload::scene_new(7u32));
        let payload = app.scope().bond(move || data.get());
        let calls = Rc::new(Cell::new(0));
        let observed = calls.clone();
        let mut factory =
            scene_composition_factory(move |context: crate::SceneWidgetCompositionContext<u32>| {
                observed.set(observed.get() + 1);
                assert_eq!(context.key, "label:original-node");
                let frame = context.frame;
                Ok(glory::ui::widget(move |scope| {
                    let count = scope.cage(0u32);
                    let focused = scope.cage(false);
                    glory::ui::button()
                        .name("actual-live-label")
                        .label(scope.bond(move || {
                            frame.with(|frame| {
                                let crate::SceneWidgetFrame::Label(label) = &frame.frame else {
                                    panic!("actual Label frame expected");
                                };
                                format!(
                                    "{}:{}:{}:{}:{}",
                                    label.label.text,
                                    label.label_index,
                                    frame.data,
                                    count.get(),
                                    focused.get()
                                )
                            })
                        }))
                        .on(EventKind::Activate, move |_| count.revise(|v| *v += 1))
                        .on(EventKind::FocusChanged, move |event| {
                            if let Event::FocusChanged(value) = event {
                                focused.revise(|v| *v = value);
                            }
                        })
                        .show_in(scope)?;
                    Ok(())
                }))
            });
        app.mount(factory(context(&app, frame, payload)).unwrap())
            .unwrap();
        let id = app.tree().roots()[0];
        let label = || {
            let tree = app.tree();
            let Props::Button(button) = tree.node(id).unwrap().descriptor.view().props else {
                panic!("real Button expected");
            };
            button.label.clone()
        };
        assert_eq!(label(), "first:0:7:0:false");
        app.dispatch(id, Event::Activate).unwrap();
        app.dispatch(id, Event::FocusChanged(true)).unwrap();
        source.revise(|value| *value = label_frame("reordered", 9));
        data.revise(|value| *value = SlotPayload::scene_new(8u32));
        app.flush().unwrap();
        assert_eq!(app.tree().roots(), &[id]);
        assert_eq!(label(), "reordered:9:8:1:true");
        assert_eq!(calls.get(), 1);
    }
}
