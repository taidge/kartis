use kartis::motion::PropertyMotionOptions;
use kartis::render::{to_canvas_script, to_svg};
use kartis::surface::{SurfaceInput, SurfaceModel, SurfaceOptions};
use kartis::*;

fn data() -> Vec<HierarchyDatum> {
    let mut data = vec![
        HierarchyDatum::new("root", None, 0.),
        HierarchyDatum::new("group", Some("root".into()), 0.),
        HierarchyDatum::new("a", Some("group".into()), 1.),
        HierarchyDatum::new("b", Some("group".into()), 3.),
        HierarchyDatum::new("outside", Some("root".into()), 2.),
    ];
    data[2].label = "A: Alpha".into();
    data
}

fn mark<'a>(scene: &'a Scene, id: &str) -> &'a Mark {
    scene.marks.iter().find(|mark| mark.id == id).unwrap()
}

fn circle(scene: &Scene, id: &str) -> (f64, f64, f64) {
    match mark(scene, id).geometry {
        Geometry::Circle { x, y, radius } => (x, y, radius),
        _ => panic!("expected circle geometry"),
    }
}

#[test]
fn sampled_focus_renders_and_picks_live_geometry_with_raw_metadata() {
    let config = ChartConfig::default();
    let mut treemap = TreemapLayoutOptions::default();
    treemap.style.value_format = ValueFormat::Fixed(2);
    let mut packing = CirclePackingLayoutOptions::default();
    packing.style.value_format = ValueFormat::Fixed(2);
    let mut icicle = IcicleLayoutOptions::default();
    icicle.style.value_format = ValueFormat::Fixed(2);
    for view in [
        HierarchyView::Treemap(treemap),
        HierarchyView::CirclePacking(packing),
        HierarchyView::Icicle(icicle),
    ] {
        let mut motion =
            HierarchyFocusMotion::new(&config, &data(), &view, Default::default()).unwrap();
        let old = mark(motion.scene(), "node:a").geometry.clone();
        let initial = motion
            .target_layout()
            .clone()
            .into_scene_interaction()
            .unwrap();
        initial.validate().unwrap();
        let mut surface =
            SurfaceModel::new(initial.scene.clone(), SurfaceOptions::default()).unwrap();
        surface.set_layout(initial).unwrap();
        assert!(surface.datum("node:outside").is_some());

        motion.focus(&config, "group", &view).unwrap();
        assert_eq!(mark(motion.scene(), "node:a").geometry, old);
        let target = mark(&motion.target_layout().scene, "node:a")
            .geometry
            .clone();
        assert_ne!(target, old);
        motion.tick(0.04).unwrap();
        let frame = motion.frame_interaction().unwrap();
        frame.validate().unwrap();
        let current = &mark(&frame.scene, "node:a").geometry;
        assert_ne!(*current, old);
        assert_ne!(*current, target);
        assert!(mark(&frame.scene, "node:outside").opacity > 0.);
        assert!(to_svg(&frame.scene).unwrap().contains("A: Alpha"));
        assert!(to_canvas_script(&frame.scene, "focus").is_ok());
        let [x, y] = match current {
            Geometry::Circle { x, y, .. } => [*x, *y],
            Geometry::Rect {
                x,
                y,
                width,
                height,
            } => [x + width / 2., y + height / 2.],
            _ => panic!("expected hierarchy geometry"),
        };
        surface.set_layout(frame).unwrap();
        assert_eq!(surface.plot(), ClipRect::plot(&config));
        assert!(surface.logical_datum("node:outside").is_none());
        surface
            .apply(SurfaceInput::Move {
                id: 1,
                point: (x, y),
            })
            .unwrap();
        assert_eq!(surface.hovered(), ["node:a"]);
        let active = surface.active_interaction(surface.hovered()).unwrap();
        assert_eq!(active.anchor, [x, y]);
        assert_eq!(active.rows[0].label, "A: Alpha");
        assert_eq!(active.rows[0].value, Some(1.));
        assert_eq!(active.rows[0].formatted_value.as_deref(), Some("1.00"));
        assert_eq!(
            motion.target_layout().node("a").unwrap().parent.as_deref(),
            Some("group")
        );

        for _ in 0..180 {
            motion.tick(1. / 60.).unwrap();
            if motion.is_settled() {
                break;
            }
        }
        assert!(motion.is_settled());
        assert_eq!(motion.scene(), &motion.target_layout().scene);
    }
}

