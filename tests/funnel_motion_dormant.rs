//! Inactive native-role memory belongs to the still-mounted keyed Part.
//! Reference: Git 0dae2c3 Part.tsx calls both useAnimatedPath hooks and useSpring
//! before its conditional border path; Parts.tsx maps by the typed business ID.
//! The analytic oscillator below is independent of Kartis's spring evaluator.
use std::cell::Cell;
use std::rc::Rc;

use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use kartis::motion::PropertyState;
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 400.,
        height: 200.,
        margin: Margin {
            left: 0.,
            right: 0.,
            top: 0.,
            bottom: 0.,
        },
        ..Default::default()
    }
}
fn options(border: f64, opacity: f64) -> FunnelLayoutOptions {
    FunnelLayoutOptions {
        size: FunnelSize::DatumPixels,
        interpolation: FunnelInterpolation::Linear,
        border_width: border,
        border_opacity: 1.,
        fill_opacity: opacity,
        labels: None,
        before: None,
        after: None,
        legend: false,
        ..Default::default()
    }
}
fn source(width: f64) -> Vec<FunnelDatum> {
    let mut datum = FunnelDatum::new("a", 8.);
    datum.size = Some(width);
    datum.color = Some(Color(10, 20, 30, 255));
    vec![datum]
}
fn frame(width: f64, border: f64, opacity: f64) -> FunnelSvgFrame {
    FunnelSvgFrame::new(&config(), &source(width), &options(border, opacity)).unwrap()
}
fn near(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-10,
        "{actual:.17} != {expected:.17}"
    );
}

// m=1,k=170,c=26 => damped frequency exactly 1. No rest snapping is
// involved in these short intervals. This uses initial value AND velocity.
fn reference(value: f64, velocity: f64, target: f64, seconds: f64) -> (f64, f64) {
    let y = value - target;
    let b = velocity + 13. * y;
    let e = (-13. * seconds).exp();
    let (sin, cos) = seconds.sin_cos();
    (
        target + e * (y * cos + b * sin),
        e * ((b - 13. * y) * cos + (-13. * b - y) * sin),
    )
}
fn border(model: &FunnelMotion) -> PropertyState {
    model
        .frame()
        .part_property(
            0,
            FunnelMotionPartRole::Border,
            FunnelMotionMarkChannel::StrokeWidth,
        )
        .unwrap()
        .expect("keyed Part keeps its border spring without a native border Mark")
}
fn path_state(model: &FunnelMotion) -> PropertyState {
    model
        .frame()
        .part_path_progress(0, FunnelMotionPartRole::Body)
        .unwrap()
        .unwrap()
}
fn actual_body(model: &FunnelMotion) -> &Mark {
    let index = model.frame().parts()[0].body_mark_index.unwrap();
    &model.frame().scene().marks[index]
}

#[test]
fn first_border_activation_animates_from_the_mounted_zero_width_spring() {
    let zero = frame(120., 0., 1.);
    assert!(zero.border_mark(0).unwrap().is_none());
    let mut model = FunnelMotion::new(zero, Default::default()).unwrap();
    near(border(&model).value, 0.);
    model.retarget(frame(120., 6., 1.)).unwrap();
    near(border(&model).value, 0.);
    near(border(&model).target, 6.);
    near(border(&model).velocity, 0.);
    assert!(!model.is_settled());
    model.tick(0.05).unwrap();
    let (value, velocity) = reference(0., 0., 6., 0.05);
    near(value, 0.8365095561160942);
    near(border(&model).value, value);
    near(border(&model).velocity, velocity);
    let slot = model.frame().parts()[0].border_mark_index.unwrap();
    near(
        model.frame().scene().marks[slot].stroke.unwrap().width,
        value,
    );
}

