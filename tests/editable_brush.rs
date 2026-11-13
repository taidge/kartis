use kartis::Brush;
use kartis::brush::*;

fn rect(a: [f64; 4]) -> Brush {
    Brush {
        x0: a[0],
        y0: a[1],
        x1: a[2],
        y1: a[3],
    }
}
fn options() -> EditableBrushOptions {
    EditableBrushOptions {
        extent: rect([10., 20., 310., 220.]),
        handle_size: 10.,
        ..Default::default()
    }
}
fn seeded() -> EditableBrush {
    let mut b = EditableBrush::new(options()).unwrap();
    b.set_selection(Some(rect([80., 70., 220., 170.]))).unwrap();
    b
}
fn begin(b: &mut EditableBrush, target: BrushTarget, point: (f64, f64)) -> BrushSession {
    let e = b
        .apply(EditableBrushInput::Begin {
            id: 4,
            point,
            target,
            modifiers: BrushModifiers::default(),
        })
        .unwrap();
    assert_eq!(e.len(), 1);
    assert_eq!(e[0].phase, BrushPhase::Start);
    b.session().unwrap()
}

#[test]
fn actual_d3_browser_trajectories_match_geometry_modes_events_and_handle_rectangles() {
    let mut controller = None;
    let mut enabled = false;
    let mut modifiers = BrushModifiers::default();
    let mut name = "";
    let mut cases = 0;
    let mut steps = 0;
    let mut event_count = 0;
    let mut handles = 0;
    for line in include_str!("fixtures/editable-brush-d3.csv")
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let p: Vec<_> = line.split('|').collect();
        match p[0] {
            "C" => {
                name = p[1];
                enabled = matches!(p[6], "geometry" | "filter");
                if !enabled {
                    continue;
                }
                cases += 1;
                modifiers = BrushModifiers::default();
                let axis = match p[2] {
                    "x" => BrushAxis::X,
                    "y" => BrushAxis::Y,
                    "xy" => BrushAxis::XY,
                    _ => panic!("bad axis"),
                };
                let mut b = EditableBrush::new(EditableBrushOptions {
                    axis,
                    extent: parse_rect(p[3]).unwrap(),
                    handle_size: p[5].parse().unwrap(),
                    modifiers: p[7] == "1",
                })
                .unwrap();
                b.set_selection(parse_rect(p[4])).unwrap();
                controller = Some(b);
            }
            "S" if enabled => {
                let b = controller.as_mut().unwrap();
                let token = b.session();
                let point = || {
                    let n: Vec<f64> = p[5].split(',').map(|v| v.parse().unwrap()).collect();
                    (n[0], n[1])
                };
                let events = match p[3] {
                    "down" => {
                        modifiers = flags(p[7]);
                        b.apply(EditableBrushInput::Begin {
                            id: 4,
                            point: point(),
                            target: target(p[4]),
                            modifiers,
                        })
                        .unwrap()
                    }
                    "move" => token
                        .map(|session| {
                            b.apply(EditableBrushInput::Move {
                                session,
                                id: 4,
                                point: point(),
                            })
                            .unwrap()
                        })
                        .unwrap_or_default(),
                    "up" => token
                        .map(|session| {
                            b.apply(EditableBrushInput::End {
                                session,
                                id: 4,
                                point: point(),
                            })
                            .unwrap()
                        })
                        .unwrap_or_default(),
                    "key" => {
                        let key: Vec<_> = p[6].split(',').collect();
                        let down = key[1] == "1";
                        match key[0] {
                            "Alt" => modifiers.alt = down,
                            "Shift" => modifiers.shift = down,
                            "Space" => modifiers.space = down,
                            _ => panic!("bad key"),
                        }
                        b.apply(EditableBrushInput::Modifiers {
                            session: token.unwrap(),
                            modifiers,
                        })
                        .unwrap()
                    }
                    other => panic!("unexpected equivalent-profile action {other}"),
                };
                assert_eq!(
                    b.selection(),
                    parse_rect(p[9]),
                    "{name} step {} selection",
                    p[2]
                );
                let expected: Vec<_> = p[10].split(';').filter(|s| !s.is_empty()).collect();
                assert_eq!(
                    events.len(),
                    expected.len(),
                    "{name} step {} events {:?}",
                    p[2],
                    events
                );
                for (event, expected) in events.iter().zip(expected) {
                    let v: Vec<_> = expected.split(':').collect();
                    let phase = match event.phase {
                        BrushPhase::Start => "start",
                        BrushPhase::Change => "brush",
                        BrushPhase::End => "end",
                        BrushPhase::Cancel => "cancel",
                    };
                    let mode = match event.mode {
                        BrushMode::Create | BrushMode::Resize => "handle",
                        BrushMode::Move => "drag",
                        BrushMode::Center => "center",
                        BrushMode::Space => "space",
                        BrushMode::Programmatic | BrushMode::Clear => "none",
                    };
                    assert_eq!(
                        (phase, mode, event.selection),
                        (v[0], v[1], parse_rect(v[2])),
                        "{name} step {} event",
                        p[2]
                    );
                    assert_eq!(event.source, BrushSource::User);
                    event_count += 1;
                }
                steps += 1;
            }
            "H" if enabled => {
                let b = controller.as_ref().unwrap();
                let handle = match target(p[3]) {
                    BrushTarget::Handle(h) => h,
                    _ => panic!("bad handle"),
                };
                let measured = b.handles().into_iter().find(|h| h.handle == handle);
                if p[4] == "0" {
                    assert!(measured.is_none(), "{name} hidden handle");
                    continue;
                }
                let n: Vec<f64> = p[5].split(',').map(|v| v.parse().unwrap()).collect();
                assert_eq!(
                    measured.unwrap().bounds,
                    rect([n[0], n[1], n[0] + n[2], n[1] + n[3]]),
                    "{name} handle {} step {}",
                    p[3],
                    p[2]
                );
                handles += 1;
            }
            _ => {}
        }
    }
    assert_eq!((cases, steps, event_count, handles), (67, 253, 234, 1648));
}

