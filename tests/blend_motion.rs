use kartis::*;

fn fixture(x: f64) -> Scene {
    let mut scene = ChartConfig {
        width: 120.,
        height: 80.,
        margin: Margin {
            left: 0.,
            right: 0.,
            top: 0.,
            bottom: 0.,
        },
        ..Default::default()
    }
    .scene()
    .unwrap();
    scene.background = Color(160, 120, 80, 255);
    scene.marks.push(Mark::new(
        "paint",
        Geometry::Rect {
            x,
            y: 20.,
            width: 55.,
            height: 40.,
        },
        Color(200, 100, 50, 255),
    ));
    scene.marks[0].series = "series".into();
    scene
}

fn profile(scene: &Scene, mode: BlendMode) -> MarkBlends {
    MarkBlends::new(scene, [("paint", mode)]).unwrap()
}

enum Solver {
    Scene(Box<motion::SpringMotion>),
    Properties(Box<motion::PropertyMotion>),
}

impl Solver {
    fn pair(scene: &Scene, mode: BlendMode) -> [Self; 2] {
        [
            Self::Scene(Box::new(
                motion::SpringMotion::new_with_blends(
                    scene.clone(),
                    Default::default(),
                    profile(scene, mode),
                )
                .unwrap(),
            )),
            Self::Properties(Box::new(
                motion::PropertyMotion::new_with_blends(
                    scene.clone(),
                    Default::default(),
                    profile(scene, mode),
                )
                .unwrap(),
            )),
        ]
    }

    fn frame(&self) -> (&Scene, &MarkBlends) {
        match self {
            Self::Scene(m) => (m.scene(), m.blends()),
            Self::Properties(m) => (m.scene(), m.blends()),
        }
    }

    fn retarget(&mut self, scene: Scene, modes: MarkBlends) -> Result<()> {
        match self {
            Self::Scene(m) => m.retarget_with_blends(scene, modes),
            Self::Properties(m) => m.retarget_with_blends(scene, modes),
        }
    }

    fn tick(&mut self, dt: f64) -> Result<()> {
        match self {
            Self::Scene(m) => m.tick(dt).map(|_| ()),
            Self::Properties(m) => m.tick(dt).map(|_| ()),
        }
    }

    fn reduced(&mut self) {
        match self {
            Self::Scene(m) => m.set_reduced_motion(true),
            Self::Properties(m) => m.set_reduced_motion(true),
        }
    }
}

#[test]
fn unchanged_mode_moves_the_same_identity_then_settles_to_exact_target() {
    let old = fixture(10.);
    let next = fixture(32.);
    let modes = profile(&next, BlendMode::Multiply);
    for mut solver in Solver::pair(&old, BlendMode::Multiply) {
        solver.retarget(next.clone(), modes.clone()).unwrap();
        solver.tick(0.05).unwrap();
        let (scene, active) = solver.frame();
        assert_eq!(active, &modes);
        assert_eq!(scene.marks.len(), 1);
        assert_eq!(scene.marks[0].id, "paint");
        let Geometry::Rect { x, .. } = scene.marks[0].geometry else {
            unreachable!()
        };
        assert!(x > 10. && x < 32., "{x}");
        for _ in 0..100 {
            solver.tick(0.03).unwrap();
        }
        assert_eq!(solver.frame(), (&next, &modes));
    }
}

#[test]
fn incompatible_same_id_and_interrupted_retarget_preserve_each_visible_mode() {
    let scene = fixture(10.);
    let screen = profile(&scene, BlendMode::Screen);
    let difference = profile(&scene, BlendMode::Difference);
    for mut solver in Solver::pair(&scene, BlendMode::Multiply) {
        solver.retarget(scene.clone(), screen.clone()).unwrap();
        let (frame, modes) = solver.frame();
        assert_eq!(frame.marks.len(), 2);
        modes.validate(frame).unwrap();
        let ghost = frame.marks.iter().find(|mark| mark.id != "paint").unwrap();
        assert!(ghost.label.is_empty());
        assert_eq!(ghost.series, "series");
        assert_eq!(ghost.opacity, 1.);
        assert_eq!(modes.mode(&ghost.id), BlendMode::Multiply);
        assert_eq!(modes.mode("paint"), BlendMode::Screen);
        assert_eq!(
            frame
                .marks
                .iter()
                .find(|m| m.id == "paint")
                .unwrap()
                .opacity,
            0.
        );

        solver.tick(0.05).unwrap();
        solver.retarget(scene.clone(), difference.clone()).unwrap();
        let (frame, modes) = solver.frame();
        assert_eq!(frame.marks.len(), 3);
        modes.validate(frame).unwrap();
        for mark in frame.marks.iter().filter(|m| m.id != "paint") {
            assert!(mark.label.is_empty() && mark.opacity > 0.);
        }
        let retired: Vec<_> = modes
            .iter()
            .filter_map(|(id, mode)| (id != "paint").then_some(mode))
            .collect();
        assert!(retired.contains(&BlendMode::Multiply) && retired.contains(&BlendMode::Screen));
        assert_eq!(modes.mode("paint"), BlendMode::Difference);

        solver.reduced();
        assert_eq!(solver.frame(), (&scene, &difference));
    }
}