#[test]
fn hidden_border_ticks_and_reentry_preserves_its_nonzero_velocity() {
    let six = frame(120., 6., 1.);
    let id = six.border_mark(0).unwrap().unwrap().id.clone();
    let mut model = FunnelMotion::new(six, Default::default()).unwrap();
    model.retarget(frame(120., 0., 1.)).unwrap();
    assert!(model.frame().parts()[0].border_mark_index.is_none());
    assert!(model.frame().scene().marks.iter().all(|mark| mark.id != id));
    assert!(
        model
            .frame()
            .mark_property(&id, FunnelMotionMarkChannel::StrokeWidth)
            .is_none()
    );
    assert!(
        !model.is_settled(),
        "inactive numeric work must keep the clock alive"
    );
    model.tick(0.05).unwrap();
    let (x, v) = reference(6., 0., 0., 0.05);
    near(border(&model).value, x);
    near(border(&model).velocity, v);
    let historical = model.frame().clone();
    model.retarget(frame(120., 6., 1.)).unwrap();
    near(border(&model).value, x);
    near(border(&model).velocity, v);
    near(border(&model).target, 6.);
    model.tick(0.05).unwrap();
    let (next, speed) = reference(x, v, 6., 0.05);
    near(next, 4.585738014944076);
    near(border(&model).value, next);
    near(border(&model).velocity, speed);
    assert!(
        next < x,
        "retargeting must not erase the old inward velocity"
    );
    near(
        historical
            .part_property(
                0,
                FunnelMotionPartRole::Border,
                FunnelMotionMarkChannel::StrokeWidth,
            )
            .unwrap()
            .unwrap()
            .value,
        x,
    );
    assert!(historical.parts()[0].border_mark_index.is_none());
}

#[test]
fn opacity_zero_keeps_the_actual_body_and_its_path_history_without_faking_a_role() {
    let initial = frame(120., 0., 1.);
    let invisible = frame(240., 0., 0.);
    let restored = frame(80., 0., 1.);
    let invisible_geometry = invisible.body_mark(0).unwrap().unwrap().geometry.clone();
    let mut model = FunnelMotion::new(initial, Default::default()).unwrap();
    model.retarget(invisible).unwrap();
    assert_eq!(actual_body(&model).opacity, 0.);
    model.tick(0.05).unwrap();
    let old = path_state(&model);
    near(old.value, reference(0., 0., 1., 0.05).0);
    assert!(old.velocity > 0.);
    model.retarget(restored).unwrap();
    assert_eq!(actual_body(&model).opacity, 1.);
    assert_eq!(
        actual_body(&model).geometry,
        invisible_geometry,
        "useAnimatedPath resets from the previous committed target, not the sampled contour"
    );
    near(path_state(&model).value, 0.);
    near(path_state(&model).velocity, 0.);
}

#[test]
fn a_truly_collapsed_body_keeps_progress_but_reentry_starts_at_its_zero_area_target() {
    let collapsed = frame(0., 0., 1.);
    assert!(collapsed.body_mark(0).unwrap().is_none());
    assert!(collapsed.scene().marks.is_empty());
    let mut model = FunnelMotion::new(frame(120., 0., 1.), Default::default()).unwrap();
    model.retarget(collapsed.clone()).unwrap();
    assert!(model.frame().parts()[0].body_mark_index.is_none());
    assert!(
        model.frame().scene().marks.is_empty(),
        "dormant state must not invent zero-area ink"
    );
    model.tick(0.05).unwrap();
    near(path_state(&model).value, reference(0., 0., 1., 0.05).0);
    assert!(
        model
            .frame()
            .part_geometry_states(0, FunnelMotionPartRole::Body)
            .unwrap()
            .is_some()
    );
    model.retarget(frame(240., 0., 1.)).unwrap();
    // Independent authored linear profile: one 200px-tall band, 66% blending,
    // zero width about x=200. Both halves of the closed area still exist in
    // the path generator although the target Scene has no body Mark.
    assert_eq!(
        actual_body(&model).geometry,
        Geometry::Path(vec![
            C::Move(P::new(200., 0.)),
            C::Line(P::new(200., 66.)),
            C::Line(P::new(200., 134.)),
            C::Line(P::new(200., 200.)),
            C::Line(P::new(200., 200.)),
            C::Line(P::new(200., 134.)),
            C::Line(P::new(200., 66.)),
            C::Line(P::new(200., 0.)),
            C::Close,
        ])
    );
    near(path_state(&model).value, 0.);
    assert_eq!(collapsed.source()[0].size, Some(0.));
}

