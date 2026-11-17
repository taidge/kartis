use glory::render::PathCommand;
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 420.5,
        height: 300.5,
        margin: Margin {
            left: 60.,
            right: 20.,
            top: 20.,
            bottom: 40.,
        },
        ..Default::default()
    }
}
fn profile(domain: &[f64], range: &[f64]) -> PiecewiseScaleOptions {
    PiecewiseScaleOptions {
        domain: domain.into(),
        range: range.into(),
        ..Default::default()
    }
}
fn axes() -> CartesianAxes {
    let mut a = CartesianAxes::default();
    a.x.visible = false;
    a.x.grid = false;
    a.y.visible = false;
    a.y.grid = false;
    a
}
fn line(p: PiecewiseScaleOptions, points: Vec<Point>) -> PiecewiseLineLayout {
    line_layout_with_piecewise_scale_options(
        &config(),
        &[Series::new("source", points)],
        &LineOptions {
            y_domain: Some([0., 10.]),
            axes: axes(),
            ..Default::default()
        },
        &LineStyleOptions::default(),
        &PiecewiseLineScaleOptions {
            x: Some(p),
            ..Default::default()
        },
    )
    .unwrap()
}
fn commands(layout: &PiecewiseLineLayout) -> Vec<&PathCommand> {
    layout
        .layout
        .scene
        .marks
        .iter()
        .filter(|m| m.id.starts_with("line-stroke:"))
        .flat_map(|m| match &m.geometry {
            Geometry::Path(path) => path.iter().collect::<Vec<_>>(),
            _ => panic!(),
        })
        .collect()
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}

#[test]
fn line_keeps_outside_raw_values_when_repeated_edge_maps_them_into_the_plot() {
    let layout = line(
        profile(&[0., 0., 1.], &[0., 0.4, 1.]),
        [-1., 0., 0.5, 1., 2.]
            .into_iter()
            .enumerate()
            .map(|(i, x)| Point::new(x, i as f64 + 1.))
            .collect(),
    );
    assert_eq!(
        layout
            .layout
            .sites
            .iter()
            .map(|s| s.source_index)
            .collect::<Vec<_>>(),
        [0, 1, 2, 3]
    );
    near(
        layout.layout.sites[0].position[0],
        config().margin.left + config().inner_width() * 0.2,
    );
    for site in &layout.layout.sites {
        near(site.position[0], layout.x_scale.map(site.raw_x));
        near(site.position[1], layout.y_scale.map(site.raw_y));
    }
    layout
        .interaction_binding()
        .validate(&layout.layout.scene)
        .unwrap();
}

#[test]
fn fully_constant_piecewise_domain_preserves_projected_linear_run_instead_of_virtual_midpoint() {
    let layout = line(
        profile(&[5., 5., 5.], &[0., 0.5, 1.]),
        vec![Point::new(4., 1.), Point::new(5., 2.), Point::new(6., 3.)],
    );
    let commands = commands(&layout);
    assert_eq!(
        commands
            .iter()
            .filter(|c| matches!(c, PathCommand::Move(_)))
            .count(),
        1
    );
    let pixels = commands
        .iter()
        .map(|c| match c {
            PathCommand::Move(p) | PathCommand::Line(p) => [p.x, p.y],
            _ => panic!(),
        })
        .collect::<Vec<_>>();
    assert_eq!(pixels.len(), 3);
    for (pixel, site) in pixels.iter().zip(&layout.layout.sites) {
        near(pixel[0], site.position[0]);
        near(pixel[1], site.position[1]);
    }
    assert!(layout.viewport_controller(Default::default()).is_err());
}

#[test]
fn repeated_constant_area_baseline_uses_actual_scale_before_plot_clipping() {
    let data = [Series::new(
        "source",
        vec![Point::new(0., 5.), Point::new(1., 5.)],
    )];
    let layout = line_layout_with_piecewise_scale_options(
        &config(),
        &data,
        &LineOptions {
            area: true,
            baseline: 0.,
            axes: axes(),
            ..Default::default()
        },
        &LineStyleOptions::default(),
        &PiecewiseLineScaleOptions {
            y: Some(profile(&[5., 5., 5.], &[0., 0.5, 1.])),
            ..Default::default()
        },
    )
    .unwrap();
    let y = layout.y_scale.map(0.);
    assert!((y - layout.y_scale.map(5.)).abs() > 10.);
    let area = layout
        .layout
        .scene
        .marks
        .iter()
        .find(|m| m.id.starts_with("line-area:"))
        .unwrap();
    let Geometry::Path(path) = &area.geometry else {
        panic!()
    };
    assert!(
        path.iter()
            .any(|c| matches!(c,PathCommand::Line(p) if (p.y-y).abs()<1e-8)),
        "actual baseline absent: {path:?}"
    );
}

