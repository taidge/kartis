//! Real Memory children must observe bit-only original-source updates even
//! when the source Scene, rendered SVG, binding and placement are unchanged.
use std::sync::Arc;

use glory::render::{Event, EventKind, NodeId, Props};
use glory::{App, Bond, Widget};
use kartis::*;

#[derive(Clone)]
struct Source<T: Clone + PartialEq + 'static> {
    frame: SceneInteraction,
    composition: PlotWidgetsComposition<T>,
}

fn config() -> ChartConfig {
    ChartConfig {
        width: 320.,
        height: 220.,
        margin: Margin {
            left: 24.,
            right: 24.,
            top: 24.,
            bottom: 24.,
        },
        ..Default::default()
    }
}
fn placement() -> PlotWidgetPlacement {
    PlotWidgetPlacement::AtDatum {
        offset: [-12., -6.],
        measured: TextSize {
            width: 24.,
            height: 12.,
        },
    }
}
fn bits(c: &CategoryKey) -> String {
    match c {
        CategoryKey::Number(n) => n.to_bits().to_string(),
        CategoryKey::String(s) => format!("text:{s}"),
    }
}
fn optional(n: Option<f64>) -> String {
    n.map(|n| n.to_bits().to_string())
        .unwrap_or_else(|| "None".into())
}
fn node(app: &App) -> NodeId {
    let tree = app.tree();
    let mut queue = tree.roots().to_vec();
    let mut out = vec![];
    while let Some(id) = queue.pop() {
        let n = tree.node(id).unwrap();
        if n.descriptor
            .view()
            .classes
            .iter()
            .any(|s| s == "raw-bit-child")
        {
            out.push(id);
        }
        queue.extend(n.children.iter().copied());
    }
    assert_eq!(out.len(), 1);
    out[0]
}
fn text(app: &App, id: NodeId) -> String {
    let t = app.tree();
    let Props::Button(p) = t.node(id).unwrap().descriptor.view().props else {
        panic!("Button expected")
    };
    p.label.clone()
}
fn child<T: Clone + PartialEq + 'static>(
    frame: Bond<TypedPlotWidgetFrame<T>>,
    read: fn(&T) -> String,
) -> impl Widget {
    glory::ui::widget(move |scope| {
        let count = scope.cage(0);
        let focus = scope.cage(false);
        glory::ui::button()
            .class("raw-bit-child")
            .label(scope.bond(move || {
                frame.with(|f| {
                    format!(
                        "{}|count={}|focus={}",
                        read(&f.data),
                        count.get(),
                        focus.get()
                    )
                })
            }))
            .on(EventKind::Activate, move |_| count.revise(|n| *n += 1))
            .on(EventKind::FocusChanged, move |e| {
                if let Event::FocusChanged(v) = e {
                    focus.revise(|f| *f = v)
                }
            })
            .show_in(scope)
    })
}
fn memory<T: Clone + PartialEq + 'static>(
    first: Source<T>,
    second: Source<T>,
    read: fn(&T) -> String,
    strict: bool,
) {
    assert_eq!(
        first.frame, second.frame,
        "ordinary Scene/binding equality must remain unchanged"
    );
    assert_eq!(
        render::to_svg(&first.frame.scene).unwrap(),
        render::to_svg(&second.frame.scene).unwrap(),
        "actual authored SVG paint must be byte-identical"
    );
    assert_eq!(
        first.composition.widgets.layout,
        second.composition.widgets.layout
    );
    let old = read(first.composition.widgets.data.first_key_value().unwrap().1);
    let new = read(second.composition.widgets.data.first_key_value().unwrap().1);
    assert_ne!(old, new, "test must change owned raw bits");
    let app = App::memory();
    let source = app.scope().cage(first);
    let frame = app.scope().bond(move || source.with(|s| s.frame.clone()));
    if strict {
        let packet = app
            .scope()
            .bond(move || source.with(|s| s.composition.clone()));
        app.mount(
            chart_layout(frame)
                .legend(false)
                .tooltip(false)
                .highlight(false)
                .plot_widgets_composed(packet, move |c| Ok(child(c.frame, read))),
        )
        .unwrap();
    } else {
        let packet = app
            .scope()
            .bond(move || source.with(|s| s.composition.widgets.clone()));
        app.mount(
            chart_layout(frame)
                .legend(false)
                .tooltip(false)
                .highlight(false)
                .plot_widgets_with_data(packet, move |c| Ok(child(c.frame, read))),
        )
        .unwrap();
    }
    let id = node(&app);
    app.dispatch(id, Event::Activate).unwrap();
    app.dispatch(id, Event::FocusChanged(true)).unwrap();
    assert_eq!(text(&app, id), format!("{old}|count=1|focus=true"));
    source.revise(|s| *s = second);
    app.flush().unwrap();
    assert_eq!(
        node(&app),
        id,
        "bit-only updates must retain the actual child NodeId"
    );
    assert_eq!(
        text(&app, id),
        format!("{new}|count=1|focus=true"),
        "accepted source raw bits were swallowed although real child state survived"
    );
    app.clear().unwrap();
    assert_eq!(app.stats().nodes, 0);
}

