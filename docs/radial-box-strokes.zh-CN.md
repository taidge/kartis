# RadialBar 与 BoxPlot 描边继承颜色

这四个可选角色复用 `InheritedColor`，不改变固定默认值。每项默认均为 `None`；`Some(InheritedColor::Theme)` 解析为该角色已有描边的颜色。单独配置 profile 不会启用缺失的边框，已配置的宽度与端帽仍保持原义。

| 配置 | 继承色与对比度表面 | Theme 回退 |
| --- | --- | --- |
| `StackedRadialBarOptions.border_color_profile` | `paints` 之前的原数值分类色盘 | 扇段 `border.color` |
| `BoxPlotLayoutOptions.border_color_profile` | 原数值子组色盘 | 箱体 `border.color` |
| `BoxPlotLayoutOptions.median_color_profile` | 原数值子组色盘 | `median.color` |
| `BoxPlotLayoutOptions.whisker_color_profile` | 原数值子组色盘 | `whisker.color` |

完整程序检查解析后的 RGBA 与标记保存的透明度，再通过普通 SVG 和 Canvas API 导出两个 Scene。`ColorModifier::Opacity` 替换角色颜色自身的 alpha，不替换或预乘标记透明度。

```rust
use kartis::*;
use glory::render::{GraphicStroke, StrokeCap};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let config = ChartConfig::default();
    let inherited = |darker, opacity| {
        InheritedColor::FromContext(vec![
            ColorModifier::Darker(darker),
            ColorModifier::Opacity(opacity),
        ])
    };
    let border = GraphicStroke {
        color: Color(70, 80, 90, 101),
        width: 3.,
        cap: StrokeCap::Square,
    };
    let categories = vec!["sales".into(), "cost".into()];
    let rings = vec![BarSeries::new("quarter", vec![Some(2.), Some(8.)])];
    let radial = layout_stacked_radial_bar_with_arc_style(
        &config,
        &categories,
        &rings,
        &StackedRadialBarOptions {
            colors: [("sales".into(), Color(20, 40, 60, 160))].into(),
            border: Some(border),
            border_color_profile: Some(inherited(1., 0.5)),
            opacity: 0.4,
            ..Default::default()
        },
        &StackedRadialArcStyle {
            corner_radius: 6.,
            ..Default::default()
        },
    )?;
    let sales = radial
        .scene
        .marks
        .iter()
        .find(|mark| mark.series == "sales")
        .ok_or("missing sales segment")?;
    assert_eq!(sales.stroke.map(|s| s.color), Some(Color(14, 28, 42, 128)));
    assert_eq!(sales.opacity, 0.4);
    radial.interaction_binding()?;

    let samples = [1., 2., 3., 4.]
        .into_iter()
        .enumerate()
        .map(|(i, value)| BoxObservation::new(i.to_string(), "Q1", "sales", Some(value)))
        .collect::<Vec<_>>();
    let boxes = boxplot_layout(
        &config,
        &samples,
        &BoxPlotLayoutOptions {
            colors: [("sales".into(), Color(20, 40, 60, 160))].into(),
            border: Some(border),
            border_color_profile: Some(inherited(1., 0.5)),
            median_color_profile: Some(inherited(2., 0.75)),
            whisker_color_profile: Some(InheritedColor::FromContext(vec![ColorModifier::Opacity(
                0.25,
            )])),
            fill_opacity: 0.2,
            ..Default::default()
        },
    )?;
    for (part, expected) in [
        (":border", Color(14, 28, 42, 128)),
        (":median", Color(10, 20, 29, 191)),
        (":whiskers", Color(20, 40, 60, 64)),
        (":caps", Color(20, 40, 60, 64)),
    ] {
        let mark = boxes
            .scene
            .marks
            .iter()
            .find(|mark| mark.id.ends_with(part))
            .ok_or("missing boxplot stroke")?;
        assert_eq!(mark.stroke.map(|s| s.color), Some(expected));
        assert_eq!(mark.opacity, 1.);
    }
    for scene in [&radial.scene, &boxes.scene] {
        scene.validate()?;
        assert!(render::to_svg(scene)?.contains("<svg"));
        assert!(!render::to_canvas_script(scene, "chart")?.is_empty());
    }
    Ok(())
}
```

RadialBar 在选择自定义填充画笔之前，从原分类色盘解析扇段边框。轨道及其颜色保持独立。普通和弧形样式的 Scene／布局入口共用这个角色。实际 `Some` 来源即使已隐藏、值为零或因 padding 坍缩，仍提供颜色上下文；空来源及全 `None` 来源不补造上下文。没有可见墨迹时，modifier 的内在配置仍需校验。对于被 padding 压缩到坍缩的弧形样式扇段，是否保留仅轮廓墨迹，由解析后的边框 alpha 和扇段透明度决定，不能在覆盖颜色后仍由旧固定颜色判断。RadialBar 已配置边框的宽度必须为正：宽度为 `0.0` 或 `-0.0` 的 `Some` 会被拒绝，禁用边框应使用 `border: None`。其余零值、坍缩几何及交互策略保持原有规则。

BoxPlot 沿用既有子组色盘和包含隐藏身份的完整子组顺序，因此隐藏一个子组不会重排其他子组的色盘槽位。`boxplot_layout` 与 `boxplot_layout_with_scale_options` 共用这些角色。只有实际可见且非空的摘要箱体才有上下文。预留空槽位和仅含缺失值的观测不会变成零值箱体；真正的常值箱体仍有上下文。对于这样的真实箱体，即使没有边框，或中位线、须的宽度为零，profile 仍会校验。边框缺失时不输出描边，其未使用的 Theme 回退为数值色盘。

BoxPlot 的三个描边角色均独立于 `fill_opacity`。示例中，箱体填充标记透明度为 0.2，边框、中位线、须及端帽仍分别保存 alpha 128、191、64。须的角色同时作用于两段连线和两个端帽。对于 BoxPlot，`border: Some(...)` 和 `outlier_border: Some(...)` 的宽度为 `0.0` 或 `-0.0` 时，与 `None` 一样禁用该描边。离群点边框禁用后，已填充离群点的填充、半径和几何保持不变。中位线和须的宽度为零时仍沿用原有跳过行为，`cap_size: 0` 仍禁用端帽。这些角色不重设离群点、标签、坐标轴、网格或图例的颜色，来源 ID、统计量、值域及尺度映射保持不变。本次没有新增按 group 取色的 `colorBy`、BoxPlot 自定义 `FillPaint` API 或私有受控工厂更新协议。数据、色盘或 profile 改变后，需要重新构建布局。

`Fixed`、有序 modifier 和 `Contrast` 均使用共享解析器。Contrast 是 Rust 扩展，按原数值 RGBA 表面与图表背景计算；半透明表面要求不透明 backdrop。它不承诺渐变、图案或 fill-opacity 合成后每个像素的视觉对比度。类型化颜色在 alpha 为零时保留通道，以及它与 CSS 恢复 alpha 的差异，见[边框颜色角色](border-color-roles.zh-CN.md)。

生成的普通 Scene 可由 SVG、Canvas、CPU raster 和 Glory 宿主在各自既有能力范围内消费。语义 Graphic 的渐变、图案限制保持不变，这些角色没有新增语义渐变支持。独立的类别带、标签选项和汇总值域配置见[标签与带刻度](chart-labels-and-bands.zh-CN.md)及[汇总值域](summary-index-legend.zh-CN.md)。