#[test]
fn circle_focus_retains_proportions_and_retargets_from_the_sampled_zoom() {
    let config = ChartConfig::default();
    let view = HierarchyView::CirclePacking(Default::default());
    let mut motion =
        HierarchyFocusMotion::new(&config, &data(), &view, Default::default()).unwrap();
    let old_group = circle(motion.scene(), "node:group");
    let old_a = circle(motion.scene(), "node:a");
    motion.focus(&config, "group", &view).unwrap();
    let new_group = circle(&motion.target_layout().scene, "node:group");
    let plot = ClipRect::plot(&config);
    assert_eq!(new_group.0, plot.x + plot.width / 2.);
    assert_eq!(new_group.1, plot.y + plot.height / 2.);
    assert!(new_group.2 > old_group.2);

    motion.tick(0.06).unwrap();
    let sampled_group = circle(motion.scene(), "node:group");
    let sampled_a = circle(motion.scene(), "node:a");
    assert!(old_group.2 < sampled_group.2 && sampled_group.2 < new_group.2);
    let scale = sampled_group.2 / old_group.2;
    assert!((sampled_a.0 - (sampled_group.0 + (old_a.0 - old_group.0) * scale)).abs() < 1e-8);
    assert!((sampled_a.1 - (sampled_group.1 + (old_a.1 - old_group.1) * scale)).abs() < 1e-8);
    assert!((sampled_a.2 - old_a.2 * scale).abs() < 1e-8);

    motion.back(&config, &view).unwrap();
    assert_eq!(motion.controller().current(), Some("root"));
    assert_eq!(circle(motion.scene(), "node:group"), sampled_group);
    motion.tick(0.001).unwrap();
    assert!(circle(motion.scene(), "node:group").2 > sampled_group.2);
    motion.finish();
    assert_eq!(circle(motion.scene(), "node:group"), old_group);
}

#[test]
fn rejected_navigation_and_motion_budget_preserve_the_accepted_candidate() {
    let config = ChartConfig::default();
    let mut initial = TreemapLayoutOptions::default();
    initial.style.hidden.insert("b".into());
    let view = HierarchyView::Treemap(initial.clone());
    let options = PropertyMotionOptions {
        max_transient_marks: 4,
        ..Default::default()
    };
    let mut motion = HierarchyFocusMotion::new(&config, &data(), &view, options).unwrap();
    let accepted = motion.clone();

    initial.style.hidden.clear();
    initial.style.hidden.insert("a".into());
    let target = HierarchyView::Treemap(initial);
    let error = match motion.focus(&config, "group", &target) {
        Err(error) => error,
        Ok(_) => panic!("expected the transient union budget to reject the update"),
    };
    assert!(error.to_string().contains("transient union"));
    assert!(motion.focus(&config, "missing", &view).is_err());
    assert!(motion.tick(f64::NAN).is_err());
    assert_eq!(motion.scene(), accepted.scene());
    assert_eq!(motion.target_layout(), accepted.target_layout());
    assert_eq!(
        motion.controller().revision(),
        accepted.controller().revision()
    );
    assert_eq!(
        motion.controller().history(),
        accepted.controller().history()
    );
    assert_eq!(
        motion.controller().treemap_state(),
        accepted.controller().treemap_state()
    );

    motion.set_reduced_motion(true);
    motion.focus(&config, "group", &target).unwrap();
    assert!(motion.is_settled());
    assert_eq!(motion.controller().current(), Some("group"));
    assert!(!motion.scene().marks.iter().any(|mark| mark.id == "node:a"));
    motion.parent(&config, &target).unwrap();
    assert_eq!(motion.controller().current(), Some("root"));
    assert!(motion.is_settled());
}

#[test]
fn replacement_retains_surviving_focus_and_resize_refreshes_the_surface_plot() {
    let mut config = ChartConfig::default();
    let view = HierarchyView::Icicle(Default::default());
    let mut data = data();
    let mut motion = HierarchyFocusMotion::new(&config, &data, &view, Default::default()).unwrap();
    motion.focus(&config, "a", &view).unwrap();
    motion.tick(0.03).unwrap();
    data.retain(|node| node.id != "a");
    data.iter_mut().find(|node| node.id == "b").unwrap().value = 7.;
    motion.replace(&config, &data, &view).unwrap();
    assert_eq!(motion.controller().current(), Some("group"));
    assert_eq!(motion.target_layout().node("b").unwrap().value, 7.);
    let frame = motion.frame_interaction().unwrap();
    frame.validate().unwrap();
    assert_eq!(mark(&frame.scene, "node:b").value, 7.);
    let sites = &frame.interaction.as_ref().unwrap().sites;
    assert_eq!(
        sites.iter().find(|site| site.series == "b").unwrap().value,
        Some(7.)
    );
    assert!(sites.iter().all(|site| site.series != "a"));
    let mut surface = SurfaceModel::new(frame.scene.clone(), SurfaceOptions::default()).unwrap();
    surface.set_layout(frame).unwrap();

    config.width = 480.;
    config.margin.left = 24.;
    motion.relayout(&config, &view).unwrap();
    assert!(motion.is_settled());
    surface
        .set_layout(motion.frame_interaction().unwrap())
        .unwrap();
    assert_eq!(surface.plot(), ClipRect::plot(&config));
    assert_eq!(motion.controller().current(), Some("group"));
    motion.root(&config, &view).unwrap();
    assert_eq!(motion.controller().current(), Some("root"));
}
