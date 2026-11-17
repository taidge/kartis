use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

use kartis::*;

// No Clone, PartialEq, Eq, Debug, Send, Sync or serialization implementation.
struct Opaque {
    note: String,
    metric: f64,
    local: Rc<Cell<u32>>,
}

fn config() -> ChartConfig {
    ChartConfig {
        width: 300.,
        height: 220.,
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 20.,
            bottom: 20.,
        },
        ..Default::default()
    }
}

fn options() -> WaffleLayoutOptions {
    WaffleLayoutOptions {
        rows: 2,
        columns: 2,
        total: Some(100.),
        padding: 8.,
        ..Default::default()
    }
}

fn rows(note: &str, metric: f64) -> Vec<WaffleDatum<Opaque>> {
    [("a", 50.), ("zero", -0.)]
        .into_iter()
        .map(|(id, value)| {
            WaffleDatum::new(
                Opaque {
                    note: format!("{note}:{id}"),
                    metric,
                    local: Rc::new(Cell::new(7)),
                },
                RadialDatum::new(id, value),
            )
        })
        .collect()
}

fn placement(_: &WaffleDataCellWidgetContext<'_, Opaque>) -> Result<Option<PlotWidgetPlacement>> {
    Ok(Some(PlotWidgetPlacement::AtDatum {
        offset: [0., 0.],
        measured: TextSize {
            width: 8.,
            height: 6.,
        },
    }))
}

#[test]
fn shared_raw_pool_is_not_cloned_by_captures_widgets_or_projection_queries() {
    let source: Arc<[WaffleDatum<Opaque>]> = rows("original", -0.).into();
    let frame = WaffleDataFrame::from_shared(&config(), source.clone(), &options()).unwrap();
    let other = WaffleDataFrame::from_shared(&config(), source.clone(), &options()).unwrap();
    assert_ne!(
        frame, other,
        "distinct computations have distinct capture identity"
    );
    assert!(frame.shares_source(&other));
    assert_eq!(frame, frame.clone());
    assert_eq!(
        frame.computed().source()[1].value.to_bits(),
        (-0.0_f64).to_bits()
    );
    assert_eq!(frame.config().width, 300.);
    assert_eq!(frame.options(), &options());
    assert!(format!("{frame:?}").contains("WaffleDataFrame"));
    for (i, row) in source.iter().enumerate() {
        assert!(std::ptr::eq(frame.data(i).unwrap(), &row.data));
        assert!(std::ptr::eq(other.data(i).unwrap(), &row.data));
    }
    let packet = frame.cell_widgets(9, placement).unwrap();
    assert_eq!(packet, packet.clone());
    assert_eq!(packet.data.len(), 4);
    let mut occupied = 0;
    let mut empty = 0;
    for payload in packet.data.values() {
        assert_eq!(payload.frame(), &frame);
        assert!(payload.frame().shares_source(&other));
        match payload.data() {
            Some(data) => {
                assert!(std::ptr::eq(data, &source[0].data));
                data.local.set(data.local.get() + 1);
                occupied += 1;
            }
            None => empty += 1,
        }
    }
    assert_eq!((occupied, empty), (2, 2));
    assert_eq!(source[0].data.local.get(), 9);
    assert_eq!(other.data(0).unwrap().local.get(), 9);
    drop((frame, other, source));
    assert!(
        packet
            .data
            .values()
            .all(|p| p.data().is_none_or(|d| d.note == "original:a"))
    );
}

#[test]
fn raw_only_changes_notify_the_whole_cohort_and_old_payload_keeps_its_raw_bits() {
    let old = WaffleDataFrame::from_shared(&config(), rows("old", -0.).into(), &options()).unwrap();
    let packet = old.cell_widgets(3, placement).unwrap();
    let updated =
        WaffleDataFrame::from_shared(&config(), rows("new", 0.).into(), &options()).unwrap();
    assert_ne!(old, updated);
    assert!(!old.shares_source(&updated));
    assert_eq!(old.scene(), updated.scene());
    assert_eq!(old.layout(), updated.layout());
    assert_ne!(old.computed(), updated.computed());
    assert_eq!(old.data(0).unwrap().metric.to_bits(), (-0.0_f64).to_bits());
    assert_eq!(updated.data(0).unwrap().metric.to_bits(), 0.0_f64.to_bits());
    let updated_packet = updated.cell_widgets(3, placement).unwrap();
    assert_eq!(packet.layout, updated_packet.layout);
    assert_ne!(packet, updated_packet);
    for payload in packet.data.values() {
        assert_eq!(payload.frame(), &old);
        if let Some(data) = payload.data() {
            assert_eq!(data.note, "old:a");
            assert_eq!(data.metric.to_bits(), (-0.0_f64).to_bits());
        }
    }
}

