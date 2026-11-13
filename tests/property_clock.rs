use kartis::{
    clock::ExplicitClock,
    motion::{PropertyMotion, PropertyMotionOptions},
    *,
};

fn scene(x: f64, y: f64) -> Scene {
    let mut scene = ChartConfig::default().scene().unwrap();
    scene.marks.push(Mark::new(
        "mark",
        Geometry::Rect {
            x,
            y,
            width: 30.,
            height: 40.,
        },
        Color(30, 80, 140, 255),
    ));
    scene
}

#[test]
fn independent_property_velocities_survive_clock_retarget_and_reduced_motion_cancels_work() {
    let mut direct =
        PropertyMotion::new(scene(10., 20.), PropertyMotionOptions::default()).unwrap();
    direct.retarget(scene(110., 220.)).unwrap();
    let mut clock = ExplicitClock::from_motion(direct.clone()).unwrap();
    direct.tick(0.08).unwrap();
    clock.step(0.08).unwrap();
    assert_eq!(clock.scene(), direct.scene());
    let x = clock.motion().mark_property("mark", "geometry.x").unwrap();
    let y = clock.motion().mark_property("mark", "geometry.y").unwrap();
    assert!(x.velocity > 0. && y.velocity > x.velocity);
    clock.retarget(scene(0., 400.)).unwrap();
    assert_eq!(
        clock.motion().mark_velocity("mark", "geometry.x"),
        Some(x.velocity)
    );
    assert_eq!(
        clock.motion().mark_velocity("mark", "geometry.y"),
        Some(y.velocity)
    );
    clock.step(0.001).unwrap();
    assert!(
        clock
            .motion()
            .mark_property("mark", "geometry.x")
            .unwrap()
            .value
            > x.value
    );
    assert!(
        clock
            .motion()
            .mark_property("mark", "geometry.y")
            .unwrap()
            .value
            > y.value
    );
    assert_eq!(clock.pending_frames(), 1);
    clock.set_reduced_motion(true);
    assert!(clock.is_settled());
    assert_eq!(clock.pending_frames(), 0);
    assert_eq!(clock.scene(), &scene(0., 400.));
}
