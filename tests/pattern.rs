use std::f64::consts::PI;

use kartis::*;
fn definitions() -> Vec<PatternFill> {
    vec![
        PatternShape::Dots {
            diameter: 4.,
            stagger: false,
        },
        PatternShape::Dots {
            diameter: 4.,
            stagger: true,
        },
        PatternShape::Squares {
            size: 4.,
            stagger: false,
        },
        PatternShape::Squares {
            size: 4.,
            stagger: true,
        },
        PatternShape::Lines { width: 3. },
    ]
    .into_iter()
    .map(|shape| PatternFill {
        shape,
        ..Default::default()
    })
    .collect()
}
fn tile_contains(tile: &PatternTile, p: [f64; 2]) -> bool {
    let p = [p[0].rem_euclid(tile.size[0]), p[1].rem_euclid(tile.size[1])];
    tile.shapes.iter().any(|g| match g {
        Geometry::Circle { x, y, radius } => (p[0] - x).hypot(p[1] - y) < *radius,
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => p[0] >= *x && p[0] < x + width && p[1] >= *y && p[1] < y + height,
        _ => panic!("unexpected pattern primitive"),
    })
}
fn expected(pattern: &PatternFill, p: [f64; 2]) -> bool {
    let s = pattern.spacing;
    if let PatternShape::Lines { width } = pattern.shape {
        let y = p[1].rem_euclid(s);
        return y.min(s - y) < width / 2.;
    }
    let size = match pattern.shape {
        PatternShape::Dots { diameter, .. } => diameter,
        PatternShape::Squares { size, .. } => size,
        _ => unreachable!(),
    };
    let stagger = matches!(
        pattern.shape,
        PatternShape::Dots { stagger: true, .. } | PatternShape::Squares { stagger: true, .. }
    );
    for ix in -8_i32..=8 {
        for iy in -8_i32..=8 {
            if stagger && ix.rem_euclid(2) != iy.rem_euclid(2) {
                continue;
            }
            let dx = (p[0] - (f64::from(ix) + 0.5) * s).abs();
            let dy = (p[1] - (f64::from(iy) + 0.5) * s).abs();
            if matches!(pattern.shape, PatternShape::Dots { .. }) && dx.hypot(dy) < size / 2.
                || matches!(pattern.shape, PatternShape::Squares { .. })
                    && dx < size / 2.
                    && dy < size / 2.
            {
                return true;
            }
        }
    }
    false
}
#[test]
fn tiles_repeat_exact_motifs_including_negative_cells_and_both_stripe_halves() {
    for pattern in definitions() {
        let tile = pattern.tile().unwrap();
        for ix in -80..=80 {
            for iy in -70..=70 {
                let p = [f64::from(ix) * 0.37 + 0.013, f64::from(iy) * 0.41 + 0.017];
                assert_eq!(
                    tile_contains(&tile, p),
                    expected(&pattern, p),
                    "{:?}: {p:?}",
                    pattern.shape
                );
            }
        }
        let foreground_area = tile
            .shapes
            .iter()
            .map(|g| match g {
                Geometry::Circle { radius, .. } => PI * radius * radius,
                Geometry::Rect {
                    x,
                    y,
                    width,
                    height,
                } => {
                    ((x + width).min(tile.size[0]) - x.max(0.)).max(0.)
                        * ((y + height).min(tile.size[1]) - y.max(0.)).max(0.)
                }
                _ => unreachable!(),
            })
            .sum::<f64>();
        let expected = match pattern.shape {
            PatternShape::Dots { diameter, stagger } => {
                PI * (diameter / 2.).powi(2) * if stagger { 2. } else { 1. }
            }
            PatternShape::Squares { size, stagger } => size * size * if stagger { 2. } else { 1. },
            PatternShape::Lines { width } => pattern.spacing * width,
        };
        assert!((foreground_area - expected).abs() < 1e-10);
    }
}
#[test]
fn generated_svg_and_script_use_one_background_and_a_bounded_rotated_lattice() {
    for mut pattern in definitions() {
        for angle in [0., 0.000001, 45., 89.999999, 90., 180., -270.] {
            pattern.rotation = angle;
            pattern.offset = [7.25, -3.5];
            let svg = pattern.svg_definition().unwrap();
            assert!(svg.starts_with("<pattern id=\"kartis-pattern-"));
            assert!(
                svg.contains("patternUnits=\"userSpaceOnUse\"")
                    && svg.contains("overflow=\"hidden\"")
            );
            assert!(svg.ends_with("</pattern>"));
            assert_eq!(svg.matches("<pattern ").count(), 1);
            let script = pattern.canvas_script(1.25).unwrap();
            assert!(
                script.contains("createPattern(tile,'repeat')")
                    && script.contains("new DOMMatrix([")
            );
            assert!(!svg.contains("NaN") && !svg.contains("Infinity"));
            assert!(!script.contains("NaN") && !script.contains("Infinity"));
            let tile = pattern.tile().unwrap();
            assert!(tile.size.iter().all(|n| *n <= 16.));
        }
    }
}
#[test]
fn fractional_backing_dimensions_preserve_pitch_rotation_offset_and_signed_zero_identity() {
    let mut pattern = PatternFill {
        shape: PatternShape::Lines { width: 1. },
        spacing: 5.3,
        rotation: 90.,
        offset: [7., -3.],
        ..Default::default()
    };
    let pixels = pattern.tile_pixels(1.25).unwrap();
    assert_eq!(pixels, [7, 7]);
    let [a, b, c, d, e, f] = pattern.pixel_transform(pixels).unwrap();
    assert_eq!(a, 0.);
    assert_eq!(d, 0.);
    assert!((b * 7. - 5.3).abs() < 1e-12 && (c * 7. + 5.3).abs() < 1e-12);
    assert_eq!([e, f], [7., -3.]);
    pattern.rotation = 0.;
    pattern.offset = [0., 0.];
    let id = pattern.deterministic_id();
    pattern.rotation = -0.;
    pattern.offset = [-0., 0.];
    assert_eq!(id, pattern.deterministic_id());
    pattern.foreground.3 = 128;
    assert_ne!(id, pattern.deterministic_id());
    let shape = PatternShape::Squares {
        size: 0.,
        stagger: false,
    };
    pattern.shape = shape;
    let id = pattern.deterministic_id();
    pattern.shape = PatternShape::Squares {
        size: -0.,
        stagger: false,
    };
    assert_eq!(id, pattern.deterministic_id());
}
#[test]
fn malformed_and_expensive_tiles_fail_before_backend_allocation() {
    let mut pattern = PatternFill::default();
    for spacing in [0., -1., f64::NAN, f64::INFINITY, 1025.] {
        pattern.spacing = spacing;
        assert!(pattern.validate().is_err());
    }
    pattern = PatternFill::default();
    for size in [-1., 9., f64::NAN] {
        pattern.shape = PatternShape::Dots {
            diameter: size,
            stagger: false,
        };
        assert!(pattern.validate().is_err());
    }
    pattern = PatternFill::default();
    for density in [0., -1., f64::NAN, f64::INFINITY, 801.] {
        assert!(pattern.tile_pixels(density).is_err());
        assert!(pattern.canvas_script(density).is_err());
    }
    let large = PatternFill {
        spacing: 1024.,
        shape: PatternShape::Dots {
            diameter: 4.,
            stagger: true,
        },
        ..Default::default()
    };
    assert_eq!(large.tile_pixels(1.).unwrap(), [2048, 2048]);
    assert!(large.tile_pixels(2.).is_err());
    assert!(large.tile_operation_cost(2.).is_err());
    assert!(pattern.pixel_transform([0, 8]).is_err());
    assert!(pattern.pixel_transform([4096, 4096]).is_err());
    pattern.rotation = f64::NAN;
    assert!(pattern.svg_definition().is_err());
    let zero = PatternFill {
        shape: PatternShape::Lines { width: 0. },
        ..Default::default()
    };
    assert!(zero.tile().unwrap().shapes.is_empty());
}
#[cfg(feature = "raster")]
mod raster {
    use tiny_skia::{
        FillRule, FilterQuality, Paint, PathBuilder, Pattern, Pixmap, Rect, SpreadMode, Transform,
    };

