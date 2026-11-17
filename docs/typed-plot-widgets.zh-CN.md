# 类型化实时绘图区 Widget

[English](typed-plot-widgets.md)

`Chart::plot_widgets_with_data` 在图表上方的测量插槽中挂载真实 Glory Widget。
每个子控件收到已接受的 `TypedPlotWidgetFrame<T>`：当前标记、位置、裁剪、交互资格，以及应用拥有的类型化元数据。
适用于无法仅从 `Mark.value` 还原源数据的区间控件、编辑器和分角色控件。
`T` 需要实现 `Clone + PartialEq + 'static`，不要求实现 `Debug`。

## 一个受控输入

将 Scene 和插槽元数据保存在同一受控输入，再从中派生两个 `Vary` 输入。
子控件应在自己的 Bond 和事件处理器中读取 `context.frame`。
直接读取外部候选输入会绕过拒绝边界：该输入可能已包含宿主尚未接受的候选数据。

以下完整示例在 Memory 宿主上挂载真实 Button。
仅更新元数据或更新完整帧，都保留控件键与本地状态。

```rust
use std::collections::BTreeMap;

use glory::render::EventKind;
use glory::{App, Widget};
use kartis::*;

#[derive(Clone, PartialEq)]
struct Payload {
    interval: [f64; 2],
    text: String,
}

fn proposal(value: f64, text: &str) -> Result<(Scene, PlotWidgetsData<Payload>)> {
    let mut scene = ChartConfig::default().scene()?;
    let mut mark = Mark::new(
        "revenue",
        Geometry::Circle { x: 100., y: 80., radius: 10. },
        Color(35, 100, 185, 255),
    );
    mark.series = "sales".into();
    mark.label = "Revenue".into();
    mark.value = value;
    scene.marks.push(mark);
    let slots = PlotWidgetsData {
        layout: PlotWidgetsLayout {
            plot: None,
            slots: vec![PlotWidgetSlot {
                mark_id: "revenue".into(),
                revision: 0,
                placement: PlotWidgetPlacement::AtDatum {
                    offset: [-40., -12.],
                    measured: TextSize { width: 80., height: 24. },
                },
            }],
        },
        data: BTreeMap::from([("revenue".into(), Payload {
            interval: [0., value], text: text.into(),
        })]),
    };
    Ok((scene, slots))
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let app = App::memory();
    let input = app.scope().cage(proposal(10., "Accepted")?);
    let scene = app.scope().bond(move || input.with(|p| p.0.clone()));
    let slots = app.scope().bond(move || input.with(|p| p.1.clone()));
    app.mount(chart(scene).plot_widgets_with_data(slots, |context| {
        let frame = context.frame;
        Ok(glory::ui::widget(move |scope| {
            let count = scope.cage(0usize);
            glory::ui::button()
                .class("revenue-editor")
                .label(scope.bond(move || frame.with(|accepted| {
                    format!("{} {:?}: {} / {} clicks",
                        accepted.data.text,
                        accepted.data.interval,
                        accepted.frame.mark.value,
                        count.get())
                })))
                .on(EventKind::Activate, move |_| count.revise(|n| *n += 1))
                .show_in(scope)
        }))
    }))?;

    // 仅更新元数据：Scene 和子控件修订号不变。
    input.revise(|p| p.1.data.get_mut("revenue").unwrap().text = "Updated".into());
    app.flush()?;

    // 成功构建后，再替换唯一受控输入。
    let next = proposal(20., "Next frame")?;
    input.revise(|p| *p = next);
    app.flush()?;
    app.clear()?;
    Ok(())
}
```

`PlotWidgetsData<T>` 拥有自己的输入数据，和其他 `Vary` API 一样接受普通值、`Cage` 和 `Bond`。
元数据按 `PartialEq` 的值比较，不按内存分配地址比较。
替换为相等元数据不需要重建子控件；即使 Scene 相同，数据变化也会更新已接受的实时帧。
库验证数据包结构，不会替应用验证 `T` 内部任意业务约束。

## 身份和拒绝

- `layout.slots` 最多包含 128 个唯一且真实存在的 `mark_id`，`data` 必须恰好包含这些键。
  元数据缺失或多余、插槽重复、源标记不存在、位置无效，都会拒绝候选帧。
