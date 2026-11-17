use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use kartis::*;
// Intentionally no Clone/PartialEq/Debug/Serialize, and Rc<Cell> is not Send/Sync.
struct Raw {
    status: &'static str,
    metric: f64,
    token: Rc<Cell<usize>>,
}
fn config() -> ChartConfig {
    ChartConfig {
        width: 420.,
        height: 300.,
        margin: Margin {
            top: 20.,
            right: 24.,
            bottom: 28.,
            left: 36.,
        },
        title: "Raw Funnel / 原始".into(),
        ..Default::default()
    }
}
fn options() -> FunnelLayoutOptions {
    FunnelLayoutOptions {
        size: FunnelSize::DatumPixels,
        labels: None,
        legend: false,
        before: Some(FunnelSeparatorOptions {
            length: 18.,
            ..Default::default()
        }),
        after: Some(FunnelSeparatorOptions {
            length: 18.,
            ..Default::default()
        }),
        ..Default::default()
    }
}
fn rows(metric: f64) -> Vec<FunnelDataDatum<Raw>> {
    [
        ("hot", CategoryKey::String("1".into()), 8., 120.),
        ("cold", CategoryKey::Number(1.), 4., 80.),
        ("hot", CategoryKey::String("zero".into()), -0., 0.),
    ]
    .into_iter()
    .map(|(status, id, value, size)| {
        let mut projected = FunnelDatum::new(id, value);
        projected.size = Some(size);
        FunnelDataDatum::new(
            Raw {
                status,
                metric,
                token: Rc::new(Cell::new(0)),
            },
            projected,
        )
    })
    .collect()
}
fn frame() -> FunnelDataFrame<Raw> {
    FunnelDataFrame::new(&config(), rows(-0.), &options()).unwrap()
}
#[test]
fn no_raw_trait_bounds_and_old_projection_send_sync_are_preserved() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<FunnelSvgFrame>();
    let frame = frame();
    let cloned = frame.clone();
    assert_eq!(frame, cloned);
    assert!(format!("{frame:?}").contains("source_len"));
    assert_eq!(frame.data(0).unwrap().metric.to_bits(), (-0.0f64).to_bits());
    assert!(frame.data(100).is_none());
    assert_eq!(frame.source()[1].projected.id, CategoryKey::Number(1.));
    let borrowed = String::from("borrowed nonstatic data");
    let pure = FunnelDataFrame::new(
        &config(),
        vec![FunnelDataDatum::new(
            borrowed.as_str(),
            FunnelDatum::new("a", 1.),
        )],
        &options(),
    )
    .unwrap();
    assert_eq!(pure.data(0), Some(&borrowed.as_str()));
}
#[test]
fn raw_only_changes_and_same_shared_source_rebuilds_are_new_captures() {
    let a = frame();
    let b = FunnelDataFrame::new(&config(), rows(0.), &options()).unwrap();
    assert_ne!(a, b);
    assert!(!a.shares_source(&b));
    assert_eq!(a.scene(), b.scene());
    assert_eq!(
        render::to_svg(a.scene()).unwrap(),
        render::to_svg(b.scene()).unwrap()
    );
    assert_ne!(
        a.data(0).unwrap().metric.to_bits(),
        b.data(0).unwrap().metric.to_bits()
    );
    let source: Arc<[_]> = rows(-0.).into();
    let a = FunnelDataFrame::from_shared(&config(), source.clone(), &options()).unwrap();
    let b = FunnelDataFrame::from_shared(&config(), source, &options()).unwrap();
    assert_ne!(a, b);
    assert!(a.shares_source(&b));
    assert!(std::ptr::eq(a.data(0).unwrap(), b.data(0).unwrap()));
}
#[test]
fn typed_size_style_stage_functions_have_full_raw_source_and_run_once_in_original_order() {
    let source: Arc<[_]> = rows(-0.).into();
    let log = Rc::new(RefCell::new(Vec::new()));
    let events = log.clone();
    let mut size = move |c: FunnelDataSizeContext<'_, Raw>| {
        assert_eq!(c.source.len(), 3);
        assert!(std::ptr::eq(
            c.data,
            &c.source[c.computed.source_index].data
        ));
        events
            .borrow_mut()
            .push(format!("size{}", c.computed.source_index));
        Ok(c.computed.datum.size.unwrap())
    };
    let events = log.clone();
    let mut color = move |c: &FunnelDataColorContext<'_, Raw>| {
        assert_eq!(c.source.len(), 3);
        assert!(std::ptr::eq(
            c.data,
            &c.source[c.computed.source_index].data
        ));
        events
            .borrow_mut()
            .push(format!("color{}", c.computed.source_index));
        Ok(if c.data.status == "hot" {
            Color(200, 30, 50, 255)
        } else {
            Color(20, 70, 180, 255)
        })
    };
    let events = log.clone();
    let mut format = move |value: f64| {
        events.borrow_mut().push("format".into());
        Ok(format!("{:016x}", value.to_bits()))
    };
    let events = log.clone();
    let mut border = move |c: &FunnelDataStyleContext<'_, Raw>| {
        assert_eq!(c.source.len(), 3);
        events
            .borrow_mut()
            .push(format!("border{}", c.computed.part.source_index));
        Ok(Color(10, 20, 30, 255))
    };
    let events = log.clone();
    let mut label = move |c: &FunnelDataStyleContext<'_, Raw>| {
        assert_eq!(c.computed.border_color, Color(10, 20, 30, 255));
        assert_eq!(c.data.metric.to_bits(), (-0.0f64).to_bits());
        events
            .borrow_mut()
            .push(format!("label{}", c.computed.part.source_index));
        Ok(Color(80, 90, 100, 255))
    };
    let events = log.clone();
    let mut stage = move |c: &FunnelDataStageContext<'_, Raw>| {
        assert_eq!(c.source.len(), 3);
        assert!(std::ptr::eq(
            c.data,
            &c.source[c.computed.source_index].data
        ));
        events
            .borrow_mut()
            .push(format!("stage{}", c.computed.source_index));
        Ok(c.computed.inherit(c.computed.geometry.clone()))
    };
    let data = FunnelDataFrame::with_shared_factories(
        &config(),
        source,
        &options(),
        &mut FunnelDataFactories {
            color: Some(&mut color),
            formatted_value: Some(&mut format),
            border_color: Some(&mut border),
            label_color: Some(&mut label),
            size: Some(&mut size),
            stage: Some(&mut stage),
        },
    )
    .unwrap();
    assert_eq!(
        &*log.borrow(),
        &[
            "size0", "size1", "size2", "color0", "format", "border0", "label0", "color1", "format",
            "border1", "label1", "color2", "format", "border2", "label2", "stage0", "stage1"
        ]
    );
    assert_eq!(data.computed().styles()[2].color, Color(200, 30, 50, 255));
    assert_eq!(
        data.computed().styles()[2].label_color,
        Some(Color(80, 90, 100, 255))
    );
    assert!(data.computed().body_mark(2).unwrap().is_none());
}
#[test]
fn borrowed_callbacks_do_not_escape_the_result_and_empty_factories_preserve_legacy_bytes() {
    let calls = Cell::new(0);
    let data = {
        let mut color = |c: &FunnelDataColorContext<'_, Raw>| {
            calls.set(calls.get() + 1);
            Ok(c.computed.default_color)
        };
        FunnelDataFrame::with_factories(
            &config(),
            rows(-0.),
            &options(),
            &mut FunnelDataFactories {
                color: Some(&mut color),
                ..Default::default()
            },
        )
        .unwrap()
    };
    assert_eq!(calls.get(), 3);
    assert_eq!(data.data(0).unwrap().status, "hot");
    let source = rows(-0.);
    let projection = source
        .iter()
        .map(|d| d.projected.clone())
        .collect::<Vec<_>>();
    let plain = FunnelSvgFrame::new(&config(), &projection, &options()).unwrap();
    let typed = FunnelDataFrame::new(&config(), source, &options()).unwrap();
    assert_eq!(plain.scene(), typed.scene());
    assert_eq!(
        render::to_svg(plain.scene()).unwrap(),
        render::to_svg(typed.scene()).unwrap()
    );
    assert_eq!(
        render::to_canvas_script(plain.scene(), "canvas").unwrap(),
        render::to_canvas_script(typed.scene(), "canvas").unwrap()
    );
}
#[test]
fn annotation_predicates_and_scene_notes_borrow_complete_raw_and_computed_cohort_once() {
    let data = frame();
    let events = Rc::new(RefCell::new(Vec::new()));
    let seen = events.clone();
    let mut matcher = move |c: FunnelDataAnnotationMatchContext<'_, Raw>| {
        assert_eq!(c.computed.collection.len(), 3);
        assert_eq!(c.frame.source().len(), 3);
        assert_eq!(c.frame.computed(), c.computed.frame);
        assert!(std::ptr::eq(
            c.data,
            c.frame.data(c.computed.source_index).unwrap()
        ));
        seen.borrow_mut()
            .push(format!("m{}", c.computed.source_index));
        Ok(c.data.status == "hot")
    };
    let seen = events.clone();
    let mut note = move |c: FunnelDataAnnotationNoteContext<'_, Raw>| {
        assert_eq!(c.frame.computed(), c.computed.frame);
        assert_eq!(c.frame.computed().parts().len(), 3);
        assert_eq!(c.data.metric.to_bits(), (-0.0f64).to_bits());
        seen.borrow_mut()
            .push(format!("n{}", c.computed.binding.source_index));
        let mut scene = c.computed.scene()?;
        scene.marks.push(Mark::new(
            "raw-note",
            Geometry::Rect {
                x: c.computed.x,
                y: c.computed.y,
                width: 14.,
                height: 8.,
            },
            Color(40, 90, 170, 255),
        ));
        Ok(scene)
    };
    let spec = FunnelAnnotationSpec::new(
        FunnelAnnotationMatch::Predicate("hot".into()),
        FunnelAnnotationKind::Dot,
        FunnelAnnotationNote::Custom("geometry".into()),
        FunnelAnnotationPosition::Relative(24.),
        FunnelAnnotationPosition::Relative(-10.),
    );
    let annotated = data
        .with_annotations(
            &[spec],
            &mut FunnelDataAnnotationFactories {
                matcher: Some(&mut matcher),
                note: Some(&mut note),
            },
            &Default::default(),
        )
        .unwrap();
    assert_eq!(&*events.borrow(), &["m0", "m1", "m2", "n0", "n2"]);
    assert!(annotated.shares_source(&data));
    assert_ne!(annotated, data);
    assert_eq!(
        annotated.computed().annotations().unwrap().bindings().len(),
        2
    );
    assert_eq!(annotated.annotation_data(1).unwrap().status, "hot");
    assert!(std::ptr::eq(
        annotated.annotation_data(1).unwrap(),
        data.data(2).unwrap()
    ));
    let cleared = annotated
        .with_annotations(&[], &mut Default::default(), &Default::default())
        .unwrap();
    assert!(cleared.shares_source(&data));
    assert_eq!(cleared.scene().marks, data.scene().marks);
    assert_eq!(
        cleared.computed().annotations().unwrap().bindings().len(),
        0
    );
}
#[test]
fn typed_legacy_role_widgets_keep_raw_frame_and_closing_separator_none() {
    let data = frame();
    let token = data.data(0).unwrap().token.clone();
    let before = Rc::strong_count(&token);
    let closing = Cell::new(0);
    let packet = data
        .widgets(3, |c| {
            assert_eq!(c.frame, &data);
            match c.computed.source_index {
                Some(i) => assert!(std::ptr::eq(c.data.unwrap(), data.data(i).unwrap())),
                None => {
                    assert!(c.data.is_none());
                    assert!(c.computed.separator.unwrap().part_id.is_none());
                    closing.set(closing.get() + 1);
                }
            }
            Ok(Some(PlotWidgetPlacement::Rect(LayoutRect {
                x: 1.,
                y: 2.,
                width: 8.,
                height: 8.,
            })))
        })
        .unwrap();
    assert!(closing.get() > 0);
    assert_eq!(
        Rc::strong_count(&token),
        before,
        "payload cloning must clone Arc frames, not D or its fields"
    );
    assert!(!packet.data.is_empty());
    for row in packet.data.values() {
        assert_eq!(row.frame(), &data);
        if row.computed().source_index.is_none() {
            assert!(row.data().is_none());
        }
    }
    let clone = packet.data.values().next().unwrap().clone();
    assert_eq!(clone.frame(), &data);
    assert!(format!("{clone:?}").contains("FunnelDataWidgetData"));
}
#[test]
fn invalid_projected_input_preflights_before_every_typed_factory() {
    let calls = Cell::new(0);
    let mut input = rows(-0.);
    input[2].projected.value = f64::NAN;
    let mut size = |_: FunnelDataSizeContext<'_, Raw>| {
        calls.set(calls.get() + 1);
        Ok(1.)
    };
    let mut color = |_: &FunnelDataColorContext<'_, Raw>| {
        calls.set(calls.get() + 1);
        Ok(Color(1, 2, 3, 255))
    };
    assert!(
        FunnelDataFrame::with_factories(
            &config(),
            input,
            &options(),
            &mut FunnelDataFactories {
                size: Some(&mut size),
                color: Some(&mut color),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(calls.get(), 0);
}
#[test]
fn constructor_and_annotation_rejections_cannot_replace_existing_raw_or_projection() {
    let data = frame();
    let previous = data.clone();
    let pixels = render::to_svg(data.scene()).unwrap();
    let calls = Cell::new(0);
    let mut reject = |_: &FunnelDataColorContext<'_, Raw>| {
        calls.set(calls.get() + 1);
        Err(ChartError("authored raw color rejection".into()))
    };
    assert!(
        FunnelDataFrame::with_factories(
            &config(),
            rows(0.),
            &options(),
            &mut FunnelDataFactories {
                color: Some(&mut reject),
                ..Default::default()
            },
        )
        .is_err()
    );
    assert_eq!(calls.get(), 1);

    let mut note = |_: FunnelDataAnnotationNoteContext<'_, Raw>| {
        Err(ChartError("authored raw note rejection".into()))
    };
    let spec = FunnelAnnotationSpec::new(
        FunnelAnnotationMatch::All,
        FunnelAnnotationKind::Circle,
        FunnelAnnotationNote::Custom("no".into()),
        FunnelAnnotationPosition::Relative(20.),
        FunnelAnnotationPosition::Relative(10.),
    );
    assert!(
        data.with_annotations(
            &[spec],
            &mut FunnelDataAnnotationFactories {
                matcher: None,
                note: Some(&mut note)
            },
            &Default::default()
        )
        .is_err()
    );
    assert_eq!(data, previous);
    assert_eq!(render::to_svg(data.scene()).unwrap(), pixels);
    assert_eq!(data.data(0).unwrap().metric.to_bits(), (-0.0f64).to_bits());
}
#[test]
fn checked_projection_replacement_rejects_source_bit_edits_but_keeps_legal_extra_ink() {
    let data = frame();
    let mut layout = data.layout().clone();
    layout.scene.labels.push(Label {
        rotation: 0.,
        x: 25.,
        y: 30.,
        text: "decoration".into(),
        color: Color(0, 0, 0, 255),
        size: 12.,
        anchor: TextAnchor::Start,
    });
    let replacement = data
        .with_computed(FunnelSvgFrame::from_layout(layout).unwrap())
        .unwrap();
    assert!(replacement.shares_source(&data));
    assert_eq!(replacement.computed().extras().len(), 1);
    let mut projected = data
        .source()
        .iter()
        .map(|d| d.projected.clone())
        .collect::<Vec<_>>();
    projected[2].value = 0.;
    let different = FunnelSvgFrame::new(&config(), &projected, &options()).unwrap();
    assert!(data.with_computed(different).is_err());
}

