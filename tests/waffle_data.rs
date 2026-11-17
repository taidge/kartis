use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use kartis::*;

// Intentionally implements none of Clone, Debug or PartialEq. Raw application
// values are not constrained to the numeric projection or copied into cells.
struct ApplicationData {
    status: String,
    revision: usize,
    raw_value: f64,
    drops: Rc<Cell<usize>>,
}
impl Drop for ApplicationData {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
    }
}

fn config() -> ChartConfig {
    ChartConfig {
        width: 320.,
        height: 240.,
        margin: Margin {
            left: 23.,
            right: 17.,
            top: 19.,
            bottom: 21.,
        },
        ..Default::default()
    }
}
fn options() -> WaffleLayoutOptions {
    WaffleLayoutOptions {
        rows: 2,
        columns: 3,
        total: Some(100.),
        border_width: 2.,
        padding: 5.,
        ..Default::default()
    }
}
fn source(
    values: &[f64],
    revision: usize,
    drops: &Rc<Cell<usize>>,
) -> Vec<WaffleDatum<ApplicationData>> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            WaffleDatum::new(
                ApplicationData {
                    status: format!("business-{revision}-{index}"),
                    revision,
                    // Deliberately independent of projected.value.
                    raw_value: -0.,
                    drops: drops.clone(),
                },
                RadialDatum {
                    id: format!("g{index}"),
                    label: if index == 0 {
                        String::new()
                    } else {
                        format!("Group {index}")
                    },
                    value: *value,
                },
            )
        })
        .collect()
}
fn placement(
    context: &WaffleDataCellWidgetContext<'_, ApplicationData>,
) -> Result<Option<PlotWidgetPlacement>> {
    let bounds = context.computed.cell.bounds;
    Ok(Some(PlotWidgetPlacement::Rect(LayoutRect {
        x: bounds.x,
        y: bounds.y,
        width: bounds.width,
        height: bounds.height,
    })))
}
fn require_packet_traits<T: Clone + PartialEq + std::fmt::Debug>(_: &T) {}
fn resolved_color(index: usize) -> Color {
    Color(31 + index as u8, 91, 151, 255)
}

#[test]
fn capture_identity_observes_raw_only_updates_without_traits_on_application_data() {
    let drops = Rc::new(Cell::new(0));
    let first =
        WaffleDataFrame::new(&config(), source(&[34., 50., -0.], 7, &drops), &options()).unwrap();
    let clone = first.clone();
    let updated =
        WaffleDataFrame::new(&config(), source(&[34., 50., -0.], 8, &drops), &options()).unwrap();
    require_packet_traits(&first);
    assert_eq!(clone, first);
    assert_ne!(updated, first);
    assert_eq!(
        updated.scene(),
        first.scene(),
        "only application data changed"
    );
    assert_eq!(first.data(0).unwrap().status, "business-7-0");
    assert_eq!(updated.data(0).unwrap().status, "business-8-0");
    assert_eq!(first.data(0).unwrap().revision, 7);
    assert_eq!(
        first.data(0).unwrap().raw_value.to_bits(),
        (-0f64).to_bits()
    );
    assert_eq!(first.source()[0].projected.value, 34.);
    assert_eq!(first.source()[0].projected.label, "");
    assert!(first.data(first.source().len()).is_none());
    let old_packet = first.cell_widgets(9, placement).unwrap();
    let new_packet = updated.cell_widgets(9, placement).unwrap();
    assert_eq!(old_packet.layout, new_packet.layout);
    for (key, payload) in &old_packet.data {
        assert_eq!(payload.computed(), new_packet.data[key].computed());
    }
    assert_ne!(
        old_packet, new_packet,
        "raw-only updates must notify retained Widget data"
    );
    drop((old_packet, new_packet));
    for (index, group) in first.layout().groups.iter().enumerate() {
        assert_eq!(group.source, first.source()[index].projected);
        assert_eq!(
            group.source.value.to_bits(),
            first.source()[index].projected.value.to_bits()
        );
    }
    let positive_zero =
        WaffleDataFrame::new(&config(), source(&[34., 50., 0.], 7, &drops), &options()).unwrap();
    assert_ne!(positive_zero, first);
    assert_eq!(
        positive_zero.layout(),
        first.layout(),
        "layout equality treats zero signs as equal"
    );
    assert_ne!(
        positive_zero.source()[2].projected.value.to_bits(),
        first.source()[2].projected.value.to_bits()
    );
    drop((clone, first, updated, positive_zero));
    assert_eq!(drops.get(), 9, "one drop for each of nine owned raw values");
}

