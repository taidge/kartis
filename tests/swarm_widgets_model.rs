#[path = "../examples/shared/swarm_widgets.rs"]
mod model;
use kartis::*;
#[test]
fn browser_fixture_uses_actual_swarm_role_payloads_and_preserves_raw_bits() {
    let base = model::source(0, 600., 0).unwrap();
    let zero = model::source(8, 600., 0).unwrap();
    assert_eq!(base.frame, zero.frame);
    assert_eq!(
        base.composition.widgets.layout,
        zero.composition.widgets.layout
    );
    assert_ne!(base.composition.widgets.data, zero.composition.widgets.data);
    for phase in 0..12 {
        let source = model::source(phase, 600., 0).unwrap();
        source.frame.validate().unwrap();
        let data = &source.composition.widgets.data;
        assert_eq!(
            data.len(),
            match phase {
                2 => 3,
                6 => 4,
                7 => 3,
                11 => 0,
                _ => 5,
            }
        );
        for d in data.values() {
            assert_eq!(d.site.node_id, d.source[d.source_index].id);
            assert_eq!(d.group.id, d.source[d.source_index].group);
            assert!(!d.group.hidden);
            assert_eq!(d.source.len(), 6);
            assert_eq!(
                d.source
                    .iter()
                    .find(|s| s.id == "b")
                    .unwrap()
                    .value
                    .to_bits(),
                if phase == 8 { 0 } else { (-0_f64).to_bits() }
            );
        }
        if phase == 9 {
            assert_eq!(
                data.values()
                    .find(|d| d.site.node_id == "a")
                    .unwrap()
                    .source_index,
                5
            );
        }
        if phase == 11 {
            assert_eq!(
                source.frame.plot,
                Some(ClipRect {
                    x: 40.,
                    y: 30.,
                    width: 520.,
                    height: 200.
                })
            );
        }
    }
}
