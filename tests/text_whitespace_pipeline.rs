use std::collections::BTreeSet;

use kartis::*;

fn scene() -> Scene {
    let mut s = ChartConfig::default().scene().unwrap();
    for (i, text) in ["removed", "  A \t B  "].into_iter().enumerate() {
        s.labels.push(Label {
            x: 40.,
            y: 40. + i as f64 * 30.,
            rotation: 0.,
            text: text.into(),
            color: Color(0, 0, 0, 255),
            size: 20.,
            anchor: TextAnchor::Start,
        });
        s.label_series.insert(i, format!("row-{i}"));
    }
    s.typography
        .label_whitespace
        .insert(1, LabelWhitespace::Preserve);
    s
}

#[test]
fn filtering_and_partial_outlines_remap_preserve_modes() {
    let s = scene();
    let filtered = s.without_series(&BTreeSet::from(["row-0".into()]));
    assert_eq!(filtered.labels.len(), 1);
    assert_eq!(
        filtered.typography.label_whitespace(0),
        LabelWhitespace::Preserve
    );
    filtered.validate().unwrap();
    #[cfg(feature = "raster")]
    {
        use kartis::render::raster::FontRegistry;
        let mut s = s.clone();
        s.labels[0].rotation = 30.;
        let mut fonts = FontRegistry::new();
        fonts
            .register(include_bytes!("fonts/ABeeZee-Regular.ttf").as_slice(), 0)
            .unwrap();
        let output = scene_with_painted_labels(
            &s,
            &fonts,
            &PaintedLabelOptions {
                rotated_only: true,
                ..Default::default()
            },
            "ws",
        )
        .unwrap();
        assert_eq!(output.labels.len(), 1);
        assert_eq!(
            output.typography.label_whitespace(0),
            LabelWhitespace::Preserve
        );
        assert_eq!(output.labels[0].text, s.labels[1].text);
        output.validate().unwrap();
    }
}

#[test]
fn both_motion_engines_keep_source_and_target_whitespace_during_fade() {
    let mut from = scene();
    from.labels.remove(0);
    from.label_series.clear();
    from.typography.label_whitespace.clear();
    let mut to = from.clone();
    to.typography
        .label_whitespace
        .insert(0, LabelWhitespace::Preserve);
    let mut spring = motion::SpringMotion::new(from.clone(), Default::default()).unwrap();
    spring.retarget(to.clone()).unwrap();
    let frame = spring.tick(1. / 60.).unwrap();
    frame.validate().unwrap();
    assert!(frame.labels.len() >= 2);
    let modes: Vec<_> = (0..frame.labels.len())
        .map(|i| frame.typography.label_whitespace(i))
        .collect();
    assert!(modes.contains(&LabelWhitespace::Legacy) && modes.contains(&LabelWhitespace::Preserve));
    let mut properties = motion::PropertyMotion::new(from, Default::default()).unwrap();
    properties.retarget(to).unwrap();
    let frame = properties.tick(1. / 60.).unwrap();
    frame.validate().unwrap();
    let modes: Vec<_> = (0..frame.labels.len())
        .map(|i| frame.typography.label_whitespace(i))
        .collect();
    assert!(modes.contains(&LabelWhitespace::Legacy) && modes.contains(&LabelWhitespace::Preserve));
}