#[test]
fn widgets_share_actual_raw_objects_and_keep_the_whole_cohort_alive() {
    let drops = Rc::new(Cell::new(0));
    let rows = source(&[34., 50., -0.], 11, &drops);
    let raw_addresses: Vec<_> = rows
        .iter()
        .map(|row| &row.data as *const ApplicationData)
        .collect();
    let frame = WaffleDataFrame::new(&config(), rows, &options()).unwrap();
    for (index, address) in raw_addresses.iter().enumerate() {
        assert_eq!(frame.data(index).unwrap() as *const _, *address);
    }
    let packet = frame
        .cell_widgets(19, |context| {
            match context.computed.group {
                Some(group) => assert!(std::ptr::eq(
                    context.data.unwrap(),
                    frame.data(group.source_index).unwrap()
                )),
                None => assert!(context.data.is_none()),
            }
            placement(context)
        })
        .unwrap();
    require_packet_traits(&packet);
    assert_eq!(packet.data.len(), 6);
    assert!(packet.layout.slots.iter().all(|slot| slot.revision == 19));
    let group_zero: Vec<_> = packet
        .data
        .values()
        .filter(|payload| {
            payload
                .computed()
                .group
                .as_ref()
                .is_some_and(|group| group.source_index == 0)
        })
        .collect();
    assert_eq!(group_zero.len(), 2);
    assert!(std::ptr::eq(
        group_zero[0].data().unwrap(),
        group_zero[1].data().unwrap()
    ));
    assert!(Arc::ptr_eq(
        group_zero[0].computed().group.as_ref().unwrap(),
        group_zero[1].computed().group.as_ref().unwrap()
    ));
    assert_eq!(
        packet
            .data
            .values()
            .filter(|payload| payload.data().is_none())
            .count(),
        1
    );
    for payload in packet.data.values() {
        require_packet_traits(payload);
        assert_eq!(payload.frame(), &frame);
        assert_eq!(payload.clone(), *payload);
        if let Some(group) = payload.computed().group.as_ref() {
            assert_eq!(
                payload.data().unwrap() as *const _,
                raw_addresses[group.source_index]
            );
            assert_eq!(
                group.source.value.to_bits(),
                frame.source()[group.source_index].projected.value.to_bits()
            );
        }
    }
    let held = group_zero[0].clone();
    drop(frame);
    drop(packet);
    assert_eq!(
        drops.get(),
        0,
        "a single payload retains even zero-allocation raw groups"
    );
    assert_eq!(
        held.frame().source()[2].projected.value.to_bits(),
        (-0f64).to_bits()
    );
    drop(held);
    assert_eq!(drops.get(), 3);
}

#[test]
fn raw_callbacks_include_hidden_zero_and_overflow_groups_in_upstream_order() {
    // Matches the independently executed upstream reference's six-slot fixture:
    // five rounded one-cell groups plus tail request two, so tail occupies one.
    let values = [-0., 1., 2., 0., 1., 1., 1., 1., 3.];
    let drops = Rc::new(Cell::new(0));
    let rows = source(&values, 23, &drops);
    let addresses: Vec<_> = rows
        .iter()
        .map(|row| &row.data as *const ApplicationData)
        .collect();
    let trace = RefCell::new(Vec::new());
    let expected_ranges = [0..0, 0..1, 1..1, 1..1, 1..2, 2..3, 3..4, 4..5, 5..7];
    let mut color = |context: &WaffleDataColorContext<'_, ApplicationData>| {
        let index = context.computed.source_index;
        assert_eq!(context.data as *const _, addresses[index]);
        assert_eq!(context.data.revision, 23);
        assert_eq!(context.data.status, format!("business-23-{index}"));
        assert_eq!(
            context.computed.datum.value.to_bits(),
            values[index].to_bits()
        );
        trace.borrow_mut().push(format!("color:{index}"));
        Ok(resolved_color(index))
    };
    let mut formatter = |value: f64| {
        trace
            .borrow_mut()
            .push(format!("format:{:016x}", value.to_bits()));
        Ok(format!("value:{:016x}", value.to_bits()))
    };
    let mut border =
        |context: &WaffleDataBorderContext<'_, ApplicationData>| match context.computed.target {
            WaffleBorderTarget::Group(group) => {
                let index = group.source_index;
                assert_eq!(context.data.unwrap() as *const _, addresses[index]);
                assert_eq!(group.requested, expected_ranges[index]);
                assert_eq!(
                    group.occupied,
                    if index == 8 {
                        5..6
                    } else {
                        expected_ranges[index].clone()
                    }
                );
                assert_eq!(group.hidden, index == 2);
                assert_eq!(group.color, resolved_color(index));
                assert_eq!(group.border_color, group.color);
                assert_eq!(
                    group.formatted_value,
                    format!("value:{:016x}", values[index].to_bits())
                );
                assert!(group.polygons.is_empty());
                trace.borrow_mut().push(format!("border:{index}"));
                Ok(Color(80, 90, 100, 255))
            }
            WaffleBorderTarget::Empty(_) => {
                assert!(context.data.is_none());
                trace.borrow_mut().push("empty".into());
                Ok(Color(90, 100, 110, 255))
            }
        };
    let frame = WaffleDataFrame::with_factories(
        &config(),
        rows,
        &WaffleLayoutOptions {
            total: Some(12.),
            hidden: ["g2".into()].into(),
            ..options()
        },
        &mut WaffleDataFactories {
            color: Some(&mut color),
            formatted_value: Some(&mut formatter),
            border_color: Some(&mut border),
        },
    )
    .unwrap();
    let expected: Vec<_> = values
        .iter()
        .enumerate()
        .flat_map(|(index, value)| {
            [
                format!("color:{index}"),
                format!("format:{:016x}", value.to_bits()),
                format!("border:{index}"),
            ]
        })
        .chain(std::iter::once("empty".into()))
        .collect();
    assert_eq!(*trace.borrow(), expected);
    assert!(frame.layout().cells.iter().all(|cell| cell.owner.is_some()));
    assert_eq!(frame.layout().groups[8].requested, 5..7);
    assert_eq!(frame.layout().groups[8].occupied, 5..6);
    assert_eq!(
        frame.source()[0].projected.value.to_bits(),
        (-0f64).to_bits()
    );
    assert_eq!(frame.source()[3].projected.value.to_bits(), 0f64.to_bits());
}

