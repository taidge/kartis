//! A complete application-owned Funnel layer with retained Glory controls.
use glory::render::{Event, EventKind, NodeId, Props};
use glory::{App, Widget};
use kartis::*;
#[derive(Clone, Debug)]
struct Source {
    frame: SceneInteraction,
    composition: PlotWidgetsComposition<FunnelWidgetData>,
}
fn source(data: &[FunnelDatum]) -> Result<Source> {
    let config = ChartConfig::default();
    let options = FunnelLayoutOptions {
        labels: None,
        before: Some(FunnelSeparatorOptions {
            length: 100.,
            offset: 10.,
            stroke: None,
        }),
        after: None,
        ..Default::default()
    };
    let mut color = |ctx: &FunnelColorContext<'_>| {
        Ok(if ctx.datum.value >= 60. {
            Color(25, 140, 80, 255)
        } else {
            Color(20, 100, 180, 255)
        })
    };
    let mut format = |value: f64| Ok(format!("{value:.0} tickets"));
    let layout = funnel_layout_with_factories(
        &config,
        data,
        &options,
        &mut FunnelFactories {
            color: Some(&mut color),
            formatted_value: Some(&mut format),
            ..Default::default()
        },
    )?;
    // Explicit interpolation turns an original part profile into geometry.
    // Actual custom-stage ink remains available on its host Mark instead.
    for part in &layout.parts {
        let _ = funnel_part_geometry(part, options.interpolation)?;
    }
    let widgets = funnel_widgets(&layout, data, 0, |ctx| {
        let selected = match ctx.role {
            FunnelWidgetRole::Body => ctx.source_index == Some(0),
            FunnelWidgetRole::BeforeSeparator => ctx.source_index == Some(1),
            _ => false,
        };
        Ok(selected.then_some(PlotWidgetPlacement::AtDatum {
            offset: [-40., -12.],
            measured: TextSize {
                width: 80.,
                height: 24.,
            },
        }))
    })?;
    let replace_ink = widgets
        .layout
        .slots
        .iter()
        .map(|slot| slot.mark_id.clone())
        .collect();
    Ok(Source {
        frame: funnel_widget_frame(&layout)?,
        composition: PlotWidgetsComposition {
            widgets,
            replace_ink,
        },
    })
}
fn children(app: &App) -> Vec<NodeId> {
    let tree = app.tree();
    let mut pending = tree.roots().to_vec();
    let mut result = vec![];
    while let Some(id) = pending.pop() {
        let node = tree.node(id).unwrap();
        if node
            .descriptor
            .view()
            .classes
            .iter()
            .any(|c| c == "funnel-demo")
        {
            result.push(id);
        }
        pending.extend(node.children.iter().copied());
    }
    result
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut data = vec![
        FunnelDatum::new("visits", 80.),
        FunnelDatum::new("sales", 40.),
    ];
    let app = App::memory();
    let input = app.scope().cage(source(&data)?);
    let frame = app.scope().bond(move || input.with(|s| s.frame.clone()));
    let packet = app
        .scope()
        .bond(move || input.with(|s| s.composition.clone()));
    app.mount(chart_layout(frame).plot_widgets_composed(packet, |ctx| {
        let frame = ctx.frame;
        Ok(glory::ui::widget(move |scope| {
            let count = scope.cage(0u32);
            glory::ui::button()
                .class("funnel-demo")
                .label(scope.bond(move || {
                    frame.with(|f| {
                        let part = &f.data.parts[f.data.part_index.unwrap()];
                        format!(
                            "{:?}: {} {}, clicks={}",
                            f.data.role,
                            part.datum.label,
                            part.formatted_value,
                            count.get()
                        )
                    })
                }))
                .on(EventKind::Activate, move |_| count.revise(|n| *n += 1))
                .show_in(scope)
        }))
    }))?;
    let before = children(&app);
    assert_eq!(before.len(), 2);
    for id in &before {
        app.dispatch(*id, Event::Activate)?;
    }
    app.flush()?;
    data[0].value = 60.;
    let next = source(&data)?;
    input.revise(|old| *old = next);
    app.flush()?;
    assert_eq!(children(&app), before);
    for id in before {
        let tree = app.tree();
        let Props::Button(button) = tree.node(id).unwrap().descriptor.view().props else {
            panic!("not a Button")
        };
        assert!(button.label.contains("clicks=1"));
    }
    app.clear()?;
    Ok(())
}