#[derive(Clone, Copy)]
enum BumpCase {
    Rank,
    X,
    SiblingRank,
    SiblingX,
}
fn bump(case: BumpCase, positive: bool) -> Source<BumpPointWidgetData> {
    let zero = if positive { 0. } else { -0. };
    let mut selected = BumpDatum::new("selected", 2.);
    let mut sibling = BumpDatum::new("sibling", 1.);
    match case {
        BumpCase::Rank => selected.rank = Some(zero),
        BumpCase::X => selected.x = zero.into(),
        BumpCase::SiblingRank => sibling.rank = Some(zero),
        BumpCase::SiblingX => sibling.x = zero.into(),
    }
    let o = BumpLayoutOptions {
        axes: BumpAxes::none(),
        legend: false,
        labels: BumpEndpointLabels {
            start: false,
            end: false,
            ..Default::default()
        },
        style: BumpSeriesStyle {
            width: 0.,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut layout = bump_layout(
        &config(),
        &[BumpSeries::new(
            "original",
            vec![selected, sibling, BumpDatum::gap("gap")],
        )],
        &o,
    )
    .unwrap();
    // Compatibility scalar is not original typed source; author the same +0
    // scalar on both frames so even SVG metadata bytes are identical.
    for mark in &mut layout.scene.marks {
        if mark.value == 0. {
            mark.value = 0.;
        }
    }
    let widgets = bump_point_widgets(&layout, 0, |c| {
        Ok((c.point.source_index == 0).then(placement))
    })
    .unwrap();
    let replace_ink = widgets.data.keys().cloned().collect();
    Source {
        frame: bump_widget_frame(&layout).unwrap(),
        composition: PlotWidgetsComposition {
            widgets,
            replace_ink,
        },
    }
}
fn bump_read(d: &BumpPointWidgetData) -> String {
    format!(
        "point-x={};point-rank={};source={}",
        bits(&d.point.raw_x),
        optional(d.point.rank),
        d.series
            .data
            .iter()
            .map(|p| format!("{}:{}", bits(&p.x), optional(p.rank)))
            .collect::<Vec<_>>()
            .join(",")
    )
}

#[derive(Clone, Copy)]
enum ScatterCase {
    X,
    Y,
    Size,
    SiblingX,
    SiblingY,
    SiblingSize,
    NullableSize,
}
fn scatter(case: ScatterCase, positive: bool) -> Source<ScatterNodeWidgetData> {
    let zero = if positive { 0. } else { -0. };
    let mut selected = ScatterDatum::new("selected", 1., 2.);
    selected.size_value = Some(1.);
    let mut sibling = ScatterDatum::new("sibling", -1., -2.);
    sibling.size_value = None;
    match case {
        ScatterCase::X => selected.x = zero.into(),
        ScatterCase::Y => selected.y = zero.into(),
        ScatterCase::Size => selected.size_value = Some(zero),
        ScatterCase::SiblingX => sibling.x = zero.into(),
        ScatterCase::SiblingY => sibling.y = zero.into(),
        ScatterCase::SiblingSize => sibling.size_value = Some(zero),
        ScatterCase::NullableSize => selected.size_value = positive.then_some(0.),
    }
    let raw = vec![ScatterSeries::new("original", vec![selected, sibling])];
    let axis = ScatterAxisScale::Continuous {
        options: ContinuousScaleOptions {
            nice: ScaleNice::Off,
            ..Default::default()
        },
        min: ScatterBound::Fixed(-4.),
        max: ScatterBound::Fixed(4.),
    };
    let o = ScatterLayoutOptions {
        axes: ScatterAxes {
            top: None,
            right: None,
            bottom: None,
            left: None,
        },
        scales: ScatterScaleOptions {
            x: axis.clone(),
            y: axis,
        },
        ..Default::default()
    };
    let mut layout = scatter_layout(&config(), &raw, &o).unwrap();
    for mark in &mut layout.scene.marks {
        if mark.value == 0. {
            mark.value = 0.;
        }
    }
    let widgets = scatter_node_widgets(&layout, &raw, 0, |c| {
        Ok((c.site.source_index == 0).then(placement))
    })
    .unwrap();
    let mut binding = layout.interaction_binding();
    binding.picking_profile = PickingProfile::PaintedGeometry;
    let frame = SceneInteraction {
        scene: layout.scene,
        plot: Some(layout.plot),
        interaction: Some(binding),
    };
    frame.validate().unwrap();
    let replace_ink = widgets.data.keys().cloned().collect();
    Source {
        frame,
        composition: PlotWidgetsComposition {
            widgets,
            replace_ink,
        },
    }
}
fn scatter_read(d: &ScatterNodeWidgetData) -> String {
    format!(
        "site-x={};site-y={};site-size={};source={}",
        bits(&d.site.raw_x),
        bits(&d.site.raw_y),
        optional(d.site.size_value),
        d.series
            .data
            .iter()
            .map(|p| format!("{}:{}:{}", bits(&p.x), bits(&p.y), optional(p.size_value)))
            .collect::<Vec<_>>()
            .join(",")
    )
}

macro_rules! bump_tests {($($name:ident,$case:ident,$strict:expr;)*)=>{$(#[test]fn $name(){memory(bump(BumpCase::$case,false),bump(BumpCase::$case,true),bump_read,$strict);})*};}
bump_tests! {
 bump_rank_overlay,Rank,false; bump_rank_composed,Rank,true;
 bump_x_overlay,X,false; bump_x_composed,X,true;
 bump_unselected_rank_overlay,SiblingRank,false; bump_unselected_rank_composed,SiblingRank,true;
 bump_unselected_x_overlay,SiblingX,false; bump_unselected_x_composed,SiblingX,true;
}
macro_rules! scatter_tests {($($name:ident,$case:ident,$strict:expr;)*)=>{$(#[test]fn $name(){memory(scatter(ScatterCase::$case,false),scatter(ScatterCase::$case,true),scatter_read,$strict);})*};}
scatter_tests! {
 scatter_x_overlay,X,false; scatter_x_composed,X,true;
 scatter_y_overlay,Y,false; scatter_y_composed,Y,true;
 scatter_size_overlay,Size,false; scatter_size_composed,Size,true;
 scatter_unselected_x_overlay,SiblingX,false; scatter_unselected_x_composed,SiblingX,true;
 scatter_unselected_y_overlay,SiblingY,false; scatter_unselected_y_composed,SiblingY,true;
 scatter_unselected_size_overlay,SiblingSize,false; scatter_unselected_size_composed,SiblingSize,true;
 scatter_nullable_size_overlay,NullableSize,false; scatter_nullable_size_composed,NullableSize,true;
}

#[test]
fn fresh_allocations_with_same_bits_remain_value_equal_and_share_original_series() {
    let a = bump(BumpCase::Rank, false);
    let b = bump(BumpCase::Rank, false);
    let x = a.composition.widgets.data.first_key_value().unwrap().1;
    let y = b.composition.widgets.data.first_key_value().unwrap().1;
    assert!(!Arc::ptr_eq(&x.series, &y.series));
    assert_eq!(x, y);
    assert_eq!(x, &x.clone());
    let a = scatter(ScatterCase::Size, false);
    let b = scatter(ScatterCase::Size, false);
    let x = a.composition.widgets.data.first_key_value().unwrap().1;
    let y = b.composition.widgets.data.first_key_value().unwrap().1;
    assert!(!Arc::ptr_eq(&x.series, &y.series));
    assert_eq!(x, y);
    assert_eq!(x, &x.clone());
    assert_eq!(CategoryKey::Number(-0.), CategoryKey::Number(0.));
    assert_eq!(
        CategoryKey::Number(-0.).identity().unwrap(),
        CategoryKey::Number(0.).identity().unwrap()
    );
}

#[test]
fn shared_source_fast_path_still_compares_current_raw_point_fields_and_other_metadata() {
    let packet = bump(BumpCase::Rank, false);
    let a = packet.composition.widgets.data.first_key_value().unwrap().1;
    let mut b = a.clone();
    b.point.rank = Some(0.);
    assert!(Arc::ptr_eq(&a.series, &b.series));
    assert_ne!(a, &b);
    let mut b = a.clone();
    b.point.raw_x = CategoryKey::Number(-0.);
    let mut c = b.clone();
    c.point.raw_x = CategoryKey::Number(0.);
    assert_ne!(b, c);
    let mut b = a.clone();
    b.color = Color(1, 2, 3, 4);
    assert_ne!(a, &b);
    let mut b = a.clone();
    b.series_index += 1;
    assert_ne!(a, &b);
    let mut b = a.clone();
    Arc::make_mut(&mut b.series).label.push('!');
    assert_ne!(a, &b);

    let packet = scatter(ScatterCase::Size, false);
    let a = packet.composition.widgets.data.first_key_value().unwrap().1;
    let mut b = a.clone();
    b.site.size_value = Some(0.);
    assert!(Arc::ptr_eq(&a.series, &b.series));
    assert_ne!(a, &b);
    for horizontal in [false, true] {
        let mut b = a.clone();
        let mut c = b.clone();
        if horizontal {
            b.site.raw_x = (-0_f64).into();
            c.site.raw_x = 0_f64.into();
        } else {
            b.site.raw_y = (-0_f64).into();
            c.site.raw_y = 0_f64.into();
        }
        assert_ne!(b, c);
    }
    let mut b = a.clone();
    b.site.formatted_x.push('!');
    assert_ne!(a, &b);
    let mut b = a.clone();
    Arc::make_mut(&mut b.series).data[1].label = Some("changed sibling label".into());
    assert_ne!(a, &b);
    // Resolved geometry is not raw provenance; its ordinary numeric equality
    // is deliberately retained, including signed zero.
    let mut b = a.clone();
    b.site.node_size = -0.;
    let mut c = b.clone();
    c.site.node_size = 0.;
    assert_eq!(b, c);
}