    use super::*;
    #[test]
    fn full_width_translucent_stripes_have_no_odd_pixel_tile_seam() {
        let pattern = PatternFill {
            shape: PatternShape::Lines { width: 5. },
            spacing: 5.,
            foreground: Color(200, 0, 0, 128),
            background: Color(0, 0, 200, 255),
            ..Default::default()
        };
        let tile = pattern.raster_tile(1.).unwrap();
        assert_eq!(tile.pixmap.width(), 5);
        let pixels = tile.pixmap.data().as_chunks::<4>().0;
        assert!(
            pixels.iter().all(|p| p == &pixels[0]),
            "split alpha stripes must not leave a seam"
        );
        assert_eq!(pixels[0], [100, 0, 100, 255]);
    }
    #[test]
    fn repeating_stripe_edges_and_alpha_remain_continuous_after_rotation_and_view_dpr() {
        let pattern = PatternFill {
            shape: PatternShape::Lines { width: 4. },
            spacing: 8.,
            rotation: 37.,
            offset: [2.25, -1.5],
            foreground: Color(200, 0, 0, 128),
            background: Color(0, 0, 200, 255),
        };
        let factor = 2.5;
        let translate = [3., 5.];
        let tile = pattern.raster_tile(factor).unwrap();
        let [a, b, c, d, e, f] = tile.transform;
        let transform = Transform::from_row(
            (a * factor) as f32,
            (b * factor) as f32,
            (c * factor) as f32,
            (d * factor) as f32,
            (e * factor + translate[0]) as f32,
            (f * factor + translate[1]) as f32,
        );
        let shader = Pattern::new(
            tile.pixmap.as_ref(),
            SpreadMode::Repeat,
            FilterQuality::Bilinear,
            1.,
            transform,
        );
        let paint = Paint {
            shader,
            ..Default::default()
        };
        let mut output = Pixmap::new(100, 100).unwrap();
        output.fill_path(
            &PathBuilder::from_rect(Rect::from_xywh(0., 0., 100., 100.).unwrap()),
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
        let (sin, cos) = 37_f64.to_radians().sin_cos();
        let mut sampled = 0;
        for py in 0..100 {
            for px in 0..100 {
                let x = (f64::from(px) + 0.5 - translate[0]) / factor - pattern.offset[0];
                let y = (f64::from(py) + 0.5 - translate[1]) / factor - pattern.offset[1];
                let row = (-sin * x + cos * y).rem_euclid(pattern.spacing);
                let edge = row.min(pattern.spacing - row);
                if (edge - 2.).abs() < 0.75 {
                    continue;
                }
                let pixel = output.pixel(px, py).unwrap();
                let expected = if edge < 2. {
                    [100_u8, 0, 100, 255]
                } else {
                    [0, 0, 200, 255]
                };
                assert!(
                    pixel.red().abs_diff(expected[0]) <= 1
                        && pixel.blue().abs_diff(expected[2]) <= 1
                        && pixel.alpha() == 255,
                    "{px},{py}, row={row}: {pixel:?}"
                );
                sampled += 1;
            }
        }
        assert!(sampled > 4000);
    }
    #[test]
    fn staggered_shape_tiles_have_the_expected_alpha_area_and_reject_before_large_allocation() {
        for mut pattern in definitions() {
            pattern.foreground = Color(20, 100, 200, 255);
            pattern.background = Color(0, 0, 0, 0);
            let tile = pattern.raster_tile(4.).unwrap();
            let actual = tile
                .pixmap
                .data()
                .as_chunks::<4>()
                .0
                .iter()
                .map(|p| f64::from(p[3]) / 255.)
                .sum::<f64>()
                / 16.;
            let expected = match pattern.shape {
                PatternShape::Dots { diameter, stagger } => {
                    PI * (diameter / 2.).powi(2) * if stagger { 2. } else { 1. }
                }
                PatternShape::Squares { size, stagger } => {
                    size * size * if stagger { 2. } else { 1. }
                }
                PatternShape::Lines { width } => pattern.spacing * width,
            };
            assert!(
                (actual - expected).abs() < 0.5,
                "{:?}: {actual} vs {expected}",
                pattern.shape
            );
        }
        let huge = PatternFill {
            spacing: 1024.,
            ..Default::default()
        };
        assert!(huge.raster_tile(8.).is_err());
    }
}
