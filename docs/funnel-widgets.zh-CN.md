# Funnel 函数与实时自定义图层角色（未发布）

[English](funnel-widgets.md)

这是开发中的 API，尚未包含在已发布的 0.1.0 包中。

Nivo Funnel 提供函数形式的 `layers`，传入计算后的 parts、前后分隔线、
area/border 路径生成器和 current-part setter。它没有 `partComponent`、
`labelComponent` 或 `separatorComponent`。Kartis 提供显式 Rust 函数配置，
并为实际 Funnel 标记提供类型化实时 Widget 适配器；这些角色是对自定义图层
使用场景的 Rust 扩展。`funnel_layout_with_stage_factory` 修改的是 Scene 几何，
不会创建 Glory Widget。

## 函数配置与执行顺序

`FunnelFactories` 在一次布局构建中借用回调：

这些适配器以显式 Rust 类型覆盖 Nivo 的函数型 `colors`、`valueFormat`、
`borderColor` 和 `labelColor` 输入。

| 字段 | 输入与结果 |
| --- | --- |
| `color` | `FunnelColorContext` 包含原始 `datum`、源索引、默认颜色和 plot；返回颜色，覆盖调色板或显式 datum 颜色的计算结果，但不改原数据。 |
| `formatted_value` | 输入原始 `f64`，返回拥有所有权的字符串；未提供时沿用 `FunnelLayoutOptions.value_format`。 |
| `border_color` | `FunnelPartStyleContext` 包含计算后的 part、plot、默认颜色、填充和边框透明度、当前边框宽度；返回颜色。 |
| `label_color` | 计算后的样式上下文还包含已解析的边框颜色；返回颜色。 |

使用 `funnel_layout_with_factories`、`funnel_layout_with_size_and_factories`
或 `funnel_layout_with_factories_and_stage_factory`。现有构造入口保留默认输出。
先验证源数据并批量计算全部 size，再按源顺序调用颜色、格式化和阶段样式回调，
包含没有实际 body、border 或 label 的计算阶段。显式样式回调可以在对应绘制
角色关闭时执行；返回标签颜色不会启用标签。

这与 Nivo 的回调时序存在区别：Nivo 逐个 datum 计算 size 和样式，样式函数接收
的是尚未填入 profile 的临时 part；Rust 样式上下文已经拥有完整的四采样 profile。
不要用外部副作用假定两者有相同的回调交错顺序。

显式标签颜色回调的结果会保留，定制 body 几何之后不会再次执行它；未提供该
回调时，继承标签颜色仍按现有阶段工厂规则跟随替换 body 的颜色。未使用且关闭
的 `InheritedColor::Contrast` 边框保留既有准入行为。显式样式回调需要先解析默认
上下文：如果透明表面上的默认颜色无法解析，构建可能在进入回调之前失败。
提供回调不会绕过默认上下文验证。错误不返回候选布局，回调自己的外部副作用
不参与回滚。

## 原始 profile 路径与当前自定义墨迹

`funnel_part_geometry(part, interpolation)` 生成原始闭合阶段 profile；两侧
完全重合时返回 `None`。`funnel_part_border_geometry` 返回原始两条开放侧边，
即使没有启用边框绘制也能调用。两者都要求显式传入 `FunnelInterpolation`，
公开 part 不保存该选项。八个 profile 点的所有坐标必须有限且满足 Scene 预算。

这些工具对应 area/border 路径生成用途，不提供 React generator 对象。自定义
阶段可以绘制偏移 Path 或其他轮廓，其当前几何和画笔以 Scene／宿主的实际
`Mark` 为准，可以与这些工具返回的原始 profile 不同。

## 实有角色与完整共享数据

`funnel_widgets(&layout, original_source, revision, placement)` 返回
`PlotWidgetsData<FunnelWidgetData>`。同步 placement 上下文借用完整 source、
layout、可选的 datum／part／separator、实际 Mark，以及标签池、裁剪和 series
归属；这些引用不会被保留。返回 `None` 跳过该角色，四类角色合计最多选择 128 个：

| 角色 | 归属与位置 |
| --- | --- |
| `Body` | 已实际生成的阶段 body；位置为原始 `part.center`。 |
| `Border` | 实际 profile 边框 Mark；位置仍为该 part 原始中心。 |
| `BeforeSeparator` | 实际前分隔线 Mark；位置为端点中点。 |
| `AfterSeparator` | 实际后分隔线 Mark；位置为端点中点。 |

回调遵循实际 Scene 标记的绘制顺序，也遵循显式 draw order。不存在的 body 不会
生成插槽。起始宽度为零的阶段仍可能向下一非零阶段渐变而具有 body；两侧完整
重合时才没有 body。显式全部零像素宽度不会生成 body，默认全部零值域却采用
半宽 body。透明度为零或没有画笔的实际 Mark 仍有角色。没有 body 的阶段不会
进入 body 几何工厂，因此该回调不会移除其独立边框。宿主 hidden 状态管理
可见性和输入，不伪造或丢弃源数据。

收尾分隔线的 `source_index` 和 `part_index` 为 `None`，`separator_index`
仍指向实际分隔线，不会假装归属于最后一个 datum。`AtDatum` 会转换为显式矩形，
使用原 part 中心或分隔线中点，加上传入的测量偏移；自定义 body 的包围盒中心
不会取代这个锚点。

