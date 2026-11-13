use std::collections::BTreeSet;
use std::f64::consts::{FRAC_PI_2, TAU};

use glory::render::{GraphicStroke, PathCommand as C, StrokeCap};
use kartis::*;
fn config() -> ChartConfig {
    ChartConfig {
        width: 400.,
        height: 400.,
        margin: Margin {
            left: 40.,
            right: 40.,
            top: 40.,
            bottom: 40.,
        },
        ..Default::default()
    }
}
fn data() -> Vec<RadialDatum> {
    vec![RadialDatum::new("a", 1.), RadialDatum::new("b", 3.)]
}
fn path(s: &Scene, id: &str) -> Vec<C> {
    s.marks
        .iter()
        .find(|m| m.id == id)
        .unwrap()
        .geometry
        .commands()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}
#[test]
fn rounded_annular_quarter_has_analytic_tangent_endpoints_and_real_cut_corners() {
    let c = config();
    let mut o = PieOptions {
        inner_radius: 0.5,
        labels: false,
        ..Default::default()
    };
    let sharp = pie(&c, &data(), &o).unwrap();
    o.style.corner_radius = 12.;
    let rounded = pie(&c, &data(), &o).unwrap();
    let p = path(&rounded, "a");
    let C::Move(first) = p[0] else { panic!() };
    close(first.x, 200.);
    close(first.y, 200. - (160_f64 * 136.).sqrt());
    let C::Cubic { first: control, .. } = p[1] else {
        panic!()
    };
    close(control.x, 200.);
    assert!(p.iter().filter(|x| matches!(x, C::Cubic { .. })).count() >= 6);
    assert_eq!(sharp.hit_test(201., 41.).unwrap().id, "a");
    assert!(rounded.hit_test(201., 41.).is_none());
    assert_eq!(rounded.hit_test(290., 110.).unwrap().id, "a");
    assert!(rounded.hit_test(201., 119.).is_none());
}
#[test]
fn clockwise_counterclockwise_small_angles_thin_rings_and_full_annuli_are_bounded() {
    let c = config();
    for (inner, extent, corner) in [
        (0., 0.000001, 1000.),
        (0.999999, FRAC_PI_2, 1000.),
        (0.6, TAU, 25.),
        (0.6, TAU - 1e-8, 25.),
    ] {
        for sign in [-1., 1.] {
            let mut o = PieOptions {
                inner_radius: inner,
                end_angle: extent * sign,
                labels: false,
                ..Default::default()
            };
            o.style.corner_radius = corner;
            let scene = pie(&c, &[RadialDatum::new("only", 1.)], &o).unwrap();
            scene.validate().unwrap();
            let p = path(&scene, "only");
            assert!(p.len() <= 40);
            for command in &p {
                let points = match command {
                    C::Move(p) | C::Line(p) => vec![p],
                    C::Cubic { first, second, end } => vec![first, second, end],
                    C::Close => vec![],
                };
                assert!(points.iter().all(|p| p.x.is_finite()
                    && p.y.is_finite()
                    && (p.x - 200.).abs() < 200.
                    && (p.y - 200.).abs() < 200.));
            }
            if extent == TAU {
                assert_eq!(p.iter().filter(|p| matches!(p, C::Move(_))).count(), 2);
                assert!(!p.iter().any(|p| matches!(p, C::Line(_))));
                assert!(scene.hit_test(200., 200.).is_none());
            }
        }
    }
    let mut cw = PieOptions {
        inner_radius: 0.5,
        labels: false,
        ..Default::default()
    };
    cw.style.corner_radius = 12.;
    let a = pie(&c, &data(), &cw).unwrap();
    cw.end_angle = -TAU;
    let b = pie(&c, &data(), &cw).unwrap();
    for (x, y) in [(290., 110.), (201., 41.), (200., 200.), (140., 280.)] {
        assert_eq!(
            a.hit_test(x, y).map(|m| m.id.as_str()),
            b.hit_test(400. - x, y).map(|m| m.id.as_str())
        );
    }
}
#[test]
fn labels_formats_skips_leaders_ownership_and_active_radii_are_real() {
    let c = config();
    let mut o = PieOptions {
        inner_radius: 0.5,
        ..Default::default()
    };
    o.style.arc_labels.legacy_fit = false;
    o.style.arc_labels.radius_offset = 1.;
    o.style.arc_labels.text = PieLabelText::Percent(ValueFormat::Percent(1));
    o.style.arc_labels.skip_angle = 1.;
    o.style.arc_labels.style.size = Some(18.);
    o.style.link_labels.enabled = true;
    o.style.link_labels.text = PieLabelText::Value(ValueFormat::Fixed(2));
    o.style.link_labels.offset = 5.;
    o.style.link_labels.diagonal_length = 12.;
    o.style.link_labels.straight_length = 20.;
    o.style.link_labels.thickness = 3.;
    o.style.link_labels.color = PieLinkColor::Slice;
    o.style.active_ids.insert("a".into());
    o.style.active_inner_radius_offset = 10.;
    o.style.active_outer_radius_offset = 15.;
    let scene = pie(&c, &data(), &o).unwrap();
    assert_eq!(scene.labels.len(), 4);
    assert_eq!(scene.label_series.len(), 4);
    let label = scene.labels.iter().find(|l| l.text == "25.0%").unwrap();
    close(label.x, 200. + 175. / 2_f64.sqrt());
    close(label.y, 200. - 175. / 2_f64.sqrt());
    let leader = scene
        .marks
        .iter()
        .find(|m| m.series == "a" && m.label.is_empty())
        .unwrap();
    assert_eq!(leader.stroke.unwrap().width, 3.);
    assert_eq!(leader.stroke.unwrap().color, c.theme.color(0));
    let p = leader.geometry.commands();
    let C::Move(first) = p[0] else { panic!() };
    close((first.x - 200.).hypot(first.y - 200.), 180.);
    let C::Line(last) = p[2] else { panic!() };
    let mut interaction = interaction::Interaction::default();
    assert!(interaction.pointer(&scene, last.x, last.y).is_none());
    assert!(
        scene
            .hit_test(200. + 170. / 2_f64.sqrt(), 200. - 170. / 2_f64.sqrt())
            .is_some()
    );
    let hidden = scene.without_series(&BTreeSet::from(["a".into()]));
    hidden.validate().unwrap();
    assert!(!hidden.marks.iter().any(|m| m.series == "a"));
    assert!(
        !hidden
            .labels
            .iter()
            .any(|l| l.text == "25.0%" || l.text == "1.00")
    );
    o.style.arc_labels.skip_radius = 1000.;
    o.style.link_labels.skip_angle = TAU;
    assert!(pie(&c, &data(), &o).unwrap().labels.is_empty());
}
#[test]
fn border_fill_alpha_identity_collisions_and_bad_options_are_checked_atomically() {
    let c = config();
    let d = vec![
        RadialDatum::new("a", 1.),
        RadialDatum::new("pie-leader:1:a", 3.),
    ];
    let original = d.clone();
    let mut o = PieOptions::default();
    o.style.fill_opacity = 0.25;
    o.style.border = Some(GraphicStroke {
        color: Color(240, 20, 30, 255),
        width: 4.,
        cap: StrokeCap::Round,
    });
    o.style.link_labels.enabled = true;
    let scene = pie(&c, &d, &o).unwrap();
    scene.validate().unwrap();
    assert_eq!(
        scene
            .marks
            .iter()
            .find(|m| m.id == "a")
            .unwrap()
            .fill
            .unwrap()
            .3,
        64
    );
    assert_eq!(
        scene
            .marks
            .iter()
            .find(|m| m.id == "a")
            .unwrap()
            .stroke
            .unwrap()
            .color
            .3,
        255
    );
    assert_eq!(
        scene
            .marks
            .iter()
            .map(|m| &m.id)
            .collect::<BTreeSet<_>>()
            .len(),
        4
    );
    for case in 0..9 {
        let mut bad = o.clone();
        match case {
            0 => bad.style.corner_radius = f64::NAN,
            1 => bad.style.fill_opacity = 1.1,
            2 => bad.style.arc_labels.radius_offset = 2.,
            3 => bad.style.arc_labels.text = PieLabelText::Value(ValueFormat::Fixed(21)),
            4 => bad.style.link_labels.thickness = -1.,
            5 => bad
                .style
                .active_ids
                .insert("unknown".into())
                .then_some(())
                .unwrap(),
            6 => {
                bad.style.active_ids.insert("a".into());
                bad.style.active_outer_radius_offset = -1000.;
            }
            7 => bad.style.link_labels.offset = -1000.,
            _ => bad.style.arc_labels.style.rotation = f64::INFINITY,
        };
        assert!(pie(&c, &d, &bad).is_err(), "case {case}");
    }
    assert_eq!(d, original);
    let too_many = (0..257)
        .map(|i| RadialDatum::new(i.to_string(), 1.))
        .collect::<Vec<_>>();
    assert!(pie(&c, &too_many, &o).is_err());
    let huge = pie(
        &c,
        &[
            RadialDatum::new("x", f64::MAX),
            RadialDatum::new("y", f64::MAX),
        ],
        &o,
    )
    .unwrap();
    huge.validate().unwrap();
}
#[cfg(feature = "raster")]
#[test]
fn cpu_pixels_show_true_rounded_corner_hole_border_and_separate_fill_opacity() {
    use kartis::render::raster::{FontRegistry, RasterOptions, render_raster};
    let c = config();
    let mut o = PieOptions {
        inner_radius: 0.5,
        labels: false,
        ..Default::default()
    };
    let render = |o: &PieOptions| {
        render_raster(
            &pie(&c, &data(), o).unwrap(),
            &FontRegistry::new(),
            &RasterOptions::default(),
        )
        .unwrap()
    };
    let sharp = render(&o);
    o.style.corner_radius = 16.;
    let rounded = render(&o);
    let pixel = |image: &kartis::render::raster::RenderedImage, x: usize, y: usize| -> Vec<u8> {
        image.rgba[(y * 400 + x) * 4..(y * 400 + x) * 4 + 4].to_vec()
    };
    assert_ne!(pixel(&sharp, 201, 41), vec![255; 4]);
    assert_eq!(pixel(&rounded, 201, 41), vec![255; 4]);
    assert_eq!(pixel(&rounded, 200, 200), vec![255; 4]);
    o.style.fill_opacity = 0.25;
    o.style.border = Some(GraphicStroke {
        color: Color(255, 0, 0, 255),
        width: 4.,
        cap: StrokeCap::Round,
    });
    let image = render(&o);
    let center = pixel(&image, 290, 110);
    assert!(center[0] > 150 && center[1] > 150 && center[2] > 150);
    let ring = pixel(&image, 313, 87);
    assert_eq!(ring, vec![255, 0, 0, 255]);
}
