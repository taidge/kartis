use kartis::*;
#[cfg(feature = "raster")]
use std::collections::BTreeMap;
use std::{
    collections::BTreeSet,
    f64::consts::{PI, TAU},
};
fn config() -> ChartConfig {
    ChartConfig {
        width: 420.,
        height: 420.,
        margin: Margin {
            left: 40.,
            right: 40.,
            top: 40.,
            bottom: 40.,
        },
        ..Default::default()
    }
}
fn categories() -> Vec<String> {
    vec!["work".into(), "rest:part".into(), "zero".into()]
}
fn rows() -> Vec<BarSeries> {
    vec![
        BarSeries::new("ring:a", vec![Some(2.), Some(6.), Some(0.)]),
        BarSeries::new("ring.b", vec![Some(3.), Some(9.), None]),
    ]
}
fn options() -> StackedRadialBarOptions {
    StackedRadialBarOptions {
        max_value: Some(12.),
        ..Default::default()
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}
#[test]
fn each_series_is_a_radial_band_and_values_conserve_directed_angular_stack() {
    let c = config();
    for (start, end) in [(0., TAU * 0.75), (5.5, 7.5), (5.5, 1.2)] {
        let mut o = options();
        o.start_angle = start;
        o.end_angle = end;
        let l = layout_stacked_radial_bar(&c, &categories(), &rows(), &o).unwrap();
        assert_eq!(l.domain, [0., 12.]);
        let step = (l.frame.outer_radius - l.frame.inner_radius) / (2. + o.padding);
        close(
            l.rings[0].inner_radius,
            l.frame.inner_radius + step * o.padding,
        );
        close(
            l.rings[0].outer_radius - l.rings[0].inner_radius,
            step * (1. - o.padding),
        );
        assert!(l.rings[0].outer_radius < l.rings[1].inner_radius);
        for ring in &l.rings {
            let seg = l
                .segments
                .iter()
                .filter(|s| s.ring_id == ring.id)
                .collect::<Vec<_>>();
            let span: f64 = seg.iter().map(|s| s.angles[1] - s.angles[0]).sum();
            close(span, (end - start) * ring.total / 12.);
            close(seg[0].angles[0], start);
            for w in seg.windows(2) {
                close(w[0].angles[1], w[1].angles[0]);
            }
            let area: f64 = seg
                .iter()
                .map(|s| {
                    0.5 * (s.outer_radius.powi(2) - s.inner_radius.powi(2))
                        * (s.angles[1] - s.angles[0]).abs()
                })
                .sum();
            close(
                area,
                0.5 * (ring.outer_radius.powi(2) - ring.inner_radius.powi(2))
                    * (end - start).abs()
                    * ring.total
                    / 12.,
            );
        }
    }
}
#[test]
fn hidden_categories_keep_domain_tracks_geometry_and_owned_labels_stable() {
    let mut o = options();
    o.labels = Some(RadialBarLabels {
        text: RadialBarLabelText::Percent(ValueFormat::Percent(0)),
        ..Default::default()
    });
    let full = layout_stacked_radial_bar(&config(), &categories(), &rows(), &o).unwrap();
    o.hidden.insert("work".into());
    let hidden = layout_stacked_radial_bar(&config(), &categories(), &rows(), &o).unwrap();
    assert_eq!(full.domain, hidden.domain);
    assert_eq!(full.rings, hidden.rings);
    assert_eq!(hidden.scene, full.scene.without_series(&o.hidden));
    assert_eq!(hidden.scene.legend.len(), 3);
    assert_eq!(hidden.scene.label_series.len(), 2);
    assert!(!hidden.scene.labels.iter().any(|l| l.text == "25%"));
    assert!(
        hidden
            .segments
            .iter()
            .filter(|s| s.category == "work")
            .all(|s| !s.visible)
    );
}
#[test]
fn composite_raw_ids_palette_and_metadata_survive_input_row_reorder() {
    let a = layout_stacked_radial_bar(&config(), &categories(), &rows(), &options()).unwrap();
    let mut reversed = rows();
    reversed.reverse();
    let b = layout_stacked_radial_bar(&config(), &categories(), &reversed, &options()).unwrap();
    assert_eq!(a.scene.legend, b.scene.legend);
    for m in a.scene.marks.iter().filter(|m| !m.series.is_empty()) {
        let n = b.scene.marks.iter().find(|n| n.id == m.id).unwrap();
        assert_eq!((m.value, &m.series, m.fill), (n.value, &n.series, n.fill));
        assert!(m.label.contains("ring:") || m.label.contains("ring."));
    }
    assert!(a.segments.iter().any(|s| s.ring_id == "ring:a"
        && s.category == "rest:part"
        && s.mark_id.contains("6:ring:a:9:rest:part")));
}
#[test]
fn zero_missing_empty_and_padded_segments_have_explicit_metadata_and_no_fake_ink() {
    let c = config();
    let zero = vec![BarSeries::new("z", vec![Some(0.), None, Some(0.)])];
    let l = layout_stacked_radial_bar(&c, &categories(), &zero, &Default::default()).unwrap();
    assert_eq!(l.domain, [0., 1.]);
    assert_eq!(l.segments.len(), 2);
    assert!(l.segments.iter().all(|s| !s.visible));
    assert!(l.scene.marks.iter().all(|m| m.series.is_empty()));
    assert!(stacked_radial_bar(&c, &[], &[], &Default::default()).is_ok());
    let mut o = options();
    o.pad_angle = PI;
    let l = layout_stacked_radial_bar(&c, &categories(), &rows(), &o).unwrap();
    assert!(
        l.segments
            .iter()
            .filter(|s| s.category == "work")
            .all(|s| !s.visible)
    );
    assert_eq!(
        l.segments
            .iter()
            .find(|s| s.ring_id == "ring:a" && s.category == "work")
            .unwrap()
            .value,
        2.
    );
}
#[test]
fn paint_labels_and_common_axes_use_real_positions_and_do_not_become_data() {
    let c = config();
    let mut o = options();
    o.labels = Some(RadialBarLabels {
        text: RadialBarLabelText::Category,
        radius_offset: 1.,
        rotation: 30.,
        ..Default::default()
    });
    o.paints.insert(
        "work".into(),
        FillPaint::Linear {
            start: [100., 100.],
            end: [300., 300.],
            stops: vec![
                GradientStop::new(0., Color(10, 100, 180, 255)),
                GradientStop::new(1., Color(20, 180, 100, 255)),
            ],
        },
    );
    o.value_format = ValueFormat::Fixed(2);
    let l = layout_stacked_radial_bar(&c, &categories(), &rows(), &o).unwrap();
    let seg = l
        .segments
        .iter()
        .find(|s| s.visible && s.category == "work")
        .unwrap();
    let mark = l.scene.marks.iter().find(|m| m.id == seg.mark_id).unwrap();
    assert!(mark.fill_paint.is_some());
    assert!(mark.label.contains("2.00"));
    let index = *l
        .scene
        .label_series
        .iter()
        .find(|(_, id)| *id == "work")
        .unwrap()
        .0;
    let label = &l.scene.labels[index];
    let p = l
        .frame
        .point(seg.outer_radius, (seg.angles[0] + seg.angles[1]) / 2.);
    close(label.x, p[0]);
    close(label.y, p[1]);
    assert_eq!(label.rotation, 30.);
    assert!(
        l.scene
            .marks
            .iter()
            .filter(|m| m.id.starts_with("polar:"))
            .all(|m| m.label.is_empty() && m.series.is_empty())
    );
    let svg = render::to_svg(&l.scene).unwrap();
    assert!(svg.contains("linearGradient") && svg.contains("rotate(30"));
}
#[test]
fn finite_extremes_unknown_ids_budget_and_invalid_options_are_rejected_without_mutation() {
    let c = config();
    let data = rows();
    let before = data.clone();
    for kind in 0..9 {
        let mut o = options();
        match kind {
            0 => o.padding = 1.,
            1 => o.pad_angle = f64::NAN,
            2 => o.max_value = Some(11.),
            3 => o.hidden.insert("unknown".into()).then_some(()).unwrap(),
            4 => o.opacity = 2.,
            5 => o.axes.grid.width = -1.,
            6 => o.end_angle = o.start_angle,
            7 => o.end_angle = o.start_angle + TAU * 2.,
            _ => {
                o.labels = Some(RadialBarLabels {
                    radius_offset: 2.,
                    ..Default::default()
                })
            }
        };
        assert!(
            stacked_radial_bar(&c, &categories(), &data, &o).is_err(),
            "case {kind}"
        );
    }
    assert_eq!(data, before);
    for bad in [
        vec![Some(-1.); 3],
        vec![Some(f64::NAN); 3],
        vec![Some(f64::MAX); 3],
        vec![Some(1.), Some(f64::MIN_POSITIVE), None],
    ] {
        assert!(
            stacked_radial_bar(
                &c,
                &categories(),
                &[BarSeries::new("bad", bad)],
                &Default::default()
            )
            .is_err()
        );
    }
    let cat = (0..65).map(|i| format!("c{i}")).collect::<Vec<_>>();
    let over = (0..128)
        .map(|i| BarSeries::new(format!("r{i}"), vec![Some(1.); 65]))
        .collect::<Vec<_>>();
    assert!(stacked_radial_bar(&c, &cat, &over, &Default::default()).is_err());
}
#[test]
fn dense_profile_is_admitted_and_logical_mark_ids_are_unique() {
    let cat = (0..64).map(|i| format!("c:{i}")).collect::<Vec<_>>();
    let rows = (0..128)
        .map(|i| BarSeries::new(format!("r:{i}"), vec![Some(1.); 64]))
        .collect::<Vec<_>>();
    let l = layout_stacked_radial_bar(&config(), &cat, &rows, &Default::default()).unwrap();
    assert_eq!(l.segments.len(), MAX_RADIAL_BAR_CELLS);
    assert_eq!(
        l.scene
            .marks
            .iter()
            .map(|m| &m.id)
            .collect::<BTreeSet<_>>()
            .len(),
        l.scene.marks.len()
    );
    assert!(
        l.scene
            .marks
            .iter()
            .map(|m| m.geometry.commands().len())
            .sum::<usize>()
            <= 200000
    );
}
#[cfg(feature = "raster")]
#[test]
fn cpu_ink_matches_true_ring_angle_and_hidden_sector_retains_the_track() {
    use render::raster::*;
    let c = config();
    let mut o = options();
    o.colors = BTreeMap::from([
        ("work".into(), Color(220, 30, 40, 255)),
        ("rest:part".into(), Color(20, 130, 200, 255)),
    ]);
    o.track_color = Some(Color(230, 230, 230, 255));
    o.axes.radial_start = None;
    o.axes.circular_outer = None;
    o.axes.grid.radial = false;
    o.axes.grid.circular = false;
    let l = layout_stacked_radial_bar(&c, &categories(), &rows(), &o).unwrap();
    let s = l
        .segments
        .iter()
        .find(|s| s.visible && s.category == "work")
        .unwrap();
    let p = l.frame.point(
        (s.inner_radius + s.outer_radius) / 2.,
        (s.angles[0] + s.angles[1]) / 2.,
    );
    let render =
        |scene: &Scene| render_raster(scene, &FontRegistry::new(), &Default::default()).unwrap();
    let pixel = |image: &RenderedImage| {
        image.rgba[((p[1] as usize) * 420 + p[0] as usize) * 4
            ..((p[1] as usize) * 420 + p[0] as usize) * 4 + 4]
            .to_vec()
    };
    assert_eq!(pixel(&render(&l.scene)), vec![220, 30, 40, 255]);
    o.hidden.insert("work".into());
    let hidden = stacked_radial_bar(&c, &categories(), &rows(), &o).unwrap();
    assert_eq!(pixel(&render(&hidden)), vec![230, 230, 230, 255]);
}
