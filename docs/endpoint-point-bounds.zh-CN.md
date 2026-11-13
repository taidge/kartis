# 端点标签、Radar 点颜色与 BoxPlot 值域上下界

这些新增类型化配置保留已有默认行为，沿用共享 Scene、SVG 和 Canvas 后端。

| 图型 | 配置 | 行为 |
| --- | --- | --- |
| Bump | `labels.start_style/end_style: Option<BumpEndpointLabelStyle>` | 为两端分别覆盖 padding 和继承颜色。 |
| Radar | `points.fill_color_profile/border_color_profile: Option<InheritedColor>` | 根据每个原始系列的 palette 颜色解析点画笔。 |
| BoxPlot | `domain_bounds: Option<BoxPlotDomainBounds>` | 分别选择值域两端的自动值或固定值。 |

Bump 每端覆盖包含可选 `padding` 和 `color`；未指定字段继承对应的共享标签配置。原 start/end 开关仍决定是否启用标签，共享字号、源标签、端点缺段规则、比例尺、几何、身份及标签归属保持不变。继承色的 inherited/surface 使用计算后的系列颜色，Theme 回退到主题文字色，backdrop 使用 Scene 背景。解析后的颜色 alpha 再乘一次系列 opacity。即使端点关闭或缺失也会校验覆盖配置本身；依赖具体上下文的求色仍只在实际发出标签时运行。

Radar 每个颜色 profile 覆盖对应的原有固定点颜色。`None` 保留旧路径；`Some(Theme)` 使用相同的固定色/palette 回退。inherited 和 surface 使用 `theme.color(series_index)`，backdrop 是图表背景。多边形样式覆盖继续独立生效：`RadarPointContext.color` 保留原多边形填充/边框/系列 swatch 语义，`fill` 和 `stroke` 则提供已解析的点画笔。普通图例及 slice/site 颜色保留系列语义；自定义工厂站点继续按实际画笔选择代表色。逐系列 `points` 仍整体替换全局点配置，不逐字段合并。

边框 profile 先求色，再应用一次原 `border.opacity`。profile 不会启用边框或补造 stroke。仅描边 glyph 及继承它的工厂上下文保持无 fill；工厂可显式返回不同几何或画笔。普通圆点、旧 shape、共享 glyph 和工厂上下文使用同一解析后的点颜色角色。空数据或关闭输出仍校验 modifier 本身；具体上下文检查在首个工厂之前针对真实来源系列所选择的点配置运行，包括隐藏或关闭的点。空输入不会制造系列上下文。这些颜色采用类型化 RGBA；透明色的表示边界见[继承颜色指南](inherited-label-colors.zh-CN.md)。

BoxPlot 的 `domain_bounds: None` 保留原值域路径。启用 `Some` 后，每端 `None` 沿用现有推导范围，每端 `Some(value)` 固定该源值域端点。旧完整 `domain` 与新 `domain_bounds` 不能同时设置，即使新 bounds 全为自动值。两端固定时与旧完整 domain 一样跳过自动空域推导。仅一端固定且原非零域推导使用的来源/刻度迭代器确实无值时，才用该固定端为 Log/负指数 Pow 空域提供符号提示；真实零值或混号来源仍会被拒绝。这是 Rust 的有限空态扩展，不宣称复现 Nivo 的空 min/max 行为。

固定上下界可以裁切原观测值，不会替换样本或统计量。值轴校验、UTC 整数端点、投影保护、裁剪和离群点规则继续使用原实现。共享连续比例尺的 nice 可能扩展最终展示值域，reverse 和 round 保留原有独立含义。旧比例尺入口继续要求升序域，显式配置入口保留原有降序域支持。下方完整程序同时检查两端独立自动推导，以及保持不变的四分位数。

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

Bump 示例仅改变起点间距并继承共享颜色，再同时覆盖终点的两个字段。Radar 保留普通系列站点的 swatch，自定义工厂站点则报告实际点画笔。三个 BoxPlot 的源极值均为 1..4，展示域分别为 1..4、2..4、1..3。其余对齐工作见[功能差距](feature-gaps.md)。
