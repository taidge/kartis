#![cfg(feature = "raster")]

use std::collections::BTreeSet;

use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::interaction::Interaction;
use kartis::render::raster::*;
use kartis::*;

const FONT: &[u8] = include_bytes!("fonts/KartisFixtureColrV0.ttf");
fn fonts() -> FontRegistry {
    let mut fonts = FontRegistry::new().with_color_glyphs(ColorGlyphMode::ColrV0 { palette: 0 });
    fonts.register(FONT, 0).unwrap();
    fonts
}
fn options() -> PaintedLabelOptions {
    PaintedLabelOptions {
        rotated_only: false,
        ..Default::default()
    }
}
fn scene() -> Scene {
    let mut scene = ChartConfig {
        width: 240.,
        height: 180.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    scene.labels.push(Label {
        text: "B".into(),
        x: 40.,
        y: 140.,
        size: 100.,
        color: Color(0, 255, 0, 0),
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
    scene
}
fn pixel(image: &RenderedImage, x: usize, y: usize) -> &[u8] {
    let index = (y * image.width as usize + x) * 4;
    &image.rgba[index..index + 4]
}
fn rect_clip() -> ClipPath {
    ClipPath {
        geometry: Geometry::Rect {
            x: 45.,
            y: 45.,
            width: 90.,
            height: 90.,
        },
        rule: ClipRule::NonZero,
    }
}
fn hole_clip() -> ClipPath {
    let mut path = Geometry::Rect {
        x: 40.,
        y: 40.,
        width: 100.,
        height: 100.,
    }
    .commands();
    path.extend(
        Geometry::Rect {
            x: 90.,
            y: 65.,
            width: 20.,
            height: 30.,
        }
        .commands(),
    );
    ClipPath {
        geometry: Geometry::Path(path),
        rule: ClipRule::EvenOdd,
    }
}

#[test]
fn standalone_exports_replace_transparent_foreground_text_with_actual_color_paths() {
    let source = scene();
    let before = source.clone();
    let fonts = fonts();
    let converted = scene_with_painted_labels(&source, &fonts, &options(), "export").unwrap();
    assert_eq!(source, before);
    assert!(!PaintedLabelOptions::default().rotated_only);
    assert_eq!(
        converted,
        scene_with_painted_labels(&source, &fonts, &Default::default(), "export").unwrap()
    );
    assert!(converted.labels.is_empty());
    assert_eq!(converted.marks.len(), 4);
    assert_eq!(
        converted.marks[0].id,
        "kartis-painted-export:label:0:glyph:0:layer:0"
    );
    assert!(
        converted
            .marks
            .iter()
            .all(|mark| mark.label.is_empty() && mark.value == 0.)
    );
    let svg = render::to_svg(&converted).unwrap();
    let canvas = render::to_canvas_script(&converted, "chart").unwrap();
    assert!(!svg.contains("<text") && !canvas.contains("fillText("));
    assert!(svg.contains("255,0,0") && svg.contains("0,0,255"));
    assert!(canvas.contains("255,0,0") && canvas.contains("0,0,255"));
    let original = render_raster(&source, &fonts, &Default::default()).unwrap();
    // Exported geometry requires no fonts to draw, including palette layers
    // whose foreground color was entirely transparent.
    let raster = render_raster(&converted, &FontRegistry::new(), &Default::default()).unwrap();
    assert_eq!(raster.rgba, original.rgba);
    for (x, y, expected) in [
        (50, 50, [255, 0, 0, 255]),
        (70, 80, [255, 255, 255, 255]),
        (100, 80, [127, 127, 255, 255]),
        (125, 80, [127, 0, 128, 255]),
        (50, 125, [255, 0, 0, 255]),
    ] {
        assert_eq!(pixel(&raster, x, y), expected);
    }
    assert_eq!(
        converted,
        scene_with_painted_labels(&source, &fonts, &options(), "export").unwrap()
    );
}

#[test]
fn mixed_order_business_metadata_owner_hiding_and_residual_label_indices_survive() {
    let fonts = fonts();
    let mut source = scene();
    source.labels[0].rotation = 20.;
    let mut plain = source.labels[0].clone();
    plain.text = "A".into();
    plain.rotation = 0.;
    plain.size = 18.;
    plain.color.3 = 255;
    source.labels.push(plain.clone());
    let mut colored = source.labels[0].clone();
    colored.rotation = -15.;
    source.labels.push(colored);
    source.labels.push(plain);
    for (index, owner) in [(0, "alpha"), (1, "beta"), (2, "alpha"), (3, "alpha")] {
        source.label_series.insert(index, owner.into());
    }
    for index in [0, 1, 3] {
        source.label_clips.insert(index, rect_clip());
    }
    source.label_clips.insert(2, hole_clip());
    for (id, series, x, value) in [("one", "alpha", 10., 7.), ("two", "beta", 180., 12.)] {
        let mut mark = Mark::new(
            id,
            Geometry::Rect {
                x,
                y: 10.,
                width: 20.,
                height: 20.,
            },
            Color(30, 60, 90, 255),
        );
        mark.series = series.into();
        mark.value = value;
        source.marks.push(mark);
    }
    source.draw_order = vec![
        DrawItem::Label(1),
        DrawItem::Mark(0),
        DrawItem::Label(0),
        DrawItem::Mark(1),
        DrawItem::Label(3),
        DrawItem::Label(2),
    ];
    let before = source.clone();
    let converted = scene_with_painted_labels(
        &source,
        &fonts,
        &PaintedLabelOptions {
            rotated_only: true,
            ..Default::default()
        },
        "ordered",
    )
    .unwrap();
    assert_eq!(source, before);
    assert_eq!(&converted.marks[..2], &source.marks);
    assert_eq!(
        converted.labels,
        [source.labels[1].clone(), source.labels[3].clone()]
    );
    assert_eq!(converted.label_series[&0], "beta");
    assert_eq!(converted.label_series[&1], "alpha");
    assert_eq!(converted.label_clips[&0], source.label_clips[&1]);
    assert_eq!(converted.label_clips[&1], source.label_clips[&3]);
    for (range, source_label) in [(2..6, 0), (6..10, 2)] {
        for mark in &converted.marks[range] {
            assert_eq!(mark.series, "alpha");
            assert!(mark.label.is_empty());
            assert_eq!(
                mark.clip_path.as_deref(),
                Some(&source.label_clips[&source_label])
            );
        }
    }
    assert_eq!(
        converted.draw_order,
        [
            DrawItem::Label(0),
            DrawItem::Mark(0),
            DrawItem::Mark(2),
            DrawItem::Mark(3),
            DrawItem::Mark(4),
            DrawItem::Mark(5),
            DrawItem::Mark(1),
            DrawItem::Label(1),
            DrawItem::Mark(6),
            DrawItem::Mark(7),
            DrawItem::Mark(8),
            DrawItem::Mark(9)
        ]
    );
    assert_eq!(
        render_raster(&source, &fonts, &Default::default())
            .unwrap()
            .rgba,
        render_raster(&converted, &fonts, &Default::default())
            .unwrap()
            .rgba
    );
    let hidden = BTreeSet::from(["alpha".into()]);
    let visible = converted.without_series(&hidden);
    assert_eq!(visible.marks.len(), 1);
    assert_eq!(visible.marks[0], source.marks[1]);
    assert_eq!(visible.labels, [source.labels[1].clone()]);
    assert_eq!(
        render_raster(&visible, &fonts, &Default::default())
            .unwrap()
            .rgba,
        render_raster(&source.without_series(&hidden), &fonts, &Default::default())
            .unwrap()
            .rgba
    );
    let mut interaction = Interaction::default();
    assert_eq!(interaction.step(&converted, true), Some("one"));
    assert_eq!(interaction.step(&converted, true), Some("two"));
    assert_eq!(interaction.step(&converted, true), Some("one"));
}

#[test]
fn rectangle_and_hole_clips_match_direct_color_rendering_without_text_repainting() {
    for clip in [rect_clip(), hole_clip()] {
        let mut source = scene();
        source.label_clips.insert(0, clip);
        let fonts = fonts();
        let converted = scene_with_painted_labels(&source, &fonts, &options(), "clipped").unwrap();
        let direct = render_raster(&source, &fonts, &Default::default()).unwrap();
        let output = render_raster(&converted, &FontRegistry::new(), &Default::default()).unwrap();
        assert_eq!(output.rgba, direct.rgba);
        let svg = render::to_svg(&converted).unwrap();
        let canvas = render::to_canvas_script(&converted, "chart").unwrap();
        assert!(svg.contains("clip-path=") && canvas.contains("ctx.clip("));
        assert!(!svg.contains("<text") && !canvas.contains("fillText("));
    }
}

#[test]
fn invalid_namespaces_collisions_and_painted_limits_reject_without_changing_source() {
    let fonts = fonts();
    let mut source = scene();
    assert_eq!(
        source,
        scene_with_painted_labels(
            &source,
            &fonts,
            &PaintedLabelOptions {
                rotated_only: true,
                ..Default::default()
            },
            "unused"
        )
        .unwrap()
    );
    for namespace in ["", "0name", "bad/name", "bad:name", "bad name"] {
        assert!(scene_with_painted_labels(&source, &fonts, &options(), namespace).is_err());
    }
    assert!(scene_with_painted_labels(&source, &fonts, &options(), &"n".repeat(129)).is_err());
    assert!(scene_with_painted_labels(&source, &fonts, &options(), "_ns.1-a").is_ok());
    assert!(
        scene_with_painted_labels(
            &source,
            &fonts,
            &PaintedLabelOptions {
                max_layers: 3,
                ..options()
            },
            "n"
        )
        .is_err()
    );
    assert!(
        scene_with_painted_labels(
            &source,
            &fonts,
            &PaintedLabelOptions {
                max_commands: 1,
                ..options()
            },
            "n"
        )
        .is_err()
    );
    source.marks.push(Mark::new(
        "kartis-painted-collision:label:0:glyph:0:layer:0",
        Geometry::Circle {
            x: 0.,
            y: 0.,
            radius: 1.,
        },
        Color(0, 0, 0, 255),
    ));
    let before = source.clone();
    assert!(
        scene_with_painted_labels(&source, &fonts, &options(), "collision")
            .unwrap_err()
            .0
            .contains("collision")
    );
    assert_eq!(source, before);
}

#[test]
fn aggregate_path_mark_and_amplified_clip_work_are_admitted_before_clip_copies() {
    let fonts = fonts();
    let mut source = scene();
    source.marks = (0..MAX_MARKS - 1)
        .map(|index| {
            Mark::new(
                format!("m{index}"),
                Geometry::Circle {
                    x: 0.,
                    y: 0.,
                    radius: 1.,
                },
                Color(0, 0, 0, 255),
            )
        })
        .collect();
    source.validate().unwrap();
    assert!(
        scene_with_painted_labels(&source, &fonts, &options(), "budget")
            .unwrap_err()
            .0
            .contains("mark/layer")
    );

    let mut source = scene();
    let mut path = vec![C::Line(P::new(10., 10.)); 200_000];
    path[0] = C::Move(P::new(10., 10.));
    source.marks.push(Mark::new(
        "full-path-budget",
        Geometry::Path(path),
        Color(0, 0, 0, 255),
    ));
    source.validate().unwrap();
    assert!(
        scene_with_painted_labels(&source, &fonts, &options(), "budget")
            .unwrap_err()
            .0
            .contains("path command")
    );

    let mut source = scene();
    source.labels[0].text = "B".repeat(MAX_LABEL_BYTES);
    source.labels[0].size = 1.;
    let mut path = vec![C::Line(P::new(10., 10.)); 4000];
    path[0] = C::Move(P::new(10., 10.));
    path[3999] = C::Close;
    source.label_clips.insert(
        0,
        ClipPath {
            geometry: Geometry::Path(path),
            rule: ClipRule::NonZero,
        },
    );
    source.validate().unwrap();
    // Copying this clip into every layer would expand 4,000 source commands
    // into over sixteen million owned commands. Admission rejects first.
    assert!(
        scene_with_painted_labels(&source, &fonts, &options(), "budget")
            .unwrap_err()
            .0
            .contains("clip")
    );
    assert_eq!(source.label_clips[&0].command_count(), 4000);

    let curve_clip = ClipPath {
        geometry: Geometry::Path(
            Geometry::Circle {
                x: 120.,
                y: 90.,
                radius: 1000.,
            }
            .commands(),
        ),
        rule: ClipRule::NonZero,
    };
    source.labels[0].text = "B".repeat(256);
    let cost = curve_clip.picking_stats().unwrap();
    assert!(cost.flattened_segments * 1024 > 200_000);
    assert!(curve_clip.command_count() * 1024 < 200_000);
    source.label_clips.insert(0, curve_clip);
    source.validate().unwrap();
    assert!(
        scene_with_painted_labels(&source, &fonts, &options(), "budget")
            .unwrap_err()
            .0
            .contains("clip")
    );
}

// Create a valid in-memory derivative of the authored fixture with a 300-point
// uncolored A outline. Tests remain independent of Python or system fonts.
fn complex_font() -> Vec<u8> {
    fn u16_at(bytes: &[u8], index: usize) -> u16 {
        u16::from_be_bytes(bytes[index..index + 2].try_into().unwrap())
    }
    fn u32_at(bytes: &[u8], index: usize) -> usize {
        u32::from_be_bytes(bytes[index..index + 4].try_into().unwrap()) as usize
    }
    fn table(bytes: &[u8], tag: &[u8; 4]) -> (usize, usize, usize) {
        let record = (0..u16_at(bytes, 4) as usize)
            .map(|index| 12 + index * 16)
            .find(|&index| &bytes[index..index + 4] == tag)
            .unwrap();
        (
            record,
            u32_at(bytes, record + 8),
            u32_at(bytes, record + 12),
        )
    }
    fn checksum(bytes: &[u8]) -> u32 {
        bytes.chunks(4).fold(0u32, |sum, chunk| {
            let mut word = [0; 4];
            word[..chunk.len()].copy_from_slice(chunk);
            sum.wrapping_add(u32::from_be_bytes(word))
        })
    }
    fn replace(bytes: &mut Vec<u8>, record: usize, data: &[u8]) {
        while !bytes.len().is_multiple_of(4) {
            bytes.push(0);
        }
        let offset = bytes.len() as u32;
        bytes.extend_from_slice(data);
        bytes[record + 4..record + 8].copy_from_slice(&checksum(data).to_be_bytes());
        bytes[record + 8..record + 12].copy_from_slice(&offset.to_be_bytes());
        bytes[record + 12..record + 16].copy_from_slice(&(data.len() as u32).to_be_bytes());
    }
    let face = ttf_parser::Face::parse(FONT, 0).unwrap();
    let glyph = face.glyph_index('A').unwrap().0 as usize;
    let count = usize::from(face.number_of_glyphs()) + 1;
    let (head_record, head, _) = table(FONT, b"head");
    let long_loca = u16_at(FONT, head + 50) == 1;
    let (loca_record, loca_offset, loca_length) = table(FONT, b"loca");
    let (glyf_record, glyf_offset, glyf_length) = table(FONT, b"glyf");
    let offsets: Vec<_> = (0..count)
        .map(|index| {
            if long_loca {
                u32_at(FONT, loca_offset + index * 4)
            } else {
                usize::from(u16_at(FONT, loca_offset + index * 2)) * 2
            }
        })
        .collect();
    let mut outline = Vec::new();
    for word in [1i16, 0, 0, 1000, 1000, 299, 0] {
        outline.extend_from_slice(&word.to_be_bytes());
    }
    outline.extend(std::iter::repeat_n(1, 300));
    let points = [(0i16, 0i16), (1000, 0), (1000, 1000), (0, 1000)];
    for axis in [0, 1] {
        let mut previous = 0;
        for index in 0..300 {
            let point = if axis == 0 {
                points[index % 4].0
            } else {
                points[index % 4].1
            };
            outline.extend_from_slice(&(point - previous).to_be_bytes());
            previous = point;
        }
    }
    let growth = outline.len() - (offsets[glyph + 1] - offsets[glyph]);
    let original = &FONT[glyf_offset..glyf_offset + glyf_length];
    let mut glyf = original[..offsets[glyph]].to_vec();
    glyf.extend(outline);
    glyf.extend_from_slice(&original[offsets[glyph + 1]..]);
    let mut loca = FONT[loca_offset..loca_offset + loca_length].to_vec();
    for (index, &offset) in offsets.iter().enumerate().skip(glyph + 1) {
        let offset = offset + growth;
        if long_loca {
            loca[index * 4..index * 4 + 4].copy_from_slice(&(offset as u32).to_be_bytes());
        } else {
            loca[index * 2..index * 2 + 2].copy_from_slice(&((offset / 2) as u16).to_be_bytes());
        }
    }
    let mut bytes = FONT.to_vec();
    replace(&mut bytes, glyf_record, &glyf);
    replace(&mut bytes, loca_record, &loca);
    bytes[head + 8..head + 12].fill(0);
    let head_checksum = checksum(&bytes[head..head + 54]);
    bytes[head_record + 4..head_record + 8].copy_from_slice(&head_checksum.to_be_bytes());
    let adjustment = 0xB1B0AFBAu32.wrapping_sub(checksum(&bytes));
    bytes[head + 8..head + 12].copy_from_slice(&adjustment.to_be_bytes());
    bytes
}

#[test]
fn standalone_scene_allows_complete_layers_above_semantic_graphic_command_limit() {
    let mut fonts = FontRegistry::new();
    fonts.register(complex_font(), 0).unwrap();
    let mut source = scene();
    source.labels[0].text = "A".into();
    source.labels[0].color.3 = 255;
    let text = fonts
        .painted_label(&source.labels[0], MissingGlyphPolicy::Error)
        .unwrap();
    assert!(text.layers[0].commands.len() > 256 && text.layers[0].commands.len() < 8192);
    assert!(painted_labels(&source, &fonts, &options()).is_err());
    let converted = scene_with_painted_labels(&source, &fonts, &options(), "complex").unwrap();
    assert!(render::to_svg(&converted).is_ok());
    assert!(render::to_canvas_script(&converted, "chart").is_ok());
    assert_eq!(
        render_raster(&source, &fonts, &Default::default())
            .unwrap()
            .rgba,
        render_raster(&converted, &FontRegistry::new(), &Default::default())
            .unwrap()
            .rgba
    );
}
