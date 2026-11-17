#[path = "../examples/shared/widget_raw_bits.rs"]
mod model;
use kartis::*;
#[test]
fn real_browser_model_changes_only_owned_raw_bits_with_stable_scene_and_slot() {
    for kind in 0..6 {
        let a = model::source(kind, 0, 0).unwrap();
        let b = model::source(kind, 1, 0).unwrap();
        assert_eq!(a.frame, b.frame, "kind {kind}");
        assert_eq!(
            render::to_svg(&a.frame.scene).unwrap(),
            render::to_svg(&b.frame.scene).unwrap()
        );
        assert_eq!(a.composition.widgets.layout, b.composition.widgets.layout);
        assert_ne!(a.composition.widgets.data, b.composition.widgets.data);
        assert_eq!(a.composition.widgets.data.len(), 1);
        for d in a.composition.widgets.data.values() {
            assert_eq!(d.kind(), kind);
            assert!(d.bits().iter().all(|v| v == "9223372036854775808"));
        }
        for d in b.composition.widgets.data.values() {
            assert!(d.bits().iter().all(|v| v == "0"));
        }
    }
}