fn edited_path(x: f64) -> Geometry {
    Geometry::Path(vec![
        C::Move(P::new(x, 20.)),
        C::Line(P::new(x + 10., 30.)),
        C::Close,
    ])
}
fn edited(width: f64, x: f64, red: u8, calls: &Cell<usize>) -> FunnelSvgFrame {
    let base = frame(width, 0., 1.);
    base.with_all_stage_factory(&mut |context| {
        calls.set(calls.get() + 1);
        let mut value = context.inherit(edited_path(x));
        value.body.fill = Some(Color(red, 20, 30, 255));
        value.body.fill_paint = None;
        value.retain_profile_border = false;
        Ok(value)
    })
    .unwrap()
}

#[test]
fn captured_custom_dormant_geometry_is_authoritative_and_never_replays_factories() {
    let calls = Cell::new(0);
    let a = edited(120., 20., 10, &calls);
    let hidden = edited(0., 120., 110, &calls);
    let b = edited(240., 220., 210, &calls);
    assert_eq!(
        calls.get(),
        3,
        "including the collapsed part's one capture callback"
    );
    assert!(hidden.body_mark(0).unwrap().is_none());
    let history = a.clone();
    let mut model = FunnelMotion::new(a, Default::default()).unwrap();
    model.retarget(hidden).unwrap();
    model.tick(0.05).unwrap();
    let color = model
        .frame()
        .part_property(
            0,
            FunnelMotionPartRole::Body,
            FunnelMotionMarkChannel::FillRed,
        )
        .unwrap()
        .unwrap();
    near(color.value, 10. + 100. * reference(0., 0., 1., 0.05).0);
    model.retarget(b).unwrap();
    assert_eq!(
        actual_body(&model).geometry,
        edited_path(120.),
        "must use the actual captured dormant path, never reconstruct a profile or use the latest sample"
    );
    let resumed = model
        .frame()
        .part_property(
            0,
            FunnelMotionPartRole::Body,
            FunnelMotionMarkChannel::FillRed,
        )
        .unwrap()
        .unwrap();
    near(resumed.value, color.value);
    near(resumed.velocity, color.velocity);
    near(path_state(&model).value, 0.);
    model.tick(0.05).unwrap();
    model.finish().unwrap();
    model.set_reduced_motion(true).unwrap();
    assert_eq!(
        calls.get(),
        3,
        "sampling, finish and reduced motion consume captured data only"
    );
    assert_eq!(
        history.body_mark(0).unwrap().unwrap().geometry,
        edited_path(20.)
    );
}

#[test]
fn an_uncalled_legacy_stage_factory_does_not_authorize_invented_profile_memory() {
    let calls = Cell::new(0);
    let captured = FunnelSvgFrame::with_stage_factory(
        &config(),
        &source(0.),
        &options(0., 1.),
        &mut |context| {
            calls.set(calls.get() + 1);
            Ok(context.inherit(edited_path(70.)))
        },
    )
    .unwrap();
    assert_eq!(
        calls.get(),
        0,
        "the legacy factory only visits drawable bodies"
    );
    assert!(captured.body_mark(0).unwrap().is_none());
    assert!(
        captured.scene().marks.is_empty(),
        "static construction stays legal"
    );
    assert!(
        FunnelMotion::new(captured.clone(), Default::default()).is_err(),
        "an absent custom descriptor must not be replaced by guessed profile geometry"
    );
    assert_eq!(
        calls.get(),
        0,
        "motion admission cannot replay a borrowed factory"
    );
    assert!(captured.scene().marks.is_empty());
}