fn parse_rect(value: &str) -> Option<Brush> {
    if value == "null" {
        return None;
    }
    let v: Vec<f64> = value.split(',').map(|v| v.parse().unwrap()).collect();
    Some(rect([v[0], v[1], v[2], v[3]]))
}
fn target(value: &str) -> BrushTarget {
    match value {
        "overlay" => BrushTarget::Overlay,
        "selection" => BrushTarget::Selection,
        "n" => BrushTarget::Handle(BrushHandle::N),
        "e" => BrushTarget::Handle(BrushHandle::E),
        "s" => BrushTarget::Handle(BrushHandle::S),
        "w" => BrushTarget::Handle(BrushHandle::W),
        "nw" => BrushTarget::Handle(BrushHandle::Nw),
        "ne" => BrushTarget::Handle(BrushHandle::Ne),
        "sw" => BrushTarget::Handle(BrushHandle::Sw),
        "se" => BrushTarget::Handle(BrushHandle::Se),
        _ => panic!("bad target"),
    }
}
fn flags(value: &str) -> BrushModifiers {
    let names: Vec<_> = value.split(',').collect();
    BrushModifiers {
        alt: names.contains(&"alt"),
        shift: names.contains(&"shift"),
        space: names.contains(&"space"),
        meta: names.contains(&"meta"),
        ctrl: names.contains(&"ctrl"),
    }
}

#[test]
fn stale_session_wrong_pointer_and_other_controller_cannot_modify_current_gesture() {
    let mut b = seeded();
    let origin = b.selection();
    let old = begin(&mut b, BrushTarget::Selection, (150., 120.));
    b.apply(EditableBrushInput::Move {
        session: old,
        id: 4,
        point: (160., 130.),
    })
    .unwrap();
    assert_eq!(b.cancel().unwrap()[0].selection, origin);
    let current = begin(&mut b, BrushTarget::Selection, (150., 120.));
    assert_ne!(old, current);
    let state = b.clone();
    for input in [
        EditableBrushInput::Move {
            session: old,
            id: 4,
            point: (200., 180.),
        },
        EditableBrushInput::End {
            session: old,
            id: 4,
            point: (200., 180.),
        },
        EditableBrushInput::Cancel {
            session: current,
            id: 99,
        },
    ] {
        assert!(b.apply(input).unwrap().is_empty());
        assert_eq!(b, state);
    }
    let mut other = seeded();
    let foreign = begin(&mut other, BrushTarget::Selection, (150., 120.));
    assert_ne!(foreign, current);
    assert!(
        b.apply(EditableBrushInput::Move {
            session: foreign,
            id: 4,
            point: (200., 180.)
        })
        .unwrap()
        .is_empty()
    );
    assert_eq!(b, state);
}