#[test]
fn combined_cell_factory_observes_resolved_cohort_and_queries_never_replay_functions() {
    let drops = Rc::new(Cell::new(0));
    let counts = Cell::new([0usize; 4]);
    let increment = |index: usize| {
        let mut current = counts.get();
        current[index] += 1;
        counts.set(current);
    };
    let mut color = |context: &WaffleDataColorContext<'_, ApplicationData>| {
        increment(0);
        Ok(resolved_color(context.computed.source_index))
    };
    let mut formatter = |value: f64| {
        increment(1);
        Ok(format!("resolved:{:016x}", value.to_bits()))
    };
    let mut border = |context: &WaffleDataBorderContext<'_, ApplicationData>| {
        increment(2);
        assert_eq!(
            context.data.is_some(),
            matches!(context.computed.target, WaffleBorderTarget::Group(_))
        );
        Ok(Color(12, 34, 56, 255))
    };
    let mut cell = |context: &WaffleDataCellContext<'_, ApplicationData>| {
        assert_eq!(
            &counts.get()[..3],
            &[3, 3, 4],
            "all group and empty-template functions precede cells"
        );
        increment(3);
        if let Some(group) = context.computed.group {
            assert_eq!(
                context.data.unwrap().status,
                format!("business-31-{}", group.source_index)
            );
            assert_eq!(
                context.computed.fill,
                Some(resolved_color(group.source_index))
            );
            assert_eq!(
                group.formatted_value,
                format!("resolved:{:016x}", group.source.value.to_bits())
            );
        } else {
            assert!(context.data.is_none());
        }
        Ok(context.computed.inherit(context.computed.geometry.clone()))
    };
    let frame = WaffleDataFrame::with_factories_and_cell_factory(
        &config(),
        source(&[34., 50., -0.], 31, &drops),
        &options(),
        &mut WaffleDataFactories {
            color: Some(&mut color),
            formatted_value: Some(&mut formatter),
            border_color: Some(&mut border),
        },
        &mut cell,
    )
    .unwrap();
    assert_eq!(counts.get(), [3, 3, 4, 6]);
    assert_eq!(frame.cell_widgets(5, placement).unwrap().data.len(), 6);
    assert!(
        !frame
            .area_interaction()
            .unwrap()
            .interaction
            .unwrap()
            .regions
            .is_empty()
    );
    assert_eq!(
        frame.interaction().interaction.unwrap().sites,
        frame.layout().interaction.sites
    );
    assert!(!render::to_svg(frame.scene()).unwrap().is_empty());
    let clone = frame.clone();
    assert!(std::ptr::eq(clone.data(0).unwrap(), frame.data(0).unwrap()));
    assert_eq!(
        counts.get(),
        [3, 3, 4, 6],
        "borrowed build functions are not retained by read/query/Widget APIs"
    );
}

