use kartis::render::*;
use kartis::*;
fn scene() -> Scene {
    line(
        &ChartConfig::default(),
        &[Series::new(
            "values",
            vec![Point::new(0., 1.), Point::new(1., 2.)],
        )],
        &LineOptions::default(),
    )
    .unwrap()
}

#[test]
fn extra_buffer_admission_has_checked_lower_upper_and_overflow_edges() {
    let scene = scene();
    for budget in [0, 134_217_729, usize::MAX] {
        let opts = CanvasScriptOptions {
            surface: CanvasScriptSurface::Offscreen2d {
                fallback: CanvasScriptFallback::Direct,
            },
            max_extra_buffer_pixels: budget,
        };
        assert!(to_canvas_script_with_options(&scene, "chart", &opts).is_err());
    }
    for budget in [1, 134_217_728] {
        let opts = CanvasScriptOptions {
            surface: CanvasScriptSurface::Offscreen2d {
                fallback: CanvasScriptFallback::Error,
            },
            max_extra_buffer_pixels: budget,
        };
        assert!(to_canvas_script_with_options(&scene, "chart", &opts).is_ok());
    }
}

#[test]
fn surface_selection_retains_scene_id_and_blend_validation() {
    let mut scene = scene();
    let opts = CanvasScriptOptions {
        surface: CanvasScriptSurface::Offscreen2d {
            fallback: CanvasScriptFallback::Error,
        },
        ..Default::default()
    };
    assert!(to_canvas_script_with_options(&scene, "", &opts).is_err());
    scene.marks[0].value = f64::NAN;
    assert!(to_canvas_script_with_options(&scene, "chart", &opts).is_err());
    assert!(to_canvas_script(&scene, "chart").is_err());
}

#[test]
fn explicit_direct_options_preserve_default_program_bytes() {
    let scene = scene();
    let plain = to_canvas_script(&scene, "chart").unwrap();
    let explicit = to_canvas_script_with_options(
        &scene,
        "chart",
        &CanvasScriptOptions {
            surface: CanvasScriptSurface::Direct,
            max_extra_buffer_pixels: 1,
        },
    )
    .unwrap();
    assert_eq!(plain, explicit);
    let blends =
        MarkBlends::new(&scene, [(scene.marks[0].id.clone(), BlendMode::Multiply)]).unwrap();
    assert_eq!(
        to_canvas_script_with_blends(&scene, "chart", &blends).unwrap(),
        to_canvas_script_with_blends_and_options(
            &scene,
            "chart",
            &blends,
            &CanvasScriptOptions::default()
        )
        .unwrap()
    );
}
