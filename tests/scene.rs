use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::*;

fn scene() -> Scene {
    ChartConfig::default().scene().unwrap()
}
#[test]
fn geometry_validation_and_total_budget() {
    let mut s = scene();
    s.marks.push(Mark::new(
        "a",
        Geometry::Rect {
            x: 10.,
            y: 20.,
            width: 40.,
            height: 30.,
        },
        Color(0, 0, 0, 255),
    ));
    s.validate().unwrap();
    s.marks.push(s.marks[0].clone());
    assert!(s.validate().is_err());
    s.marks.pop();
    s.marks[0].geometry = Geometry::Path(vec![C::Line(P::new(0., 0.))]);
    assert!(s.validate().is_err());
    s.marks[0].geometry = Geometry::Circle {
        x: 0.,
        y: 0.,
        radius: f64::NAN,
    };
    assert!(s.validate().is_err());
    s.marks[0].geometry = Geometry::Path(vec![C::Move(P::new(0., 0.)); 200001]);
    assert!(s.validate().is_err());
}
#[test]
fn hit_tests_obey_order_holes_and_curves() {
    let mut s = scene();
    s.marks.push(Mark::new(
        "bottom",
        Geometry::Rect {
            x: 0.,
            y: 0.,
            width: 100.,
            height: 100.,
        },
        Color(1, 2, 3, 255),
    ));
    s.marks.push(Mark::new(
        "top",
        Geometry::Circle {
            x: 50.,
            y: 50.,
            radius: 20.,
        },
        Color(4, 5, 6, 255),
    ));
    assert_eq!(s.hit_test(50., 50.).unwrap().id, "top");
    assert_eq!(s.hit_test(10., 10.).unwrap().id, "bottom");
    // Oppositely wound inner contour forms a donut hole.
    s.marks.push(Mark::new(
        "ring",
        Geometry::Path(vec![
            C::Move(P::new(0., 0.)),
            C::Line(P::new(100., 0.)),
            C::Line(P::new(100., 100.)),
            C::Line(P::new(0., 100.)),
            C::Close,
            C::Move(P::new(25., 25.)),
            C::Line(P::new(25., 75.)),
            C::Line(P::new(75., 75.)),
            C::Line(P::new(75., 25.)),
            C::Close,
        ]),
        Color(0, 0, 0, 255),
    ));
    assert_eq!(s.hit_test(50., 50.).unwrap().id, "top");
    assert_eq!(s.hit_test(10., 10.).unwrap().id, "ring");
    assert!(s.hit_test(f64::NAN, 0.).is_none());
}
#[test]
fn animation_matches_ids_rejects_topology_and_reaches_endpoints() {
    let mut a = scene();
    a.marks.push(Mark::new(
        "a",
        Geometry::Circle {
            x: 10.,
            y: 10.,
            radius: 2.,
        },
        Color(0, 0, 0, 255),
    ));
    let mut b = a.clone();
    b.marks[0].geometry = Geometry::Circle {
        x: 30.,
        y: 20.,
        radius: 6.,
    };
    b.marks[0].fill = Some(Color(200, 100, 50, 255));
    assert_eq!(a.interpolate(&b, 0.).unwrap(), a);
    assert_eq!(a.interpolate(&b, 1.).unwrap(), b);
    assert_eq!(
        a.interpolate(&b, 0.5).unwrap().marks[0].geometry,
        Geometry::Circle {
            x: 20.,
            y: 15.,
            radius: 4.
        }
    );
    b.marks[0].id = "b".into();
    assert!(a.interpolate(&b, 0.5).is_err());
    assert!(a.interpolate(&a, f64::NAN).is_err());
}
#[test]
fn interaction_handles_scaling_hiding_and_stale_selection() {
    let mut s = scene();
    let mut mark = Mark::new(
        "a",
        Geometry::Circle {
            x: 100.,
            y: 100.,
            radius: 20.,
        },
        Color(0, 0, 0, 255),
    );
    mark.series = "s".into();
    s.marks.push(mark);
    let mut i = interaction::Interaction::default();
    assert_eq!(i.pointer(&s, 100., 100.), Some("a"));
    i.activate();
    assert_eq!(i.selected.as_deref(), Some("a"));
    i.toggle_series("s");
    assert!(i.pointer(&s, 100., 100.).is_none());
    i.reconcile(&s);
    assert!(i.selected.is_none());
    assert_eq!(
        interaction::pointer_coordinates(&s, (210., 120.), (10., 10., 1600., 880.)).unwrap(),
        (100., 55.)
    );
}
