//! Capture complete Funnel roles once; a browser owner supplies live SVG views.
use kartis::*;

fn main() -> Result<()> {
    let config = ChartConfig {
        width: 480.,
        height: 320.,
        title: "Funnel SVG source".into(),
        ..Default::default()
    };
    let source = [
        FunnelDatum::new("visits", 100.),
        FunnelDatum::new("trials", 40.),
        FunnelDatum::new("sales", 10.),
    ];
    let mut border = |c: &FunnelPartStyleContext<'_>| {
        Ok(if c.part.is_current {
            Color(220, 80, 20, 255)
        } else {
            c.default_color
        })
    };
    let frame = FunnelSvgFrame::with_factories(
        &config,
        &source,
        &FunnelLayoutOptions {
            current: Some("trials".into()),
            current_size_extension: 8.,
            ..Default::default()
        },
        &mut FunnelFactories {
            border_color: Some(&mut border),
            ..Default::default()
        },
    )?;
    let plan = FunnelSvgPlan::new(
        &frame,
        vec![
            FunnelSvgLayer::new("separators", FunnelSvgLayerKind::Separators)?,
            FunnelSvgLayer::new("parts", FunnelSvgLayerKind::Parts)?,
            FunnelSvgLayer::new(
                "user-layer",
                FunnelSvgLayerKind::Custom("application".into()),
            )?,
            FunnelSvgLayer::new("labels", FunnelSvgLayerKind::Labels)?,
            FunnelSvgLayer::new("parts-again", FunnelSvgLayerKind::Parts)?,
        ],
        FunnelSvgBudget::default(),
        FunnelSvgExtrasPolicy::RequireLayer,
    )?;
    for occurrence in plan.layers() {
        println!(
            "{} {:?}: {:?}",
            occurrence.layer.key,
            occurrence.layer.kind,
            occurrence.items()
        );
    }
    for (part, style) in frame.parts().iter().zip(frame.styles()) {
        println!(
            "{} raw={} formatted={} border={:?}",
            part.datum.id.identity()?,
            part.datum.value,
            part.formatted_value,
            style.border_color
        );
    }
    // A Custom registry name is data here. This example constructs no fake
    // Widget or markup: BrowserFunnel owns actual retained layer components.
    Ok(())
}