#[test]
fn invalid_input_and_configuration_preserve_entire_accepted_gesture() {
    let mut b = seeded();
    let s = begin(&mut b, BrushTarget::Handle(BrushHandle::Se), (220., 170.));
    b.apply(EditableBrushInput::Move {
        session: s,
        id: 4,
        point: (240., 180.),
    })
    .unwrap();
    let before = b.clone();
    for input in [
        EditableBrushInput::Move {
            session: s,
            id: 4,
            point: (f64::NAN, 1.),
        },
        EditableBrushInput::End {
            session: s,
            id: 4,
            point: (1_000_001., 1.),
        },
        EditableBrushInput::Nudge {
            target: BrushTarget::Selection,
            delta: (0., f64::INFINITY),
        },
    ] {
        assert!(b.apply(input).is_err());
        assert_eq!(b, before);
    }
    assert!(b.set_selection(Some(rect([1., 1., 20., 20.]))).is_err());
    assert_eq!(b, before);
    let mut opts = options();
    opts.extent = rect([20., 30., 210., 150.]);
    assert!(b.configure(opts, BrushReconfigure::Preserve).is_err());
    assert_eq!(b, before);
    opts.handle_size = f64::NAN;
    assert!(b.configure(opts, BrushReconfigure::Clear).is_err());
    assert_eq!(b, before);
    opts = options();
    opts.extent = rect([0., 0., 1_000_000., 10.]);
    assert!(b.configure(opts, BrushReconfigure::Clamp).is_err());
    assert_eq!(b, before);
}

#[test]
fn programmatic_updates_are_idempotent_and_retire_current_baseline() {
    let mut b = seeded();
    let s = begin(&mut b, BrushTarget::Handle(BrushHandle::Se), (220., 170.));
    b.apply(EditableBrushInput::Move {
        session: s,
        id: 4,
        point: (240., 180.),
    })
    .unwrap();
    let accepted = b.selection();
    let e = b.set_selection(accepted).unwrap();
    assert_eq!(e.len(), 1);
    assert_eq!(e[0].phase, BrushPhase::Cancel);
    assert!(!b.active());
    assert!(b.set_selection(accepted).unwrap().is_empty());
    assert!(
        b.apply(EditableBrushInput::Move {
            session: s,
            id: 4,
            point: (280., 200.)
        })
        .unwrap()
        .is_empty()
    );
    assert_eq!(b.selection(), accepted);
    let next = Some(rect([40., 45., 180., 150.]));
    let e = b.set_selection(next).unwrap();
    assert_eq!(
        e.iter().map(|e| e.phase).collect::<Vec<_>>(),
        [BrushPhase::Start, BrushPhase::Change, BrushPhase::End]
    );
    assert!(
        e.iter()
            .all(|e| e.source == BrushSource::Programmatic && e.session.is_none())
    );
    assert!(b.set_selection(next).unwrap().is_empty());
    assert!(
        b.set_selection(Some(rect([80., 80., 80., 100.])))
            .unwrap()
            .last()
            .unwrap()
            .selection
            .is_none()
    );
    assert!(b.set_selection(None).unwrap().is_empty());
}