#[test]
fn removing_a_source_part_destroys_its_memory_and_readding_it_is_fresh() {
    let mut model = FunnelMotion::new(frame(120., 6., 1.), Default::default()).unwrap();
    model.retarget(frame(120., 0., 1.)).unwrap();
    model.tick(0.05).unwrap();
    assert!(border(&model).velocity < 0.);
    model
        .retarget(FunnelSvgFrame::new(&config(), &[], &options(0., 1.)).unwrap())
        .unwrap();
    assert!(model.frame().parts().is_empty());
    assert!(
        model
            .frame()
            .part_property(
                0,
                FunnelMotionPartRole::Border,
                FunnelMotionMarkChannel::StrokeWidth
            )
            .is_err()
    );
    model.retarget(frame(120., 6., 1.)).unwrap();
    near(border(&model).value, 6.);
    near(border(&model).velocity, 0.);
    assert!(model.is_settled());
}

#[test]
fn finish_and_reduced_motion_include_inactive_channels_without_adding_paint() {
    for reduced in [false, true] {
        let mut model = FunnelMotion::new(frame(120., 6., 1.), Default::default()).unwrap();
        model.retarget(frame(120., 0., 1.)).unwrap();
        model.tick(0.05).unwrap();
        let old = model.frame().clone();
        assert!(!model.is_settled());
        if reduced {
            model.set_reduced_motion(true).unwrap();
        } else {
            model.finish().unwrap();
        }
        assert!(model.is_settled());
        near(border(&model).value, 0.);
        near(border(&model).velocity, 0.);
        assert!(model.frame().parts()[0].border_mark_index.is_none());
        near(
            old.part_property(
                0,
                FunnelMotionPartRole::Border,
                FunnelMotionMarkChannel::StrokeWidth,
            )
            .unwrap()
            .unwrap()
            .value,
            reference(6., 0., 0., 0.05).0,
        );
        model.set_reduced_motion(false).unwrap();
        model.retarget(frame(120., 6., 1.)).unwrap();
        model.tick(0.05).unwrap();
        near(border(&model).value, 0.8365095561160942);
    }
}