#[test]
fn areas_offer_raw_data_only_for_actual_cell_roles_and_empty_source_has_no_fake_datum() {
    let drops = Rc::new(Cell::new(0));
    let frame = WaffleDataFrame::new(
        &config(),
        source(&[34., 50., 0.], 37, &drops),
        &WaffleLayoutOptions {
            mode: WaffleMode::Areas,
            ..options()
        },
    )
    .unwrap();
    let packet = frame
        .cell_widgets(3, |context| {
            assert!(context.data.is_none());
            assert!(context.computed.group.is_none());
            placement(context)
        })
        .unwrap();
    assert_eq!(
        packet.data.len(),
        1,
        "occupied Areas have union ink rather than individual cell Widgets"
    );
    assert!(packet.data.values().all(|payload| payload.data().is_none()));
    let empty_calls = Cell::new(0);
    let mut border = |context: &WaffleDataBorderContext<'_, ApplicationData>| {
        assert!(matches!(
            context.computed.target,
            WaffleBorderTarget::Empty(_)
        ));
        assert!(context.data.is_none());
        empty_calls.set(empty_calls.get() + 1);
        Ok(Color(30, 40, 50, 255))
    };
    let empty = WaffleDataFrame::with_factories(
        &config(),
        Vec::new(),
        &options(),
        &mut WaffleDataFactories {
            border_color: Some(&mut border),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(empty_calls.get(), 1);
    assert!(empty.source().is_empty() && empty.layout().groups.is_empty());
    assert_eq!(empty.cell_widgets(4, placement).unwrap().data.len(), 6);
}

#[test]
fn invalid_projection_is_rejected_before_any_raw_or_cell_callback() {
    let drops = Rc::new(Cell::new(0));
    for invalid in [0, 1, 2] {
        let mut rows = source(&[34., 50., 0.], 41, &drops);
        match invalid {
            0 => rows[1].projected.id = rows[0].projected.id.clone(),
            1 => rows[1].projected.value = f64::NAN,
            _ => rows[1].projected.value = -1.,
        }
        let calls = Cell::new(0);
        let mut color = |_: &WaffleDataColorContext<'_, ApplicationData>| {
            calls.set(calls.get() + 1);
            Ok(Color(0, 0, 0, 255))
        };
        let mut formatter = |_: f64| {
            calls.set(calls.get() + 1);
            Ok(String::new())
        };
        let mut border = |_: &WaffleDataBorderContext<'_, ApplicationData>| {
            calls.set(calls.get() + 1);
            Ok(Color(0, 0, 0, 255))
        };
        let mut cell = |context: &WaffleDataCellContext<'_, ApplicationData>| {
            calls.set(calls.get() + 1);
            Ok(context.computed.inherit(context.computed.geometry.clone()))
        };
        assert!(
            WaffleDataFrame::with_factories_and_cell_factory(
                &config(),
                rows,
                &options(),
                &mut WaffleDataFactories {
                    color: Some(&mut color),
                    formatted_value: Some(&mut formatter),
                    border_color: Some(&mut border),
                },
                &mut cell
            )
            .is_err()
        );
        assert_eq!(calls.get(), 0);
    }
    assert_eq!(drops.get(), 9);
}

#[test]
fn fallible_callbacks_drop_candidate_source_without_changing_a_prior_cohort() {
    let old_drops = Rc::new(Cell::new(0));
    let old = WaffleDataFrame::new(
        &config(),
        source(&[34., 50., -0.], 43, &old_drops),
        &options(),
    )
    .unwrap();
    let before = old.scene().clone();
    let packet = old.cell_widgets(7, placement).unwrap();
    let failed_drops = Rc::new(Cell::new(0));
    let calls = Cell::new(0);
    let mut color = |context: &WaffleDataColorContext<'_, ApplicationData>| {
        calls.set(calls.get() + 1);
        if context.computed.source_index == 1 {
            Err(ChartError("application rejected group one".into()))
        } else {
            Ok(Color(100, 0, 0, 255))
        }
    };
    let error = WaffleDataFrame::with_factories(
        &config(),
        source(&[34., 50., -0.], 44, &failed_drops),
        &options(),
        &mut WaffleDataFactories {
            color: Some(&mut color),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(error.0.contains("application rejected group one"));
    assert_eq!(
        calls.get(),
        2,
        "external callback side effects are not rolled back"
    );
    assert_eq!(failed_drops.get(), 3);
    let cell_calls = Cell::new(0);
    let mut cell = |context: &WaffleDataCellContext<'_, ApplicationData>| {
        cell_calls.set(cell_calls.get() + 1);
        if cell_calls.get() == 2 {
            Err(ChartError("application rejected second cell".into()))
        } else {
            Ok(context.computed.inherit(context.computed.geometry.clone()))
        }
    };
    assert!(
        WaffleDataFrame::with_factories_and_cell_factory(
            &config(),
            source(&[34., 50., -0.], 45, &failed_drops),
            &options(),
            &mut WaffleDataFactories::default(),
            &mut cell
        )
        .is_err()
    );
    assert_eq!(cell_calls.get(), 2);
    assert_eq!(failed_drops.get(), 6);
    assert_eq!(old.scene(), &before);
    assert_eq!(old.data(0).unwrap().revision, 43);
    assert!(packet.data.values().all(|payload| payload.frame() == &old));
    assert_eq!(old_drops.get(), 0);
}