#[test]
fn explicit_shared_input_domain_is_additive_and_legacy_vec_admission_stays_intact() {
    let o = WaffleLayoutOptions {
        total: Some(25.),
        ..options()
    };
    assert!(WaffleDataFrame::new(&config(), rows("legacy", -0.), &o).is_err());
    let shared = WaffleDataFrame::from_shared(&config(), rows("shared", -0.).into(), &o).unwrap();
    assert_eq!(shared.layout().groups[0].requested, 0..8);
    assert_eq!(shared.layout().groups[0].occupied, 0..4);
    assert!(shared.layout().cells.iter().all(|c| c.owner == Some(0)));
    assert!(
        WaffleDataFrame::from_shared(
            &config(),
            rows("largest-remainder", -0.).into(),
            &WaffleLayoutOptions {
                allocation: WaffleAllocation::LargestRemainder,
                ..o
            },
        )
        .is_err()
    );
    let legacy_mode = WaffleLayoutOptions {
        mode: WaffleMode::Areas,
        ..options()
    };
    let legacy = WaffleDataFrame::new(&config(), rows("areas", -0.), &legacy_mode).unwrap();
    let captured =
        WaffleDataFrame::from_shared(&config(), rows("areas", -0.).into(), &legacy_mode).unwrap();
    assert_eq!(legacy.scene(), captured.scene());
    assert_eq!(legacy.layout(), captured.layout());
    assert_eq!(captured.cell_widgets(0, placement).unwrap().data.len(), 2);
}

#[test]
fn rejected_shared_input_never_invokes_raw_color_or_cell_functions() {
    let calls = Cell::new(0);
    let mut color = |_: &WaffleDataColorContext<'_, Opaque>| {
        calls.set(calls.get() + 1);
        Ok(Color(1, 2, 3, 255))
    };
    let mut cell = |c: &WaffleDataCellContext<'_, Opaque>| {
        calls.set(calls.get() + 1);
        Ok(c.computed.inherit(c.computed.geometry.clone()))
    };
    let mut cases = vec![WaffleLayoutOptions {
        rows: usize::MAX,
        ..options()
    }];
    cases.push(WaffleLayoutOptions {
        total: Some(f64::MIN_POSITIVE),
        ..options()
    });
    for o in cases {
        assert!(
            WaffleDataFrame::with_shared_factories_and_cell_factory(
                &config(),
                rows("rejected", -0.).into(),
                &o,
                &mut WaffleDataFactories {
                    color: Some(&mut color),
                    ..Default::default()
                },
                &mut cell,
            )
            .is_err()
        );
        assert_eq!(calls.get(), 0);
    }
}

#[test]
fn combined_shared_factories_observe_one_raw_arc_and_are_not_replayed_by_svg_queries() {
    let pool: Arc<[WaffleDatum<Opaque>]> = rows("functions", -0.).into();
    let calls = Cell::new([0usize; 4]);
    let increment = |role: usize| {
        let mut c = calls.get();
        c[role] += 1;
        calls.set(c);
    };
    let resolved = |i: usize| Color(10 + i as u8, 20, 30, 255);
    let mut color = |c: &WaffleDataColorContext<'_, Opaque>| {
        increment(0);
        assert!(std::ptr::eq(c.data, &pool[c.computed.source_index].data));
        assert_eq!(c.data.metric.to_bits(), (-0.0_f64).to_bits());
        Ok(resolved(c.computed.source_index))
    };
    let mut formatted = |n: f64| {
        increment(1);
        Ok(format!("bits:{:016x}", n.to_bits()))
    };
    let mut border = |c: &WaffleDataBorderContext<'_, Opaque>| {
        increment(2);
        match c.computed.target {
            WaffleBorderTarget::Group(group) => {
                assert!(std::ptr::eq(
                    c.data.unwrap(),
                    &pool[group.source_index].data
                ));
                assert_eq!(group.color, resolved(group.source_index));
                assert_eq!(group.border_color, group.color);
                assert!(group.polygons.is_empty());
                assert_eq!(
                    group.formatted_value,
                    format!("bits:{:016x}", group.source.value.to_bits())
                );
            }
            WaffleBorderTarget::Empty(_) => assert!(c.data.is_none()),
        }
        Ok(Color(40, 50, 60, 255))
    };
    let mut cell = |c: &WaffleDataCellContext<'_, Opaque>| {
        assert_eq!(calls.get()[..3], [2, 2, 3]);
        increment(3);
        if let Some(group) = c.computed.group {
            assert!(std::ptr::eq(
                c.data.unwrap(),
                &pool[group.source_index].data
            ));
            assert_eq!(c.computed.fill, Some(resolved(group.source_index)));
        } else {
            assert!(c.data.is_none());
        }
        Ok(c.computed.inherit(c.computed.geometry.clone()))
    };
    let frame = WaffleDataFrame::with_shared_factories_and_cell_factory(
        &config(),
        pool.clone(),
        &options(),
        &mut WaffleDataFactories {
            color: Some(&mut color),
            formatted_value: Some(&mut formatted),
            border_color: Some(&mut border),
        },
        &mut cell,
    )
    .unwrap();
    assert_eq!(calls.get(), [2, 2, 3, 4]);
    assert_eq!(frame.source().as_ptr(), pool.as_ptr());
    let packet = frame.cell_widgets(3, placement).unwrap();
    let clone = packet.clone();
    assert_eq!(clone, packet);
    assert_eq!(packet.data.len(), 4);
    for payload in packet.data.values() {
        assert_eq!(payload.frame(), &frame);
        assert_eq!(payload.frame().computed(), frame.computed());
        if let Some(group) = &payload.computed().group {
            assert!(std::ptr::eq(
                payload.data().unwrap(),
                &pool[group.source_index].data
            ));
        } else {
            assert!(payload.data().is_none());
        }
    }
    let _ = frame.interaction();
    let _ = frame.area_interaction().unwrap();
    let _ = frame.computed().areas();
    let _ = frame.config();
    let _ = frame.options();
    assert_eq!(calls.get(), [2, 2, 3, 4]);
}
