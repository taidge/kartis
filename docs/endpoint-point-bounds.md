# Endpoint labels, Radar point colors and BoxPlot domain bounds

These additive typed options keep existing defaults and use the shared Scene, SVG and Canvas backends.

| Chart | Option | Behavior |
| --- | --- | --- |
| Bump | `labels.start_style/end_style: Option<BumpEndpointLabelStyle>` | Override padding and inherited color independently for each endpoint. |
| Radar | `points.fill_color_profile/border_color_profile: Option<InheritedColor>` | Resolve point paints from each original series palette color. |
| BoxPlot | `domain_bounds: Option<BoxPlotDomainBounds>` | Choose automatic or fixed values independently for the two value-domain endpoints. |

Each Bump endpoint override contains optional `padding` and `color`. Absent fields inherit the corresponding shared label option. Existing start/end flags still enable labels; the shared font size, source label, endpoint-gap rules, scales, geometry, identity and label ownership remain unchanged. Inherited colors use the computed series color as inherited/surface, theme text as Theme fallback and the Scene background as backdrop. The resolved color alpha is multiplied by series opacity once. Overrides are validated even when that endpoint is disabled or absent; context-dependent color resolution still occurs only when a label is emitted.

For Radar, each color profile overrides its corresponding legacy fixed point color. `None` preserves the old path; `Some(Theme)` uses that same fixed-color/palette fallback. Inherited and surface use `theme.color(series_index)`; backdrop is the chart background. Polygon style overrides keep their separate meaning. In particular, `RadarPointContext.color` remains the legacy polygon fill/border/series swatch, while `fill` and `stroke` contain resolved point paint. Ordinary legend and slice/site colors keep their series meaning; custom factory sites retain the existing actual-paint representative-color rule. A per-series `points` option still replaces the complete global point options rather than merging fields.

The border profile resolves before the existing `border.opacity` factor, applied once. A profile does not enable a border or supply a missing stroke. Stroke-only glyphs and their inherited factory context retain no fill; factories can explicitly return different geometry or paints. Ordinary circles, legacy shapes, shared glyphs and factory context use the same resolved point roles. Configuration modifiers are validated even for empty or disabled output; context-dependent checks run for selected point options of real source series, including hidden and disabled points, before the first factory. Empty input does not fabricate a series context. These are typed RGBA colors; the shared [inherited-color guide](inherited-label-colors.md) describes transparent-color representation boundaries.

BoxPlot `domain_bounds: None` preserves the previous domain path. With `Some`, each `None` endpoint uses the existing inferred extent and each `Some(value)` fixes that source-domain endpoint. The old complete `domain` and new `domain_bounds` cannot be supplied together, including a fully automatic bounds value. Both fixed endpoints bypass automatic empty-domain inference, just as a complete legacy domain does. One fixed nonzero endpoint supplies a sign hint to empty Log/inverse-Pow inference only when its existing source/tick iterator has no values. Real zero or mixed-sign fallback inputs still reject. This finite empty-state rule is a Rust extension and does not claim Nivo's empty min/max behavior.

Bounds can crop original observations without replacing samples or statistics. The existing value-scale validation, UTC integral endpoints, projection guards, clipping and outlier behavior remain in use. Shared continuous-scale nice expansion can extend the final display domain; reverse and rounding keep their existing independent meanings. Legacy scale entry points continue requiring ascending domains, while the configured scale entry point retains its existing descending-domain support. The example checks independent automatic bounds and unchanged quartiles as part of a complete runnable program.

