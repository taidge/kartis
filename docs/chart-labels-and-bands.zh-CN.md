# 径向标签颜色、Sunburst 文字与 BoxPlot 带刻度

这些显式配置沿用普通共享 Scene、SVG 和 Canvas 渲染器。已有入口和默认配置保留原有输出。下方完整示例在修改展示时检查原始值、几何和交互元数据。

| 图型 | 配置 | 作用 |
| --- | --- | --- |
| RadialBar | `RadialBarLabels.color_profile: Option<InheritedColor>` | 根据每个原始分类的 palette 颜色解析标签颜色。 |
| Sunburst | `sunburst_layout_with_label_factory` | 根据原始 datum 和计算节点构建所属标签文字。 |
| BoxPlot | `BoxPlotLayoutOptions.category_scale: Option<BandScaleOptions>` | 配置完整分类带的内外间距、对齐和圆整。 |

RadialBar 的 `None` 继续使用 `labels.color` 或主题文字色。`Some` 支持共享 Theme、Fixed、FromContext 和 Contrast 角色。Theme 使用同一回退色；inherited 和 surface 使用原始分类 palette，backdrop 使用图表背景。自定义填充画笔不会替换 palette 上下文。标签 alpha 继续独立于扇段透明度：示例扇段 opacity 为 0.4，继承色标签仍不透明。即使没有渲染标签，也会校验 modifier 本身。依赖上下文的解析检查拥有真实 `Some` 源值的分类，包括隐藏、零值和被阈值跳过的扇段；稀疏缺失值不会制造上下文。

Sunburst 回调借用 `SunburstLabelContext`：原始 `datum`、计算后的 `node`、`default_text`、已含基线调整的实际 `label_position` 及绘图区。node 提供源索引、深度、聚合值、格式化值、弧形和逻辑锚点。回调运行时，mark 已创建，标签索引尚未分配。回调按广度优先、同层原始兄弟顺序，为每个合格所属标签运行一次。关闭标签、隐藏/排除根节点/零值节点，以及被原弧形或 skip 规则排除的标签均不调用。这是 Rust 标签槽位的调用计数，不承诺与 Nivo 对未渲染数据调用 accessor 的次数相同。

返回空串保留所属标签槽位，但没有字形墨迹。每个结果立即按现有 1,024 字节 UTF-8 上限检查。错误会拒绝构建；闭包对外部状态造成的副作用不回滚。布局不保存回调。自定义文字确定后才捕获测量标签候选和签名，因此应在构建后测量这些候选；其他文字的旧测量会被拒绝。放置、受控隐藏/恢复和换字体仍遵守原有受管标签规则。原始来源、聚合值、格式化值和 tooltip 保持不变。

BoxPlot 的 `category_scale: Some(...)` 将共享 D3 带刻度用于分类位置、坐标轴、子组箱体、中位线、须和离群点。它替换原 `padding` 的计算；旧样式字段本身仍须合法。圆整先作用于完整分类带，再划分子组；`inner_padding` 仍是子组之间的像素间距。返回的 `layout.category_scale` 提供实际完整分类带刻度。可见子组（包括没有观测值但显式保留的槽位）仍要求严格正宽度：D3 本身合法的零带宽配置，或耗尽带宽的子组间距，仍会被拒绝。`None` 保留原布局并返回无显式刻度。`category_reverse` 必须搭配显式 profile，反转完整范围但保留组内子组顺序。反转及额外内外间距/对齐控制属于 D3/Rust 扩展；Nivo 的 BoxPlot `indexScale` 提供圆整配置。