#[test]
fn enter_exit_profiles_and_bad_retargets_do_not_change_the_accepted_frame() {
    let old = fixture(10.);
    let mut next = fixture(30.);
    next.marks[0].id = "new".into();
    let modes = MarkBlends::new(&next, [("new", BlendMode::Screen)]).unwrap();
    for mut solver in Solver::pair(&old, BlendMode::Multiply) {
        solver.retarget(next.clone(), modes.clone()).unwrap();
        solver.tick(0.04).unwrap();
        let accepted = (solver.frame().0.clone(), solver.frame().1.clone());
        assert_eq!(accepted.1.mode("paint"), BlendMode::Multiply);
        assert_eq!(accepted.1.mode("new"), BlendMode::Screen);
        assert!(
            accepted
                .0
                .marks
                .iter()
                .find(|m| m.id == "paint")
                .unwrap()
                .label
                .is_empty()
        );
        assert!(solver.retarget(old.clone(), modes.clone()).is_err());
        assert!(solver.tick(f64::NAN).is_err());
        assert_eq!(solver.frame(), (&accepted.0, &accepted.1));
        solver.reduced();
        assert_eq!(solver.frame(), (&next, &modes));
        solver
            .retarget(next.clone(), MarkBlends::default())
            .unwrap();
        assert_eq!(solver.frame().0, &next);
        assert!(solver.frame().1.is_empty());
    }
}

#[test]
fn property_immediate_topology_replacement_switches_mode_without_a_ghost() {
    let scene = fixture(10.);
    let options = motion::PropertyMotionOptions {
        topology: motion::FadeMode::Immediate,
        ..Default::default()
    };
    let mut solver = motion::PropertyMotion::new_with_blends(
        scene.clone(),
        options,
        profile(&scene, BlendMode::Multiply),
    )
    .unwrap();
    let next = profile(&scene, BlendMode::Screen);
    solver
        .retarget_with_blends(scene.clone(), next.clone())
        .unwrap();
    assert_eq!(solver.scene(), &scene);
    assert_eq!(solver.blends(), &next);
    assert!(solver.is_settled());
}

#[cfg(feature = "raster")]
#[test]
fn cpu_intermediate_paint_composites_old_then_new_against_the_live_backdrop() {
    use render::raster::{FontRegistry, render_raster_with_blends};
    let scene = fixture(10.);
    for mut solver in Solver::pair(&scene, BlendMode::Multiply) {
        solver
            .retarget(scene.clone(), profile(&scene, BlendMode::Screen))
            .unwrap();
        solver.tick(0.065).unwrap();
        let (frame, modes) = solver.frame();
        let mut expected = [160., 120., 80.];
        for item in frame.draw_items() {
            let DrawItem::Mark(index) = item else {
                continue;
            };
            let mark = &frame.marks[index];
            assert!(mark.opacity > 0. && mark.opacity < 1.);
            for (back, source) in expected.iter_mut().zip([200., 100., 50.]) {
                let ink = match modes.mode(&mark.id) {
                    BlendMode::Multiply => *back * source / 255.,
                    BlendMode::Screen => 255. - (255. - *back) * (255. - source) / 255.,
                    _ => unreachable!(),
                };
                *back += (ink - *back) * mark.opacity;
            }
        }
        let image =
            render_raster_with_blends(frame, &FontRegistry::new(), &Default::default(), modes)
                .unwrap();
        let actual = &image.rgba[(30 * 120 + 35) * 4..][..4];
        for (&actual, expected) in actual.iter().zip(expected) {
            assert!(
                (f64::from(actual) - expected).abs() <= 2.,
                "{actual} vs {expected}"
            );
        }
        assert_eq!(actual[3], 255);
    }
}