#[test]
fn single_axis_inactive_zero_span_is_valid_and_only_active_zero_clears() {
    for axis in [BrushAxis::X, BrushAxis::Y] {
        let extent = if axis == BrushAxis::X {
            rect([10., 20., 310., 20.])
        } else {
            rect([10., 20., 10., 220.])
        };
        let mut b = EditableBrush::new(EditableBrushOptions {
            axis,
            extent,
            handle_size: 10.,
            modifiers: true,
        })
        .unwrap();
        let start = if axis == BrushAxis::X {
            (100., 20.)
        } else {
            (10., 80.)
        };
        let end = if axis == BrushAxis::X {
            (200., 20.)
        } else {
            (10., 180.)
        };
        let s = begin(&mut b, BrushTarget::Overlay, start);
        b.apply(EditableBrushInput::End {
            session: s,
            id: 4,
            point: end,
        })
        .unwrap();
        assert_eq!(b.selection(), Some(Brush::from_points(start, end).unwrap()));
        assert_eq!(b.handles().len(), 2);
        for h in b.handles() {
            h.bounds.validate().unwrap();
        }
        let s = begin(&mut b, BrushTarget::Overlay, start);
        b.apply(EditableBrushInput::End {
            session: s,
            id: 4,
            point: start,
        })
        .unwrap();
        assert_eq!(b.selection(), None);
    }
    let mut b = EditableBrush::new(EditableBrushOptions {
        extent: rect([10., 20., 10., 20.]),
        ..options()
    })
    .unwrap();
    let s = begin(&mut b, BrushTarget::Overlay, (10., 20.));
    b.apply(EditableBrushInput::End {
        session: s,
        id: 4,
        point: (30., 40.),
    })
    .unwrap();
    assert_eq!(b.selection(), None);
}

#[test]
fn configure_has_explicit_preserve_clamp_clear_and_axis_fill_policies() {
    let mut b = seeded();
    let s = begin(&mut b, BrushTarget::Selection, (150., 120.));
    b.apply(EditableBrushInput::Move {
        session: s,
        id: 4,
        point: (170., 135.),
    })
    .unwrap();
    let opts = EditableBrushOptions {
        extent: rect([120., 100., 200., 150.]),
        ..options()
    };
    let e = b.configure(opts, BrushReconfigure::Clamp).unwrap();
    assert_eq!(e[0].phase, BrushPhase::Cancel);
    assert!(!b.active());
    assert_eq!(b.selection(), Some(rect([120., 100., 200., 150.])));
    assert!(
        b.configure(opts, BrushReconfigure::Preserve)
            .unwrap()
            .is_empty()
    );
    b.configure(
        EditableBrushOptions {
            axis: BrushAxis::X,
            ..options()
        },
        BrushReconfigure::Preserve,
    )
    .unwrap();
    assert_eq!(b.selection(), Some(rect([120., 20., 200., 220.])));
    assert_eq!(b.handles().len(), 2);
    b.configure(
        EditableBrushOptions {
            extent: rect([280., 200., 300., 210.]),
            ..options()
        },
        BrushReconfigure::Clamp,
    )
    .unwrap();
    assert_eq!(b.selection(), None);
    b.set_selection(Some(rect([285., 202., 295., 208.])))
        .unwrap();
    b.configure(options(), BrushReconfigure::Clear).unwrap();
    assert_eq!(b.selection(), None);
}

#[test]
fn keyboard_nudge_and_clear_use_user_lifecycle_and_same_bounded_geometry() {
    let mut b = seeded();
    let e = b
        .apply(EditableBrushInput::Nudge {
            target: BrushTarget::Selection,
            delta: (1_000., -1_000.),
        })
        .unwrap();
    assert_eq!(b.selection(), Some(rect([170., 20., 310., 120.])));
    assert_eq!(e.len(), 3);
    assert!(
        e.iter()
            .all(|e| e.source == BrushSource::User && e.session.is_none())
    );
    assert!(
        b.apply(EditableBrushInput::Nudge {
            target: BrushTarget::Selection,
            delta: (1., -1.)
        })
        .unwrap()
        .is_empty()
    );
    b.apply(EditableBrushInput::Nudge {
        target: BrushTarget::Handle(BrushHandle::W),
        delta: (140., 0.),
    })
    .unwrap();
    assert_eq!(b.selection(), None);
    assert!(b.apply(EditableBrushInput::Clear).unwrap().is_empty());
    b.set_selection(Some(rect([40., 40., 100., 100.]))).unwrap();
    let s = begin(&mut b, BrushTarget::Selection, (70., 70.));
    let e = b.apply(EditableBrushInput::Clear).unwrap();
    assert_eq!(e[0].phase, BrushPhase::Cancel);
    assert_eq!(e.last().unwrap().mode, BrushMode::Clear);
    assert_eq!(b.selection(), None);
    assert!(
        b.apply(EditableBrushInput::End {
            session: s,
            id: 4,
            point: (100., 100.)
        })
        .unwrap()
        .is_empty()
    );
}