#[test]
fn checked_projection_preserves_nested_paint_bits_and_raw_fields_are_not_coerced() {
    let mut source = rows(f64::NAN);
    source[0].projected.paint = Some(FillPaint::Linear {
        start: [-0., 0.],
        end: [200., 0.],
        stops: vec![
            GradientStop {
                offset: -0.,
                color: Color(20, 70, 120, 255),
            },
            GradientStop {
                offset: 1.,
                color: Color(160, 40, 90, 255),
            },
        ],
    });
    let data = FunnelDataFrame::new(&config(), source, &options()).unwrap();
    assert!(
        data.data(0).unwrap().metric.is_nan(),
        "unprojected application fields are not scalar-schema validated"
    );
    let mut projected = data
        .source()
        .iter()
        .map(|d| d.projected.clone())
        .collect::<Vec<_>>();
    if let Some(FillPaint::Linear { start, .. }) = &mut projected[0].paint {
        start[0] = 0.;
    }
    let edited = FunnelSvgFrame::new(&config(), &projected, &options()).unwrap();
    assert!(
        data.with_computed(edited).is_err(),
        "same paint PartialEq must not erase original signed-zero authority"
    );
    if let Some(FillPaint::Linear { start, stops, .. }) = &data.source()[0].projected.paint {
        assert_eq!(start[0].to_bits(), (-0.0f64).to_bits());
        assert_eq!(stops[0].offset.to_bits(), (-0.0f64).to_bits());
    } else {
        panic!("original paint is retained");
    }
}
