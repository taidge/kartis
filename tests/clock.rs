use std::cell::Cell;
use std::rc::Rc;

use kartis::clock::*;
use kartis::motion::*;
use kartis::*;
fn scene(x: f64) -> Scene {
    let mut s = ChartConfig::default().scene().unwrap();
    s.marks.push(Mark::new(
        "mark",
        Geometry::Rect {
            x,
            y: 20.,
            width: 30.,
            height: 40.,
        },
        Color(0, 0, 0, 255),
    ));
    s
}
fn motion() -> SpringMotion {
    SpringMotion::new(scene(10.), SpringConfig::default()).unwrap()
}
#[test]
fn explicit_clock_accepts_an_already_running_motion_without_losing_the_first_step() {
    let mut expected = motion();
    expected.retarget(scene(110.)).unwrap();
    let mut clock = ExplicitClock::from_motion(expected.clone()).unwrap();
    expected.tick(0.1).unwrap();
    clock.step(0.1).unwrap();
    assert_eq!(clock.scene(), expected.scene());
    assert_eq!(clock.motion().velocity(), expected.velocity());
    assert!(clock.motion().progress() > 0.);
}
#[test]
fn driver_requests_one_frame_at_a_time_and_stops_scheduling_at_rest() {
    let scheduler = ExplicitScheduler::default();
    let mut driver = FrameDriver::new(motion(), scheduler.clone()).unwrap();
    assert_eq!(scheduler.request_count(), 0);
    driver.retarget(scene(110.)).unwrap();
    driver.next_frame().unwrap();
    assert_eq!(scheduler.request_count(), 1);
    driver.advance(10.).unwrap();
    assert_eq!(driver.motion().progress(), 0.);
    assert_eq!(scheduler.pending_count(), 1);
    driver.advance(10.1).unwrap();
    assert!(driver.motion().progress() > 0.);
    let mut time = 10.1;
    for _ in 0..1000 {
        time += 0.016;
        driver.advance(time).unwrap();
        if driver.motion().is_settled() {
            break;
        }
    }
    assert!(driver.motion().is_settled());
    assert!(driver.pending_ticket().is_none());
    assert_eq!(scheduler.pending_count(), 0);
    assert_eq!(driver.scene(), &scene(110.));
}
#[test]
fn stopped_and_stale_tickets_cannot_consume_new_requests() {
    let scheduler = ExplicitScheduler::default();
    let mut driver = FrameDriver::new(motion(), scheduler.clone()).unwrap();
    driver.retarget(scene(110.)).unwrap();
    let old = *driver.pending_ticket().unwrap();
    driver.stop();
    assert_eq!(scheduler.pending_count(), 0);
    assert_eq!(scheduler.cancellation_count(), 1);
    driver.advance_frame(old, f64::NAN).unwrap();
    driver.resume().unwrap();
    let new = *driver.pending_ticket().unwrap();
    assert_ne!(old, new);
    driver.advance_frame(old, 100.).unwrap();
    assert_eq!(driver.pending_ticket(), Some(&new));
    driver.advance_frame(new, 1.).unwrap();
    driver.advance(1.1).unwrap();
    assert!(driver.motion().progress() > 0.);
}
#[test]
fn dropping_an_owned_driver_cancels_host_work() {
    let scheduler = ExplicitScheduler::default();
    let app = glory::App::memory();
    let captured = scheduler.clone();
    app.mount(glory::ui::widget(move |scope| {
        let mut motion = motion();
        motion.retarget(scene(110.)).unwrap();
        scope.retain(FrameDriver::new(motion, captured.clone()).unwrap())
    }))
    .unwrap();
    assert_eq!(scheduler.pending_count(), 1);
    app.clear().unwrap();
    assert_eq!(scheduler.pending_count(), 0);
    assert_eq!(scheduler.cancellation_count(), 1);
}
#[test]
fn reduced_motion_finishes_the_scene_and_cancels_the_pending_frame() {
    let scheduler = ExplicitScheduler::default();
    let mut driver = FrameDriver::new(motion(), scheduler.clone()).unwrap();
    driver.retarget(scene(110.)).unwrap();
    let old = *driver.pending_ticket().unwrap();
    driver.set_reduced_motion(true);
    assert_eq!(driver.scene(), &scene(110.));
    assert_eq!(scheduler.pending_count(), 0);
    driver.advance_frame(old, 20.).unwrap();
    assert_eq!(driver.scene(), &scene(110.));
    driver.set_reduced_motion(false);
    driver.retarget(scene(210.)).unwrap();
    assert_eq!(scheduler.pending_count(), 1);
}
#[test]
fn invalid_timestamps_keep_the_current_frame_and_outstanding_ticket() {
    let scheduler = ExplicitScheduler::default();
    let mut driver = FrameDriver::new(motion(), scheduler).unwrap();
    driver.retarget(scene(110.)).unwrap();
    let ticket = *driver.pending_ticket().unwrap();
    let before = driver.scene().clone();
    for time in [f64::NAN, -1., f64::INFINITY] {
        assert!(driver.advance_frame(ticket, time).is_err());
        assert_eq!(driver.pending_ticket(), Some(&ticket));
        assert_eq!(driver.scene(), &before);
    }
    driver.advance(10.).unwrap();
    let ticket = *driver.pending_ticket().unwrap();
    assert!(driver.advance(9.).is_err());
    assert_eq!(driver.pending_ticket(), Some(&ticket));
}
#[test]
fn host_pause_cap_is_explicit_and_configurable() {
    let scheduler = ExplicitScheduler::default();
    let mut driver = FrameDriver::new(motion(), scheduler).unwrap();
    driver.retarget(scene(110.)).unwrap();
    driver.advance(0.).unwrap();
    driver.advance(10.).unwrap();
    let mut expected = motion();
    expected.retarget(scene(110.)).unwrap();
    expected.tick(0.25).unwrap();
    assert_eq!(driver.scene(), expected.scene());
    assert_eq!(driver.max_frame_delta(), 0.25);
    assert!(driver.set_max_frame_delta(0.).is_err());
    driver.set_max_frame_delta(1.).unwrap();
    assert_eq!(driver.max_frame_delta(), 1.);
}
#[derive(Clone)]
struct FailingScheduler {
    fail: Rc<Cell<bool>>,
    next: u64,
}
impl FrameScheduler for FailingScheduler {
    type Ticket = u64;
    fn request_frame(&mut self) -> Result<u64> {
        if self.fail.get() {
            return Err(ChartError("scheduler unavailable".into()));
        }
        self.next += 1;
        Ok(self.next)
    }
    fn cancel_frame(&mut self, _: u64) {}
}
#[test]
fn scheduler_failure_retains_the_previous_delivered_scene_and_can_retry() {
    let fail = Rc::new(Cell::new(true));
    let scheduler = FailingScheduler {
        fail: fail.clone(),
        next: 0,
    };
    let mut driver = FrameDriver::new(motion(), scheduler).unwrap();
    assert!(driver.retarget(scene(110.)).is_err());
    assert_eq!(driver.scene(), &scene(10.));
    fail.set(false);
    driver.retarget(scene(110.)).unwrap();
    driver.advance(0.).unwrap();
    let before = driver.scene().clone();
    fail.set(true);
    assert!(driver.advance(0.1).is_err());
    assert_eq!(driver.scene(), &before);
    assert!(driver.pending_ticket().is_none());
    fail.set(false);
    driver.next_frame().unwrap();
    driver.advance(0.1).unwrap();
    assert!(driver.motion().progress() > 0.);
}
#[test]
fn explicit_clock_adapter_has_no_first_frame_time_loss_and_pauses_deterministically() {
    let mut clock = ExplicitClock::new(scene(10.), SpringConfig::default()).unwrap();
    clock.step(10.).unwrap();
    clock.retarget(scene(110.)).unwrap();
    clock.step(0.1).unwrap();
    let mut expected = motion();
    expected.retarget(scene(110.)).unwrap();
    expected.tick(0.1).unwrap();
    let Geometry::Rect { x: actual, .. } = clock.scene().marks[0].geometry else {
        panic!()
    };
    let Geometry::Rect { x: expected, .. } = expected.scene().marks[0].geometry else {
        panic!()
    };
    assert!((actual - expected).abs() < 1e-9);
    clock.stop();
    let before = clock.scene().clone();
    clock.step(1.).unwrap();
    assert_eq!(clock.scene(), &before);
    clock.resume().unwrap();
    clock.step(0.1).unwrap();
    assert_ne!(clock.scene(), &before);
    clock.set_reduced_motion(true);
    assert_eq!(clock.scene(), &scene(110.));
    assert_eq!(clock.pending_frames(), 0);
}
#[test]
fn manual_scheduler_queue_has_a_finite_budget() {
    let mut scheduler = ExplicitScheduler::default();
    for _ in 0..4096 {
        scheduler.request_frame().unwrap();
    }
    assert!(scheduler.request_frame().is_err());
    assert_eq!(scheduler.pending_count(), 4096);
}