#[test]
fn handle_hit_order_and_axis_availability_share_exact_public_rectangles() {
    let b = seeded();
    assert_eq!(
        b.hit_test((80., 70.)).unwrap(),
        Some(BrushTarget::Handle(BrushHandle::Nw))
    );
    assert_eq!(
        b.hit_test((150., 120.)).unwrap(),
        Some(BrushTarget::Selection)
    );
    assert_eq!(b.hit_test((20., 30.)).unwrap(), Some(BrushTarget::Overlay));
    assert_eq!(b.hit_test((0., 0.)).unwrap(), None);
    assert!(b.hit_test((f64::NAN, 0.)).is_err());
    let mut b = EditableBrush::new(EditableBrushOptions {
        axis: BrushAxis::X,
        ..options()
    })
    .unwrap();
    b.set_selection(Some(rect([80., 70., 220., 170.]))).unwrap();
    let before = b.clone();
    assert!(
        b.apply(EditableBrushInput::Begin {
            id: 4,
            point: (150., 20.),
            target: BrushTarget::Handle(BrushHandle::N),
            modifiers: BrushModifiers::default()
        })
        .is_err()
    );
    assert_eq!(b, before);
}

#[test]
fn space_keeps_the_actual_clipped_center_size_instead_of_an_oversized_baseline() {
    let official = include_str!("fixtures/editable-brush-d3-edge-cases.csv")
        .lines()
        .find(|line| line.starts_with("S|center-clamped-space-outside-extent|2|"))
        .unwrap()
        .split('|')
        .collect::<Vec<_>>();
    // The unmodified package actually escapes its extent. The checked Rust
    // profile locks the already displayed size instead of retaining that bug.
    assert_eq!(
        parse_rect(official[9]),
        Some(rect([-100., -80., 600., 180.]))
    );
    for (axis, handle, start, far) in [
        (BrushAxis::X, BrushHandle::W, (0., 50.), (-500., 50.)),
        (BrushAxis::Y, BrushHandle::N, (50., 0.), (50., -500.)),
        (BrushAxis::XY, BrushHandle::Nw, (0., 0.), (-500., -500.)),
    ] {
        let mut b = EditableBrush::new(EditableBrushOptions {
            axis,
            extent: rect([-100., -80., 200., 180.]),
            handle_size: 10.,
            modifiers: true,
        })
        .unwrap();
        b.set_selection(Some(rect([0., 0., 100., 100.]))).unwrap();
        let mut modifiers = BrushModifiers {
            alt: true,
            ..Default::default()
        };
        b.apply(EditableBrushInput::Begin {
            id: 4,
            point: start,
            target: BrushTarget::Handle(handle),
            modifiers,
        })
        .unwrap();
        let session = b.session().unwrap();
        b.apply(EditableBrushInput::Move {
            session,
            id: 4,
            point: far,
        })
        .unwrap();
        let clipped = b.selection();
        modifiers.space = true;
        assert!(
            b.apply(EditableBrushInput::Modifiers { session, modifiers })
                .unwrap()
                .is_empty()
        );
        assert_eq!(b.mode(), Some(BrushMode::Space));
        assert_eq!(b.selection(), clipped);
        b.apply(EditableBrushInput::Move {
            session,
            id: 4,
            point: (-700., -600.),
        })
        .unwrap();
        assert_eq!(b.selection(), clipped);
        b.apply(EditableBrushInput::End {
            session,
            id: 4,
            point: (-700., -600.),
        })
        .unwrap();
        assert!(!b.active());
    }
}

#[test]
fn empty_keyboard_nudges_are_noop_but_invalid_target_or_delta_remains_rejected() {
    let mut b = EditableBrush::new(EditableBrushOptions {
        axis: BrushAxis::X,
        ..options()
    })
    .unwrap();
    let before = b.clone();
    for target in [
        BrushTarget::Selection,
        BrushTarget::Handle(BrushHandle::W),
        BrushTarget::Handle(BrushHandle::E),
    ] {
        assert!(
            b.apply(EditableBrushInput::Nudge {
                target,
                delta: (1., 0.)
            })
            .unwrap()
            .is_empty()
        );
        assert_eq!(b, before);
    }
    for input in [
        EditableBrushInput::Nudge {
            target: BrushTarget::Overlay,
            delta: (1., 0.),
        },
        EditableBrushInput::Nudge {
            target: BrushTarget::Handle(BrushHandle::N),
            delta: (1., 0.),
        },
        EditableBrushInput::Nudge {
            target: BrushTarget::Selection,
            delta: (f64::NAN, 0.),
        },
    ] {
        assert!(b.apply(input).is_err());
        assert_eq!(b, before);
    }
    b.set_selection(Some(rect([80., 20., 220., 220.]))).unwrap();
    b.apply(EditableBrushInput::Clear).unwrap();
    assert!(
        b.apply(EditableBrushInput::Nudge {
            target: BrushTarget::Handle(BrushHandle::E),
            delta: (1., 0.)
        })
        .unwrap()
        .is_empty()
    );
}

