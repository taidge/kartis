# 保留组件中的原始数据更新

开发版，尚未发布。Bump、Scatter、Bar、Stream、Heatmap、Bullet 类型化数据包区分可以观察到的原始 `-0.0` 与 `+0.0`。即使 Scene、测量位置、业务身份及 revision 相同，仅数据变化也会刷新已挂载的子组件。普通图表几何和类别键继续使用数值相等语义。

| 角色 | 精确比较的原始值 |
| --- | --- |
| Bump | 完整 owned 系列中的数值类别、可空 rank，以及当前点的类别/rank |
| Scatter | 完整 owned 系列中的数值 X/Y、可空 size，以及当前站点的 X/Y/size |
| Bar | 完整 owned 系列的可空值及当前柱的原始值 |
| Stream | 完整 owned 系列的可空值及当前样本的类别/值 |
| Heatmap | 原始可空 datum 值及计算 cell 保留的来源值 |
| Bullet | 当前 part 的原始值和原始区间端点 |

共享来源数组使用 `Arc` 身份快速路径；独立分配但内容相同的输入仍然相等。原始数值精确比较保留 NaN 不等语义，合法布局输入为有限数。公开 `CategoryKey`、`Mark`、比例尺及布局几何的相等规则不变。Bullet 提供当前 part，不包含所有原始 range/measure/marker 数组。零占格 Waffle 组不拥有 cell，不能把它选作该 cell 的来源数据。

值更新保持 Widget revision；需要重新构建子组件时才改变 revision。自定义数据类型 `T` 仍须正确实现 `PartialEq`：自动派生的数值比较将有符号零判为相等。子组件观察原始数据时，应在 payload 中保留这些数据。

下面的完整 Memory 应用让实际 Button 的文本从负零位模式更新为正零位模式：

```rust
//! Observe original-value updates independently of equal chart paint.
use glory::render::Props;
use glory::{App, Widget};
use kartis::*;
fn source(value: f64) -> Result<(SceneInteraction, PlotWidgetsData<BumpPointWidgetData>)> {
    let layout = bump_layout(
        &ChartConfig::default(),
        &[BumpSeries::new("row", vec![BumpDatum::new("point", value)])],
        &BumpLayoutOptions {
            axes: BumpAxes::none(),
            legend: false,
            labels: BumpEndpointLabels {
                start: false,
                end: false,
                ..Default::default()
            },
            ..Default::default()
        },
    )?;
    let packet = bump_point_widgets(&layout, 0, |_| {
        Ok(Some(PlotWidgetPlacement::AtDatum {
            offset: [-40., -12.],
            measured: TextSize {
                width: 80.,
                height: 24.,
            },
        }))
    })?;
    Ok((bump_widget_frame(&layout)?, packet))
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let before = source(-0.)?;
    let after = source(0.)?;
    assert_eq!(before.0, after.0); // ordinary numeric Scene equality
    assert_ne!(before.1, after.1); // observable original-source bits
    let app = App::memory();
    let input = app.scope().cage(before);
    let frame = app.scope().bond(move || input.with(|v| v.0.clone()));
    let packet = app.scope().bond(move || input.with(|v| v.1.clone()));
    app.mount(
        chart_layout(frame).plot_widgets_with_data(packet, |context| {
            let frame = context.frame;
            Ok(glory::ui::widget(move |scope| {
                glory::ui::button()
                    .class("raw-example")
                    .label(scope.bond(move || {
                        frame.with(|f| format!("{:016x}", f.data.point.rank.unwrap().to_bits()))
                    }))
                    .show_in(scope)
            }))
        }),
    )?;
    input.revise(|old| *old = after);
    app.flush()?;
    {
        let tree = app.tree();
        let mut pending = tree.roots().to_vec();
        let mut label = None;
        while let Some(id) = pending.pop() {
            let node = tree.node(id).unwrap();
            let view = node.descriptor.view();
            if view.classes.iter().any(|c| c == "raw-example")
                && let Props::Button(p) = view.props
            {
                label = Some(p.label.clone());
            }
            pending.extend(node.children.iter().copied());
        }
        assert_eq!(label.as_deref(), Some("0000000000000000"));
    }
    app.clear()?;
    Ok(())
}
```

运行 `cargo run --example widget_source_values`。来源预检还会在定位回调前拒绝 Stream 数值类别符号位和 Heatmap 来源值被修改的布局；这保护原始快照，同时保留展示数据原有的数值相等规则。

## 浏览器验收边界

六角色浏览器用例仅包装实际角色 payload，没有额外 phase 或缓存 bits 字段来绕过相等比较。用例明确将两阶段兼容性 `Mark.value` 的零都设为正零，类型化原始值保持不动。因此用例的来源 SVG 字节相同；未作这种作者编辑的普通 SVG 元数据可能分别写出 `-0` 与 `0`。

固定参考矩形确保组件替换唯一角色墨迹后，仍有实际 SVG/Canvas 来源绘制平面。脚本检查请求的 renderer 与非空平面。独立精确位字符串、真实子组件状态/焦点、误差不超过 1/32 逻辑像素的实际 DOM 矩形及每帧九个纯色前景采样（每通道允许 ±1）验证功能更新。拒绝候选后，已接受状态、完整根 DOM、SVG 计算绘制样式、原生 Canvas backing 字节及视口几何必须严格相同。脚本核对预期业务错误；JavaScript TypeError 或 WebAssembly trap 会使验收失败。

整视口 PNG 哈希单独记录。此前整页严格门禁仍未通过：移除全部脚本的原生 SVG 载体，在没有 WASM 和候选更新时仍复现一个抗锯齿像素变化；规定的整数对齐也没有消除它。功能验收不代表整页截图严格稳定。font-zero 纯色 Button 不验证任意字形等价；此前 Bar font8 合成差异也仍未解决。

参见[类型化数据包](typed-plot-widgets.zh-CN.md)、[采样指南](viewport-sampling.zh-CN.md)、[完整示例](../examples/widget_source_values.rs)和 [English guide](widget-source-values.md)。自动采样动画组件、任意组件导出、原生同层输入及 GPU/worker 保留为独立差距。
