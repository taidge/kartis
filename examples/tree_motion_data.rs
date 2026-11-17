//! Explicit-clock typed properties. This example does not mount or paint a chart.
use kartis::*;

fn capture(config: ChartConfig, source: &[HierarchyDatum]) -> Result<TreeEventFrame> {
    let layout = tree_layout(&config, source, &TreeOptions::default())?;
    TreeEventFrame::new(config, layout)
}

fn main() -> Result<()> {
    let source = vec![
        HierarchyDatum::new("root", None, -0.),
        HierarchyDatum::new("left", Some("root".into()), 3.),
        HierarchyDatum::new("right", Some("root".into()), 5.),
    ];
    let config = ChartConfig::default();
    let mut target_config = config.clone();
    target_config.margin.left += 30.;
    let mut motion = TreeMotion::new(capture(config, &source)?, TreeMotionOptions::default())?;
    motion.retarget(capture(target_config, &source)?)?;
    for _ in 0..6 {
        motion.tick(1. / 60.)?;
    }
    let root = motion
        .frame()
        .nodes()
        .iter()
        .find(|node| node.key() == "root")
        .expect("the authored root is present");
    println!(
        "root: sampled x={:.4}, target x={:.4}, velocity={:.4}, raw bits={}",
        root.props().x,
        root.states()[0].target,
        root.states()[0].velocity,
        root.data().datum().value.to_bits(),
    );
    motion.finish()?;
    assert!(motion.is_settled());
    Ok(())
}