#[test]
fn unbounded_viewport_inverse_preserves_authored_scale_clamp() {
    // Original d3-zoom rescaleX for scaleLinear([0,0,10],[0,50,100])
    // translated by -25, and [0,10,10] translated by +25.
    for (domain, translation, unclamped, clamped) in [
        ([0., 0., 10.], -0.25, [0., 5., 15.], [0., 5., 10.]),
        ([0., 10., 10.], 0.25, [-5., 5., 10.], [0., 5., 10.]),
    ] {
        for clamp in [false, true] {
            let mut p = profile(&domain, &[0., 0.5, 1.]);
            p.continuous.clamp = clamp;
            let layout = line(p, vec![]);
            let mut controller = layout
                .viewport_controller(ViewportLimits {
                    bounded: false,
                    ..Default::default()
                })
                .unwrap();
            let original = controller.clone();
            controller
                .apply(ViewportRequest::TransformXY {
                    transform: ViewportTransform {
                        scale_x: 1.,
                        scale_y: 1.,
                        translate_x: translation * controller.plot().width,
                        translate_y: 0.,
                    },
                    axes: ViewportSelection::X,
                })
                .unwrap();
            let stops = controller.piecewise_domains().unwrap().x.unwrap();
            for (actual, expected) in stops.iter().zip(if clamp { clamped } else { unclamped }) {
                near(*actual, expected);
            }
            controller.apply(ViewportRequest::Reset).unwrap();
            assert_eq!(controller, original);
        }
    }
}

type ProbeFrame = (SceneInteraction, Vec<(String, [f64; 2])>);
fn constant_frames() -> Vec<ProbeFrame> {
    let p = profile(&[5., 5., 5.], &[0., 0.5, 1.]);
    let l = line(
        p.clone(),
        vec![Point::new(4., 2.), Point::new(5., 5.), Point::new(6., 8.)],
    );
    let s = scatter_layout(
        &config(),
        &[ScatterSeries::new(
            "source",
            vec![
                ScatterDatum::new("a", 4., 2.),
                ScatterDatum::new("b", 5., 5.),
                ScatterDatum::new("c", 6., 8.),
            ],
        )],
        &ScatterLayoutOptions {
            scales: ScatterScaleOptions {
                x: ScatterAxisScale::Piecewise { options: p },
                y: ScatterAxisScale::Continuous {
                    options: Default::default(),
                    min: ScatterBound::Fixed(0.),
                    max: ScatterBound::Fixed(10.),
                },
            },
            axes: ScatterAxes {
                top: None,
                bottom: None,
                left: None,
                right: None,
            },
            ..Default::default()
        },
    )
    .unwrap();
    // Original scaleLinear([5,5,5],[60,230.25,400.5]) and
    // scaleLinear([0,10],[260.5,20]) at source (4,2), (5,5), (6,8).
    let expected = [[145.125, 212.4], [315.375, 140.25], [315.375, 68.1]];
    let mut frames = vec![];
    let line_probes = l
        .layout
        .sites
        .iter()
        .enumerate()
        .map(|(i, site)| {
            near(site.position[0], expected[i][0]);
            near(site.position[1], expected[i][1]);
            (site.mark_id.clone(), expected[i])
        })
        .collect();
    frames.push((
        SceneInteraction {
            scene: l.layout.scene.clone(),
            plot: Some(l.layout.plot),
            interaction: Some(l.interaction_binding()),
        },
        line_probes,
    ));
    let scatter_probes = s
        .sites
        .iter()
        .enumerate()
        .map(|(i, site)| {
            near(site.position[0], expected[i][0]);
            near(site.position[1], expected[i][1]);
            (site.mark_id.clone(), expected[i])
        })
        .collect();
    frames.push((
        SceneInteraction {
            scene: s.scene.clone(),
            plot: Some(s.plot),
            interaction: Some(s.interaction_binding()),
        },
        scatter_probes,
    ));
    for (frame, _) in &mut frames {
        frame.interaction.as_mut().unwrap().picking_profile = PickingProfile::PaintedGeometry;
    }
    frames
}