#[test]
fn inactive_roles_are_budgeted_and_failed_updates_preserve_every_old_state() {
    let collapsed = frame(0., 0., 1.);
    assert!(
        FunnelMotion::new(
            collapsed.clone(),
            FunnelMotionOptions {
                max_marks: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    let mut model = FunnelMotion::new(
        collapsed,
        FunnelMotionOptions {
            max_marks: 2,
            ..Default::default()
        },
    )
    .unwrap();
    let old = model.frame().clone();
    assert!(
        model
            .set_options(FunnelMotionOptions {
                max_marks: 1,
                ..Default::default()
            })
            .is_err()
    );
    assert_eq!(model.frame(), &old);
    assert_eq!(model.options().max_marks, 2);
    let mut more = source(0.);
    let mut next = FunnelDatum::new("b", 4.);
    next.size = Some(0.);
    more.push(next);
    assert!(
        model
            .retarget(FunnelSvgFrame::new(&config(), &more, &options(0., 1.)).unwrap())
            .is_err()
    );
    assert_eq!(model.frame(), &old);
    for dt in [f64::NAN, f64::INFINITY, -0.05, 61.] {
        assert!(model.tick(dt).is_err());
        assert_eq!(model.frame(), &old);
    }
    assert!(
        model
            .frame()
            .part_path_progress(1, FunnelMotionPartRole::Body)
            .is_err()
    );
    assert!(
        model
            .frame()
            .part_geometry_states(1, FunnelMotionPartRole::Border)
            .is_err()
    );
}

#[test]
fn invalid_latent_styles_reject_the_whole_capture_and_preserve_the_old_frame() {
    let mut data = source(0.);
    let mut second = FunnelDatum::new("b", 4.);
    second.size = Some(0.);
    data.push(second);
    let base = FunnelSvgFrame::new(&config(), &data, &options(0., 1.)).unwrap();
    let old = base.clone();
    let old_scene = base.scene().clone();
    let old_source = base.source().to_vec();
    let old_styles = base.styles().to_vec();
    assert!(old_scene.marks.is_empty());

    for invalid in 0..3 {
        let mut visited = Vec::new();
        let result = base.with_all_stage_factory(&mut |context| {
            visited.push(context.source_index);
            let mut value = context.inherit(edited_path(30.));
            value.retain_profile_border = false;
            if context.source_index == 1 {
                if invalid == 0 {
                    value.body.opacity = f64::NAN;
                } else {
                    value.body.stroke = Some(GraphicStroke {
                        color: Color(20, 30, 40, 255),
                        width: if invalid == 1 { f64::NAN } else { -1. },
                        cap: StrokeCap::Butt,
                    });
                }
            }
            Ok(value)
        });
        assert!(
            result.is_err(),
            "invalid style {invalid} needs validation even without a paint slot"
        );
        assert_eq!(
            visited,
            [0, 1],
            "callback effects occurred and are not rolled back"
        );
        assert_eq!(base, old);
        assert_eq!(base.scene(), &old_scene);
        assert_eq!(base.source(), old_source);
        assert_eq!(base.styles(), old_styles);
        assert!((0..2).all(|i| base.body_mark(i).unwrap().is_none()));
    }
    let mut visited = Vec::new();
    let valid = base
        .with_all_stage_factory(&mut |context| {
            visited.push(context.source_index);
            let mut value = context.inherit(edited_path(30.));
            value.retain_profile_border = false;
            Ok(value)
        })
        .unwrap();
    assert_eq!(
        visited,
        [0, 1],
        "the same latent geometry with valid style succeeds"
    );
    assert!(valid.scene().marks.is_empty());
    FunnelMotion::new(valid, Default::default()).unwrap();
    assert_eq!(base.scene(), &old_scene);
}

#[test]
fn typed_raw_all_stage_composition_keeps_one_opaque_pool_without_extra_traits() {
    // Deliberately no Clone, Debug, PartialEq, Serialize, Send or Sync bounds.
    struct Raw {
        note: String,
        negative_zero: f64,
        local: Rc<Cell<u32>>,
    }
    let local = Rc::new(Cell::new(9));
    let raw = FunnelDataFrame::new(
        &config(),
        vec![FunnelDataDatum::new(
            Raw {
                note: "captured application row".into(),
                negative_zero: -0.,
                local: local.clone(),
            },
            source(0.).remove(0),
        )],
        &options(0., 1.),
    )
    .unwrap();
    let before = raw.clone();
    let old_scene = raw.computed().scene().clone();
    let calls = Cell::new(0);
    let capture = |x| {
        raw.computed()
            .with_all_stage_factory(&mut |context| {
                calls.set(calls.get() + 1);
                let data = raw.data(context.source_index).unwrap();
                assert_eq!(data.note, "captured application row");
                assert_eq!(data.negative_zero.to_bits(), (-0f64).to_bits());
                let mut value = context.inherit(edited_path(x));
                value.retain_profile_border = false;
                Ok(value)
            })
            .unwrap()
    };
    let a = raw.with_computed(capture(70.)).unwrap();
    let b = raw.with_computed(capture(170.)).unwrap();
    assert_eq!(calls.get(), 2);
    assert!(a.shares_source(&raw) && b.shares_source(&raw) && a.shares_source(&b));
    assert!(std::ptr::eq(a.data(0).unwrap(), raw.data(0).unwrap()));
    assert!(std::ptr::eq(b.data(0).unwrap(), raw.data(0).unwrap()));
    assert!(Rc::ptr_eq(&a.data(0).unwrap().local, &local));
    assert_ne!(
        a, b,
        "computed captures are distinct despite sharing the raw pool"
    );
    let mut model = FunnelMotion::new(a.computed().clone(), Default::default()).unwrap();
    assert_eq!(
        model.frame().part_geometry_origin(0).unwrap(),
        FunnelMotionGeometryOrigin::StageFactory
    );
    model.retarget(b.computed().clone()).unwrap();
    model.tick(0.05).unwrap();
    near(path_state(&model).value, reference(0., 0., 1., 0.05).0);
    assert!(model.frame().scene().marks.is_empty());
    model.finish().unwrap();
    model.set_reduced_motion(true).unwrap();
    assert_eq!(
        calls.get(),
        2,
        "motion never calls the application stage factory"
    );
    assert_eq!(local.get(), 9);
    assert_eq!(raw, before);
    assert_eq!(raw.computed().scene(), &old_scene);
    assert_eq!(
        a.data(0).unwrap().negative_zero.to_bits(),
        (-0f64).to_bits()
    );
}

#[test]
fn explicit_transparent_custom_fill_keeps_numeric_history_but_absent_fill_does_not() {
    let calls = Cell::new(0);
    let capture = |color: Option<Color>, x| {
        frame(120., 0., 1.)
            .with_all_stage_factory(&mut |context| {
                calls.set(calls.get() + 1);
                let mut value = context.inherit(edited_path(x));
                value.body.fill = color;
                value.body.fill_paint = None;
                value.body.stroke = None;
                value.retain_profile_border = false;
                Ok(value)
            })
            .unwrap()
    };
    let opaque = capture(Some(Color(100, 20, 30, 255)), 20.);
    let transparent = capture(Some(Color(200, 40, 60, 0)), 120.);
    let restored = capture(Some(Color(50, 80, 100, 255)), 220.);
    let absent = capture(None, 320.);
    assert_eq!(calls.get(), 4);
    let invisible = transparent.body_mark(0).unwrap().unwrap();
    assert!(
        invisible.fill.is_none() && invisible.fill_paint.is_none() && invisible.stroke.is_none()
    );
    let state = |m: &FunnelMotion, channel| {
        m.frame()
            .part_property(0, FunnelMotionPartRole::Body, channel)
            .unwrap()
            .unwrap()
    };
    let mut model = FunnelMotion::new(opaque, Default::default()).unwrap();
    model.retarget(transparent).unwrap();
    near(state(&model, FunnelMotionMarkChannel::FillRed).target, 200.);
    near(state(&model, FunnelMotionMarkChannel::FillAlpha).target, 0.);
    model.tick(0.05).unwrap();
    let red = state(&model, FunnelMotionMarkChannel::FillRed);
    let alpha = state(&model, FunnelMotionMarkChannel::FillAlpha);
    near(red.value, reference(100., 0., 200., 0.05).0);
    near(alpha.value, reference(255., 0., 0., 0.05).0);
    assert!(red.velocity > 0. && alpha.velocity < 0.);
    assert!(
        actual_body(&model).fill.is_none(),
        "numeric capture must not manufacture actual paint"
    );
    let history = model.frame().clone();
    model.retarget(restored).unwrap();
    let resumed_red = state(&model, FunnelMotionMarkChannel::FillRed);
    let resumed_alpha = state(&model, FunnelMotionMarkChannel::FillAlpha);
    near(resumed_red.value, red.value);
    near(resumed_red.velocity, red.velocity);
    near(resumed_alpha.value, alpha.value);
    near(resumed_alpha.velocity, alpha.velocity);
    model.tick(0.05).unwrap();
    near(
        state(&model, FunnelMotionMarkChannel::FillRed).value,
        reference(red.value, red.velocity, 50., 0.05).0,
    );
    near(
        state(&model, FunnelMotionMarkChannel::FillAlpha).value,
        reference(alpha.value, alpha.velocity, 255., 0.05).0,
    );
    assert!(actual_body(&model).fill.is_some());
    near(
        history
            .part_property(
                0,
                FunnelMotionPartRole::Body,
                FunnelMotionMarkChannel::FillAlpha,
            )
            .unwrap()
            .unwrap()
            .value,
        alpha.value,
    );
    assert!(
        history.scene().marks[history.parts()[0].body_mark_index.unwrap()]
            .fill
            .is_none()
    );
    let none = FunnelMotion::new(absent, Default::default()).unwrap();
    for channel in [
        FunnelMotionMarkChannel::FillRed,
        FunnelMotionMarkChannel::FillGreen,
        FunnelMotionMarkChannel::FillBlue,
        FunnelMotionMarkChannel::FillAlpha,
    ] {
        assert!(
            none.frame()
                .part_property(0, FunnelMotionPartRole::Body, channel)
                .unwrap()
                .is_none(),
            "an explicit missing fill is not a transparent-color target"
        );
    }
    assert_eq!(
        calls.get(),
        4,
        "none of the sampling or history reads replays capture callbacks"
    );
}