#[test]
fn extreme_pointer_and_modifier_sequences_keep_every_accepted_step_bounded_and_atomic() {
    let mut accepted_steps = 0;
    for axis in [BrushAxis::X, BrushAxis::Y, BrushAxis::XY] {
        for seed in 1..=64u64 {
            let extent = rect([-100., -80., 200., 180.]);
            let mut b = EditableBrush::new(EditableBrushOptions {
                axis,
                extent,
                handle_size: 10.,
                modifiers: true,
            })
            .unwrap();
            b.set_selection(Some(rect([0., 0., 100., 100.]))).unwrap();
            let handles = b.handles();
            let target = match seed % 10 {
                0 => BrushTarget::Overlay,
                1 => BrushTarget::Selection,
                n => BrushTarget::Handle(handles[(n as usize - 2) % handles.len()].handle),
            };
            let point = if let BrushTarget::Handle(h) = target {
                let r = handles.iter().find(|r| r.handle == h).unwrap().bounds;
                ((r.x0 + r.x1) / 2., (r.y0 + r.y1) / 2.)
            } else {
                (50., 50.)
            };
            b.apply(EditableBrushInput::Begin {
                id: 4,
                point,
                target,
                modifiers: BrushModifiers::default(),
            })
            .unwrap();
            let session = b.session().unwrap();
            let mut rng = seed;
            let mut point = point;
            for step in 0..256 {
                rng ^= rng << 13;
                rng ^= rng >> 7;
                rng ^= rng << 17;
                let modifiers = BrushModifiers {
                    alt: rng & 1 != 0,
                    shift: rng & 2 != 0,
                    space: rng & 4 != 0,
                    meta: false,
                    ctrl: false,
                };
                b.apply(EditableBrushInput::Modifiers { session, modifiers })
                    .unwrap();
                point = if step % 16 == 0 {
                    (
                        (if rng & 8 != 0 { 1. } else { -1. }) * 1_000_000.,
                        (if rng & 16 != 0 { 1. } else { -1. }) * 1_000_000.,
                    )
                } else {
                    (
                        (rng % 1000) as f64 - 500.,
                        ((rng >> 16) % 1000) as f64 - 500.,
                    )
                };
                b.apply(EditableBrushInput::Move {
                    session,
                    id: 4,
                    point,
                })
                .unwrap();
                let selected = b.selection().unwrap();
                selected.validate().unwrap();
                assert!(
                    selected.x0 >= extent.x0
                        && selected.y0 >= extent.y0
                        && selected.x1 <= extent.x1
                        && selected.y1 <= extent.y1,
                    "{axis:?} seed{seed} step{step}"
                );
                assert_eq!(b.session(), Some(session));
                assert!(b.active());
                for h in b.handles() {
                    h.bounds.validate().unwrap();
                }
                if step % 16 == 0 {
                    let before = b.clone();
                    assert!(
                        b.apply(EditableBrushInput::Move {
                            session,
                            id: 4,
                            point: (f64::NAN, point.1)
                        })
                        .is_err()
                    );
                    assert_eq!(b, before);
                    assert!(
                        b.apply(EditableBrushInput::End {
                            session,
                            id: 99,
                            point
                        })
                        .unwrap()
                        .is_empty()
                    );
                    assert_eq!(b, before);
                }
                accepted_steps += 1;
            }
            b.apply(EditableBrushInput::End {
                session,
                id: 4,
                point,
            })
            .unwrap();
            assert!(!b.active());
        }
    }
    assert_eq!(accepted_steps, 49_152);
}