```rust
use kartis::*;
use std::cell::RefCell;

fn main() -> kartis::Result<()> {
    let config = ChartConfig {
        width: 480.,
        height: 320.,
        margin: Margin {
            left: 60.,
            right: 60.,
            top: 60.,
            bottom: 60.,
        },
        theme: Theme {
            colors: vec![Color(20, 40, 60, 255), Color(100, 120, 140, 255)],
            ..Default::default()
        },
        ..Default::default()
    };
    let bump_data = [BumpSeries::new(
        "Alpha",
        vec![BumpDatum::new("Q1", 1.), BumpDatum::new("Q2", 2.)],
    )];
    let mut bump_options = BumpLayoutOptions {
        x_scale: PointScaleOptions {
            padding: 0.,
            ..Default::default()
        },
        axes: BumpAxes::none(),
        legend: false,
        labels: BumpEndpointLabels {
            start: true,
            end: true,
            color: InheritedColor::Fixed(Color(180, 40, 60, 200)),
            ..Default::default()
        },
        style: BumpSeriesStyle {
            opacity: 0.5,
            point: BumpPointStyle {
                size: 0.,
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    };
    let bump_default = bump_layout(&config, &bump_data, &bump_options)?;
    bump_options.labels.start_style = Some(BumpEndpointLabelStyle {
        padding: Some(8.),
        ..Default::default()
    });
    bump_options.labels.end_style = Some(BumpEndpointLabelStyle {
        padding: Some(24.),
        color: Some(InheritedColor::Fixed(Color(40, 60, 180, 120))),
    });
    let bump = bump_layout(&config, &bump_data, &bump_options)?;
    assert_eq!(bump.scene.marks, bump_default.scene.marks);
    assert_eq!(
        bump.scene
            .labels
            .iter()
            .map(|l| (l.x, l.y, l.color))
            .collect::<Vec<_>>(),
        [
            (52., 110., Color(180, 40, 60, 100)),
            (444., 210., Color(40, 60, 180, 60))
        ]
    );
    assert_eq!(
        bump.interaction_binding(),
        bump_default.interaction_binding()
    );

    let axes = ["north", "east", "south", "west"].map(str::to_string);
    let radar_data = [
        RadarSeries::new("alpha", vec![1., 0.5, 0.75, 0.]),
        RadarSeries::new("beta", vec![0.5, 0.75, 0.25, 1.]),
    ];
    let mut radar_options = RadarLayoutOptions {
        domain: Some([0., 1.]),
        radius_ratio: 0.8,
        axis_labels: false,
        legend: false,
        grid: RadarGridOptions {
            line: RadarLineStyle {
                enabled: false,
                ..Default::default()
            },
            spokes: RadarLineStyle {
                enabled: false,
                ..Default::default()
            },
            ..Default::default()
        },
        series: RadarSeriesStyle {
            fill_enabled: false,
            fill_color: Some(Color(9, 18, 27, 255)),
            border: RadarLineStyle {
                enabled: false,
                ..Default::default()
            },
            ..Default::default()
        },
        points: RadarPointOptions {
            size: 12.,
            labels: true,
            border: RadarLineStyle {
                width: 2.,
                opacity: 0.5,
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    };
    let radar_default = radar_layout(
        &config,
        &axes,
        &radar_data,
        &radar_options,
        &Default::default(),
    )?;
    radar_options.points.fill_color_profile =
        Some(InheritedColor::FromContext(vec![ColorModifier::Darker(1.)]));
    radar_options.points.border_color_profile =
        Some(InheritedColor::FromContext(vec![ColorModifier::Brighter(
            1.,
        )]));
    let radar = radar_layout(
        &config,
        &axes,
        &radar_data,
        &radar_options,
        &Default::default(),
    )?;
    assert_eq!(
        radar.interaction_binding(),
        radar_default.interaction_binding()
    );
    for mark in radar
        .scene
        .marks
        .iter()
        .filter(|m| m.id.starts_with("radar-point:"))
    {
        let alpha = mark.series == "alpha";
        assert_eq!(
            mark.fill,
            Some(if alpha {
                Color(14, 28, 42, 255)
            } else {
                Color(70, 84, 98, 255)
            })
        );
        assert_eq!(
            mark.stroke.unwrap().color,
            if alpha {
                Color(29, 57, 86, 128)
            } else {
                Color(143, 171, 200, 128)
            }
        );
    }
    radar_options.points.glyph = Some(GlyphOptions {
        shape: GlyphShape::Stroked(StrokedGlyph::Plus),
        size: 12.,
        ..Default::default()
    });
    let radar_stroked = radar_layout(
        &config,
        &axes,
        &radar_data,
        &radar_options,
        &Default::default(),
    )?;
    assert!(
        radar_stroked
            .scene
            .marks
            .iter()
            .filter(|m| m.id.starts_with("radar-point:"))
            .all(|m| m.fill.is_none())
    );
    radar_options.points.glyph = None;
    let point_events = RefCell::new(Vec::new());
    let radar_factory = radar_layout_with_point_factory(
        &config,
        &axes,
        &radar_data,
        &radar_options,
        &Default::default(),
        |context| {
            point_events
                .borrow_mut()
                .push((context.series.id.clone(), context.axis_id.to_string()));
            assert_eq!(context.color, Color(9, 18, 27, 255));
            let mut mark = context.inherit(Geometry::Rect {
                x: context.position[0] - 5.,
                y: context.position[1] - 5.,
                width: 10.,
                height: 10.,
            });
            mark.opacity = 0.5;
            Ok(mark)
        },
    )?;
    assert_eq!(point_events.borrow().len(), 8);
    assert!(
        radar_factory
            .interaction_binding()
            .sites
            .iter()
            .all(|site| site.color
                == if site.series == "alpha" {
                    Color(14, 28, 42, 255)
                } else {
                    Color(70, 84, 98, 255)
                })
    );

    let observations = ["A", "B", "C"]
        .into_iter()
        .flat_map(|group| {
            [1., 2., 3., 4.]
                .into_iter()
                .enumerate()
                .map(move |(i, value)| {
                    BoxObservation::new(format!("{group}/{i}"), group, "", Some(value))
                })
        })
        .collect::<Vec<_>>();
    let mut box_options = BoxPlotLayoutOptions {
        legend: false,
        inner_padding: 0.,
        value_axis: AxisOptions {
            grid: false,
            tick_values: Some(vec![2., 3.]),
            ..Default::default()
        },
        ..Default::default()
    };
    let box_auto = boxplot_layout(&config, &observations, &box_options)?;
    box_options.domain_bounds = Some(BoxPlotDomainBounds {
        min: Some(2.),
        max: None,
    });
    let box_min = boxplot_layout(&config, &observations, &box_options)?;
    box_options.domain_bounds = Some(BoxPlotDomainBounds {
        min: None,
        max: Some(3.),
    });
    let box_max = boxplot_layout(&config, &observations, &box_options)?;
    assert_eq!(box_auto.domain, [1., 4.]);
    assert_eq!(box_min.domain, [2., 4.]);
    assert_eq!(box_max.domain, [1., 3.]);
    for layout in [&box_auto, &box_min, &box_max] {
        assert!(
            layout
                .boxes
                .iter()
                .all(|b| b.extrema == [1., 4.] && b.quantiles == [1.3, 1.75, 2.5, 3.25, 3.7])
        );
    }
    let _svg = render::to_svg(&bump.scene)?;
    let _script = render::to_canvas_script(&radar.scene, "radar")?;
    Ok(())
}
```

The Bump example inherits the shared start color while changing its padding, then changes both fields at the end. Radar preserves ordinary series-site swatches while custom factory sites report actual point paint. The three BoxPlots retain source extrema 1..4 while displaying domains 1..4, 2..4 and 1..3. See [feature gaps](feature-gaps.md) for the remaining alignment work.
