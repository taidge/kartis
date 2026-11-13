use kartis::*;

#[test]
fn script_preflight_does_not_treat_a_canvas_id_as_generated_code() {
    let scene = ChartConfig::default().scene().unwrap();
    let id = "plot-canvas.width=x";
    let script = render::to_canvas_script(&scene, id).unwrap();
    assert!(
        script.contains("getElementById(\"plot-canvas.width=x\")"),
        "preflight insertion rewrote a valid caller-owned canvas ID: {script}"
    );
}

fn dashed_scene(cap: glory::render::StrokeCap) -> Scene {
    let mut scene = ChartConfig::default().scene().unwrap();
    let mut mark = Mark::new(
        "dash",
        Geometry::Path(vec![
            glory::render::PathCommand::Move(glory::render::GraphicPoint::new(0., 50.)),
            glory::render::PathCommand::Line(glory::render::GraphicPoint::new(100., 50.)),
        ]),
        Color(0, 0, 0, 255),
    );
    mark.fill = None;
    mark.stroke = Some(glory::render::GraphicStroke {
        color: Color(0, 0, 0, 255),
        width: 20.,
        cap,
    });
    mark.stroke_dash = Some(StrokeDash {
        segments: vec![10., 20.],
        offset: 0.,
    });
    scene.marks.push(mark);
    scene
}

#[test]
fn dash_hit_respects_butt_cap_gaps_beyond_the_existing_two_pixel_tolerance() {
    let scene = dashed_scene(glory::render::StrokeCap::Butt);
    assert!(
        scene.hit_test(17.5, 50.5).is_none(),
        "a seven-pixel butt-cap gap must not become a round-cap hit"
    );
}

#[test]
fn dash_hit_includes_the_actual_square_cap_corner() {
    let scene = dashed_scene(glory::render::StrokeCap::Square);
    assert!(
        scene.hit_test(19.5, 59.5).is_some(),
        "the corner of the 20px square cap is painted but omitted by round distance"
    );
}

#[test]
fn dash_cap_tolerance_and_closed_join_picking_keep_the_actual_footprint() {
    assert!(
        dashed_scene(glory::render::StrokeCap::Round)
            .hit_test(17.5, 50.5)
            .is_some()
    );
    assert!(
        dashed_scene(glory::render::StrokeCap::Butt)
            .hit_test(11.5, 50.)
            .is_some()
    );
    assert!(
        dashed_scene(glory::render::StrokeCap::Butt)
            .hit_test(13., 50.)
            .is_none()
    );
    let mut scene = dashed_scene(glory::render::StrokeCap::Butt);
    scene.marks[0].geometry = Geometry::Path(vec![
        glory::render::PathCommand::Move(glory::render::GraphicPoint::new(20., 100.)),
        glory::render::PathCommand::Line(glory::render::GraphicPoint::new(50., 100.)),
        glory::render::PathCommand::Line(glory::render::GraphicPoint::new(50., 130.)),
    ]);
    scene.marks[0].stroke_dash = Some(StrokeDash {
        segments: vec![1000., 1.],
        offset: 0.,
    });
    assert!(
        scene.hit_test(59.5, 90.5).is_some(),
        "90 degree miter corner must remain hittable"
    );
    assert!(
        scene.hit_test(50., 137.5).is_none(),
        "internal joins must not add round caps to the final butt endpoint"
    );
    scene.marks[0].geometry = Geometry::Rect {
        x: 50.,
        y: 100.,
        width: 30.,
        height: 30.,
    };
    assert!(
        scene.hit_test(40.5, 90.5).is_some(),
        "closed dash retains its seam miter join"
    );
    scene.marks[0].geometry = Geometry::Path(vec![
        glory::render::PathCommand::Move(glory::render::GraphicPoint::new(20., 100.)),
        glory::render::PathCommand::Line(glory::render::GraphicPoint::new(50., 100.)),
        glory::render::PathCommand::Line(glory::render::GraphicPoint::new(21., 103.)),
    ]);
    assert!(
        scene.hit_test(70., 100.).is_none(),
        "an almost reversing miter must use a bounded bevel"
    );
}

#[test]
fn cached_pattern_construction_precedes_resize_of_the_caller_canvas() {
    let mut scene = ChartConfig::default().scene().unwrap();
    let mut mark = Mark::new(
        "pattern",
        Geometry::Rect {
            x: 0.,
            y: 0.,
            width: 100.,
            height: 100.,
        },
        Color(0, 0, 0, 255),
    );
    mark.fill_paint = Some(FillPaint::Pattern(PatternFill::default()));
    scene.marks.push(mark);
    let script = render::to_canvas_script(&scene, "canvas").unwrap();
    assert!(
        script.find("ctx.createPattern").unwrap() < script.find("canvas.width=").unwrap(),
        "host pattern failure must occur before clearing an accepted backing store"
    );
}

#[cfg(feature = "raster")]
#[test]
fn cap_hit_goldens_correspond_to_actual_cpu_ink() {
    use render::raster::*;
    for (cap, x, y, expected) in [
        (glory::render::StrokeCap::Butt, 17, 50, 255),
        (glory::render::StrokeCap::Square, 19, 59, 0),
    ] {
        let scene = dashed_scene(cap);
        let image = render_raster(&scene, &FontRegistry::new(), &RasterOptions::default()).unwrap();
        let pixel = &image.rgba
            [((y * image.width + x) * 4) as usize..((y * image.width + x) * 4 + 4) as usize];
        assert_eq!(pixel, [expected, expected, expected, 255]);
    }
}