```rust
use kartis::*;
use std::{cell::RefCell, collections::BTreeMap};

fn main() -> kartis::Result<()> {
    let config = ChartConfig {
        width: 420., height: 360.,
        margin: Margin { left: 24., right: 24., top: 24., bottom: 24. },
        ..Default::default()
    };
    let categories = ["low", "high"].map(str::to_string);
    let rows = [BarSeries::new("ring", vec![Some(2.), Some(4.)])];
    let mut radial_options = StackedRadialBarOptions {
        tracks: false, legend: false, opacity: 0.4,
        axes: PolarAxesOptions {
            radial_start: None, circular_outer: None,
            grid: PolarGridStyle { radial: false, circular: false, ..Default::default() },
            ..Default::default()
        },
        colors: BTreeMap::from([
            ("low".into(), Color(20, 40, 60, 255)),
            ("high".into(), Color(200, 100, 50, 255)),
        ]),
        labels: Some(RadialBarLabels {
            text: RadialBarLabelText::Category,
            color: Some(Color(40, 50, 60, 101)),
            ..Default::default()
        }),
        ..Default::default()
    };
    let radial_default = layout_stacked_radial_bar(&config, &categories, &rows, &radial_options)?;
    radial_options.labels.as_mut().unwrap().color_profile =
        Some(InheritedColor::FromContext(vec![ColorModifier::Darker(1.)]));
    let radial = layout_stacked_radial_bar(&config, &categories, &rows, &radial_options)?;
    assert_eq!(radial.scene.marks, radial_default.scene.marks);
    assert_eq!(radial.scene.labels.iter().map(|label| label.color).collect::<Vec<_>>(),
        [Color(14, 28, 42, 255), Color(140, 70, 35, 255)]);
    assert_eq!(radial.interaction_binding()?, radial_default.interaction_binding()?);

    let box_config = ChartConfig {
        width: 221., height: 240.,
        margin: Margin { left: 60., right: 60., top: 24., bottom: 24. },
        ..Default::default()
    };
    let observations = ["A", "B", "C"].into_iter().flat_map(|group| {
        [1., 2., 3.].into_iter().enumerate().map(move |(i, value)|
            BoxObservation::new(format!("{group}/{i}"), group, "", Some(value)))
    }).collect::<Vec<_>>();
    let mut box_options = BoxPlotLayoutOptions { inner_padding: 0., legend: false, ..Default::default() };
    let box_default = boxplot_layout(&box_config, &observations, &box_options)?;
    assert!(box_default.category_scale.is_none());
    box_options.category_scale = Some(scales::BandScaleOptions {
        padding_inner: 0.1, padding_outer: 0.1, align: 0.5, round: true,
    });
    let boxes = boxplot_layout(&box_config, &observations, &box_options)?;
    assert_eq!(boxes.boxes.iter().map(|b| b.center).collect::<Vec<_>>(), [78.5, 110.5, 142.5]);
    assert_eq!(boxes.category_scale.as_ref().unwrap().bandwidth, 29.);
    assert_eq!(boxes.domain, box_default.domain);

    let hierarchy = [
        HierarchyDatum::new("root", None, 0.),
        HierarchyDatum::new("sales", Some("root".into()), 0.),
        HierarchyDatum::new("leaf", Some("sales".into()), 4.),
        HierarchyDatum::new("outside", Some("root".into()), 4.),
    ];
    let sun_options = SunburstLayoutOptions {
        value_format: Some(ValueFormat::Fixed(0)),
        labels: Some(SunburstLabelOptions::default()),
        ..Default::default()
    };
    let sun_default = sunburst_layout(&config, &hierarchy, &sun_options)?;
    let events = RefCell::new(Vec::new());
    let sun = {
        let mut text = |context: &SunburstLabelContext<'_>| {
            events.borrow_mut().push(context.datum.id.clone());
            Ok(if context.datum.id == "outside" { String::new() } else {
                format!("{}@{}:{}", context.datum.id, context.node.depth, context.node.formatted_value)
            })
        };
        sunburst_layout_with_label_factory(&config, &hierarchy, &sun_options, &mut text)?
    };
    assert_eq!(*events.borrow(), ["sales", "outside", "leaf"]);
    assert_eq!(sun.scene.marks, sun_default.scene.marks);
    assert_eq!(sun.interaction_binding()?, sun_default.interaction_binding()?);
    assert_eq!(sun.scene.labels.iter().map(|label| label.text.as_str()).collect::<Vec<_>>(),
        ["sales@1:4", "", "leaf@2:4"]);
    assert_eq!(sun.label_candidates().len(), 3);
    assert_eq!(events.borrow().len(), 3);
    let _svg = render::to_svg(&radial.scene)?;
    let _script = render::to_canvas_script(&sun.scene, "sunburst")?;
    Ok(())
}
```

带刻度示例保留 101 像素绘图区。显式圆整后分类中心为 78.5、110.5、142.5，带宽为 29 像素；原有 profile 继续保留小数中心。两个聚合值同为 4 的 Sunburst 标签按 ID 和深度获得不同文字，第三个返回空的所属标签。字体感知边界见[渲染与字体配置](../README.zh-CN.md)，其余工作见[功能差距](feature-gaps.md)。