- `mark_id` 在重排和源更新时保留有键包装节点。
  `revision` 不变时保留子控件、本地状态和焦点资格；需要不同的 Widget 实现时再修改修订号。
- Scene、插槽位置和类型化源数据一起准备。
  无效模型、插槽或数据包保留先前已接受的几何与元数据。
  被拒绝的候选 `Cage` 值仍属于应用；宿主不会把输入源改回旧值，应修正后重新提出候选。
- 候选构建器或位置回调可能失败，应先处理结果再发布。
  返回 `Err` 时没有可发布的数据包；回调产生的外部副作用仍由应用负责。
- 子控件工厂或子控件构建失败，会保留该键先前的子控件，并通过正常的拒绝或错误回调报告。
  此时图表帧和数据包可能已经接受，因此保留下来的子控件会通过帧 Bond 读取**新接受的数据**。
  这是子控件修订保留，不是整帧回滚。
  失败修订不会自动重试；使用不同修订号重试，回到先前成功的修订号则继续保留原子控件。

空数据包仍有类型：使用
`PlotWidgetsData::<Payload> { layout: PlotWidgetsLayout::default(), data: BTreeMap::new() }`。
浏览器挂载后，即使没有插槽也保留该元数据类型。
类型化挂载会拒绝后续普通数据包或不同 `T`，普通挂载也会拒绝类型化更新。
现有公开结构体没有新增字段；`Chart::plot_widgets` 和普通浏览器 API 保持原有行为。

## 宿主和位置

浏览器 SVG 和 Canvas 通过 `BrowserChart::mount_with_plot_widgets_data` 使用真实 Glory 子控件。
仅更新类型化插槽时调用 `set_plot_widgets_data`；
需要一起提交 Scene、交互元数据、插槽和类型化源时，调用 `set_layout_with_plot_widgets_data`。
这两个更新方法也存在于 `BrowserHandle`。
Path 需要逻辑数据锚点时，应提供类型化交互帧。
相机变换只应用一次，插槽尺寸使用逻辑坐标，不随 Canvas DPR 改变。

绑定的 `chart_layout(frame)` 现为 `AtDatum` 复用同一份已接受交互锚点，支持自定义
Path；调用 `.raster` 后也先解析源锚点，再应用已接受的图像相机。只有 Scene 的 Path
继续要求显式矩形。参见[Stream 点组件](stream-widgets.zh-CN.md)，其中原始采样锚点
与实际偏移墨迹分开保留。

CPU 图片与实时子控件可使用
`raster_chart(scene, fonts, raster_options).plot_widgets_with_data(slots, factory)`。
已有栅格发布事务覆盖图片绘制、编码、源发布、已接受的 Scene 和类型化插槽。
同步发布器返回 `Err` 时一起保留旧帧。原生应用仍需提供自己的图片提供器和发布器。
这是 `raster_chart` 返回的 `Chart` API，没有给 `RasterSurface` guard 新增叠加插槽接口。

插槽使用明确的测量矩形或 `AtDatum` 偏移。源标记和插槽仍存在时，宿主隐藏与裁剪会保留子控件状态。
要退役子控件，应在同一候选中删除其插槽及同键元数据；若也要删除 Scene 标记，应一并提交。
仅移除标记却保留插槽会使 `Chart` 拒绝整个候选。
部分裁剪的子控件可以绘制可见部分，但处于 inert 状态，不能通过该部分插槽获得焦点或指针激活。
浏览器路径遮罩使用现有遮罩宿主；语义和栅格插槽会拒绝部分覆盖的非矩形路径遮罩。
输入归子 Widget 自己处理，不会自动选择底层图表标记。
弹出层遵循 Glory 单独的宿主定位与输入策略。

实时 Widget 是叠加层，原 Scene 绘制内容仍保留。
SVG、Canvas、PNG 导出只包含 Scene，不包含实时子 Widget。
这不是 React 组件或 `animatedProps` 执行层。
Bullet 的 range、measure 和 marker 类型化插槽见 [Bullet Widget](bullet-widgets.zh-CN.md)。


显式类型化数据包还可绑定空标签的真实装饰 Mark，例如 Waffle 空单元格。
精确匹配的元数据键表明绑定意图，原始 Mark 不变，也不创建源 datum、控制或拾取站点。
旧无类型插槽入口继续保留 data-Mark 规则。见[HeatMap 与 Waffle 单元格适配](cell-widgets.zh-CN.md)。