全部已选角色共享一份完整 source `Arc`、一份 parts `Arc`、一份 separators
`Arc`，以及一份 `FunnelWidgetLabels` `Arc`，在首次选中时分配。标签池包含当前
Scene 标签、按索引保存的 clip 和 series-owner 映射；应用可按 `part.series`
查找归属标签。读取标签数据不会关闭或重复绘制独立的 Scene 标签。空文本、
同阶段拥有多个标签、未生成标签是不同情况。

payload 相等性精确保留原始 value、可选 size 和数值类别键的正负零位，包含未选中
的兄弟数据和 computed parts 内的原 datum。几何仍保持普通数值相等语义。
解析后的 part 颜色可以不同于 `datum.color` 或原 paint；实际当前 Mark 画笔
通过已接受的宿主 frame 读取。

首次 placement 回调之前，会完整检查 source、profile／bounds／center、
separator、实际 Mark 角色、标签归属和私有 binding。这是当前一致性验证，
不是对最初构造器的不可变认证：公开 layout 字段和显式传入的 source 可以一起
做自洽编辑。`funnel_widget_frame(&layout)` 没有外部原 source 可供认证；它验证
当前 layout，克隆 Scene 与规范 body binding，保留 plot，并显式选择
`PaintedGeometry`。边框和分隔线不会生成规范业务控件或站点。

## 同一受控来源与真实子控件状态

下面的完整程序与[可运行示例](../examples/funnel_widgets.rs)逐字一致。它从同一
拥有所有权的受控输入派生 frame 和 packet，挂载真实 Glory Button，更新源数据，
并检查局部点击状态的保留。Memory dispatch 验证 Widget 协议，不代表操作系统输入。

```rust
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
```

运行方式为 `cargo run --example funnel_widgets`。

在子控件 Bond 和事件处理器内读取 `context.frame`，避免直接捕获尚未被宿主
接受的候选输入。相同 Mark ID／revision 保留子控件，改变 revision 请求新作用域。
严格组合将 model／packet／placement、子控件构建和绘制发布作为同一候选准入；
回调自己的外部副作用不在回滚范围内。先配置 Chart，最后调用终结式的
`.plot_widgets_composed(...)`。

`FunnelLayoutOptions.current`、当前尺寸扩展和当前边框宽度由调用者控制。
宿主选择或 hover 不会自动重建布局。应用将已接受的 body 标识映射回 datum，
更新受控 source／current 选项，重建 layout 与 packet，并一起提交。
借用的布局工厂不会被保留到后续事件中。

## 宿主顺序、替换与能力范围

示例使用位于 Scene 绘制之上的原生严格组合。`plot_widgets_with_data` 提供覆盖层，
保留源墨迹和原有的局部子控件失败策略。严格组合通过
`PlotWidgetsComposition.replace_ink` 显式列出已有测量插槽的源 Mark，替换其
物理墨迹；空集合保留源墨迹。源 Scene 和规范逻辑控件保持，已替换源墨迹不再
参与物理拾取。

浏览器的 `mount_with_plot_widgets_in_scene` 和
`set_layout_with_plot_widgets_in_scene` 按 Scene 标记位置插入 Widget；保留的
源 Mark 先绘制其墨迹，之后的 Scene 项仍可能覆盖 Widget。
`mount_with_plot_widgets_composed` 及对应 setter 使用顶层组合。同一次挂载保持
元数据类型和组合模式。准入、测量、裁剪、失败和资源预算详见
[类型化数据包](typed-plot-widgets.md)、[严格组合](widget-composition.md)与
[Scene 顺序 Widget](ordered-widgets.md)。本角色适配器没有增加任意 Nivo 式
图层列表，也不提供标签 Widget 角色。

CPU 图像 Chart 可以使用 `chart_layout(frame).raster(fonts, options)` 和同一数据包
进行严格组合，字体和 publisher 由应用提供。Scene 导出只包含 Scene 绘制，
不会嵌入实时 Widget、其状态或浏览器 DOM。这里的顺序与状态支持不代表原生
操作系统输入、React spring／animated props、任意原 datum 字段恢复、GPU 渲染
或完整 Nivo 等价性。

## 验收证据与渲染边界

定向原生测试覆盖原始位值、回调之前拒绝 source／profile／角色冲突、共享池、
零值与塌缩 profile、自定义偏移锚点、显式路径、已接受的局部状态、拒绝候选和
CPU 图像发布。浏览器用例覆盖真实 SVG／Canvas 载体、DOM 测量、可信点击与
键盘激活、源选择、hidden、受控 current、重排、resize、camera 和 revision 更新。
最终计数和保留的失败证据见[验收记录](verification.md#development-funnel-function-and-live-layer-roles)。

浏览器专用 fixture 将源标签设为 20px、Widget 内容设为 0px：Widget 绘制纯色的
测量 Button，文本／元数据和局部状态仍可通过 DOM 观察，不验收 Widget 字形栅格。
前景和保留源墨迹的采样使用逐通道 RGBA 差值不超过 1，DOM 几何差值不超过
`1/32` Scene 像素。
标签和边框的独特色彩在同一 RGBA 容差内作全局计数，同时保留精确色值计数和
实际像素／预期色／delta 见证。这只证明可见的独特源画笔，不证明每个字形轮廓
或每个阶段的边框覆盖。整视口 PNG 哈希仅作诊断，不代表逐位相同截图验收；
回滚另外严格比较已接受状态、DOM、计算后的 SVG 画笔、Canvas 后台 PNG 字节
和 capture 几何。