fn nodes(app: &glory::App, class: &str) -> Vec<glory::render::NodeId> {
    let tree = app.tree();
    let mut pending = tree.roots().to_vec();
    let mut out = vec![];
    while let Some(id) = pending.pop() {
        let node = tree.node(id).unwrap();
        if node.descriptor.view().classes.iter().any(|s| s == class) {
            out.push(id);
        }
        pending.extend(node.children.iter().copied());
    }
    out
}

#[test]
fn constant_domains_reach_real_glory_plot_events_controls_and_atomic_rejection() {
    use std::cell::RefCell;
    use std::rc::Rc;

    use glory::render::{Event, PlotPointer, PlotPointerDevice, PlotPointerPhase, Rect};
    for (frame, probes) in constant_frames() {
        frame.validate().unwrap();
        assert!(
            frame.scene.marks.iter().all(|m| m.clip_path.is_none()),
            "native rectangular marks required"
        );
        assert!(
            frame.scene.labels.iter().all(|label| label.rotation == 0.),
            "native unrotated text required"
        );
        let app = glory::App::memory();
        let input = app.scope().cage(frame.clone());
        let hits = Rc::new(RefCell::new(vec![]));
        let observed = hits.clone();
        let failures = Rc::new(RefCell::new(vec![]));
        let rejected = failures.clone();
        app.mount(
            chart_layout(input)
                .highlight(false)
                .on_physical_hover(move |hit| observed.borrow_mut().push(hit))
                .on_reject(move |error| rejected.borrow_mut().push(error)),
        )
        .unwrap_or_else(|error| panic!("mount {error:?}: {:?}", failures.borrow()));
        assert_eq!(nodes(&app, "kartis-datum").len(), 3);
        let plot = nodes(&app, "kartis-plot")[0];
        for (id, p) in &probes {
            app.dispatch(
                plot,
                Event::PlotPointer {
                    from: plot,
                    pointer: PlotPointer {
                        pointer_id: 19,
                        device: PlotPointerDevice::Pen,
                        phase: PlotPointerPhase::Move,
                        x: p[0] * 2.,
                        y: p[1] * 2.,
                        viewport: Rect {
                            width: config().width * 2.,
                            height: config().height * 2.,
                            ..Default::default()
                        },
                    },
                },
            )
            .unwrap();
            app.flush().unwrap();
            let hit = hits.borrow().last().cloned().flatten().unwrap();
            assert_eq!(&hit.logical_site_id, id);
        }
        let before = format!("{:?}", app.tree());
        let mut invalid = frame.clone();
        invalid.interaction.as_mut().unwrap().sites[0].position[0] = f64::NAN;
        input.revise(|value| *value = invalid);
        app.flush().unwrap();
        assert_eq!(failures.borrow().len(), 1);
        assert_eq!(format!("{:?}", app.tree()), before);
    }
}

#[cfg(feature = "raster")]
#[test]
fn constant_domains_paint_native_cpu_ink_at_d3_pixels_at_both_dprs() {
    use render::raster::*;
    for (frame, probes) in constant_frames() {
        for dpr in [1., 2.] {
            let image = render_raster(
                &frame.scene,
                &FontRegistry::new(),
                &RasterOptions {
                    pixel_ratio: dpr,
                    ..Default::default()
                },
            )
            .unwrap();
            for (_, p) in &probes {
                let x = (p[0] * dpr).floor() as usize;
                let y = (p[1] * dpr).floor() as usize;
                let offset = (y * image.width as usize + x) * 4;
                let Color(r, g, b, a) = config().theme.color(0);
                assert_eq!(
                    &image.rgba[offset..offset + 4],
                    [r, g, b, a],
                    "actual CPU pixel at {p:?}, DPR{dpr}"
                );
            }
        }
    }
}
