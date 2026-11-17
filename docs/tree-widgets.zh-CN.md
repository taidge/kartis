# 实时 Tree 节点与连线 Widget（Kartis 0.2.0）

[English](tree-widgets.md)

此 API 自 Kartis 0.2.0 提供。

Nivo Tree 确实提供 SVG `nodeComponent`、`linkComponent` 属性，以及 Canvas
`renderNode`、`renderLink` 函数。Kartis 为这些定制场景提供类型化 Tree 布局和
真实 Glory 节点／连线 Widget。Rust API 不执行 React 组件或 JavaScript Canvas
绘制函数。后续可选的[标签组件 API](tree-label-components.zh-CN.md)增加真实 Label
目标及三角色事务。Canvas 原始 `renderLabel` 回调和任意控件导出仍是独立缺口。

## 捕获布局与原始来源

`tree_layout(config, data, options)` 返回 `TreeLayout`，公开 `scene`、`plot`、
`nodes`、`links`。`source()`、`options()`、`root_index()` 分别读取捕获的扁平
输入、构造选项和根节点源索引。nodes 保持原输入顺序，links 保持原广度优先边
顺序。`tree_with_options` 复用同一权威计算并返回 Scene，保留已有默认 Scene／SVG
输出。

`TreeNode` 包含源／父／子索引、深度和高度、原值、聚合 `value`、原始位置、
解析后的半径、构造时样式和填充颜色，以及可选的实际 Mark ID。`TreeLink`
包含源／目标节点的源索引、原端点、真实端点中点、连线索引和可选 Mark ID。
`style`、`color` 属于构造元数据；后续自定义几何和画笔应读取已接受宿主 frame
中的实际 Mark。

Rust 输入是具有全局唯一字符串 ID 的扁平 `HierarchyDatum` 数组，要求唯一根，
并检查父关系和环。内部节点原值必须为零，`node.value` 是子树聚合值，也是兼容
Scene 标量的来源。所核对的 Nivo Tree hooks 使用嵌套输入和由路径点连得到的
UID，保留原始 `node.data`，没有调用 `hierarchy.sum`。Rust 聚合模型是独立的
既有合同，不能当作还原的 Nivo 原 datum。节点 Mark ID 保持 `node:<id>`，
连线 ID 保持 `link:<target-id>`；目标重新挂接父节点时可以保留这个连线 key，
这与 Nivo 路径 UID 身份不同。

Tidy 复用既有 Buchheim 计算，Cluster 复用既有 Rust cluster 约定，不承诺与所有
Nivo Tree／dendogram 布局逐数值相同。四种笛卡尔方向之外，第五种 radial 方向
是 Rust 扩展。共享字段 `TreeOptions.hierarchy.include_root` 对 Tree 不起作用：
Tree 始终生成根节点，保留原有行为。

`TreeOptions.node_radius` 和 `TreeNodeStyle.radius` 现在允许恰好为零。
样式半径原有的正值排除区间 `(0, 1e-6)` 保持，这一限制针对 style override。
零半径保留真实零尺寸 Circle／Rect／Path 角色，不创造绘制足迹；边框可以独立
贡献范围和墨迹。

## 一包承载两类角色

`tree_node_widgets`、`tree_link_widgets`、`tree_widgets` 接收 layout、revision
和借用的同步 placement 回调。混合回调使用 `TreeWidgetContext::Node`／`::Link`，
混合数据为 `TreeWidgetData::Node`／`::Link`。节点上下文借用原 datum、计算节点
和实际 Mark；连线上下文借用两端原 datum、计算连线和实际 Mark。两者都提供
layout 和完整捕获 source。返回 `None` 跳过该角色。引用及回调不会被保留，
回调自己的外部副作用不能回滚。
Glory 工厂不会自动获得 Nivo 的事件／tooltip props、`setCurrentNode`、
`linkGenerator` 或 Spring `animatedProps`；应用使用自己的 Glory 子控件事件
和已接受的 frame 数据。

所有已选角色通过 `Arc` 共享完整 source、nodes、links 和当前标签池。source
在构造时捕获一次，计算／标签数据包池在首次选择时分配一次，避免逐插槽复制。
`TreeWidgetLabels` 保留当前标签、按索引保存的裁剪和 series-owner 映射，
包括移动或测量后的标签。两类角色合计最多选择 128 个插槽，不是各自 128 个。

只有实际 Mark 才产生角色。关闭连线后仍保留完整计算元数据，但 `mark_id: None`，
不会产生连线回调或 Widget。真实零半径、透明度为零和没有填充／描边的节点仍
有角色；宿主的裁剪、hidden、透明度和能力决定实际绘制与物理输入。宿主 hidden
保留底层源数据和子控件状态。

`AtDatum` 转换为显式矩形，使用原始节点中心或真实端点中点，再加测量偏移，
不使用曲线或自定义 Path 的包围盒中心。需要可交互控件时应让测量插槽完整位于
plot 内：部分被裁剪的插槽可能被设为 inert。示例将叶节点控件向内放置，避免
跨越 plot 边界。

任何 placement 回调之前，先完整核对私有捕获的 source／拓扑／位置／构造资格、
当前计算元数据、全部受管理实际 Mark 和标签归属。修改受管理计算字段来表达
新拓扑会被拒绝，应重新构造 layout。合法的当前 Scene 画笔、偏移 Path 几何，
以及自洽更新的标签文本／位置／裁剪仍然支持。Circle／Rect 保持原始逻辑中心，
偏移轮廓使用 Path。额外装饰不会变成节点／连线源角色。

`tree_widget_frame(&layout)` 克隆当前 Scene，保留准确 plot，显式使用
`PaintedGeometry` binding。规范业务站点仅属于有标签的节点。连线仍是装饰性
物理角色，空标签节点不会被虚构业务标签或控件。无墨迹但有标签的节点可以保留
逻辑键盘行，同时不产生物理绘制足迹。当前 Path 锚点解析到捕获的原始节点中心。

## 原始位值与兼容标量文本

拥有所有权的源值和 `node.raw_value` 保留精确数值位，包括未选中的兄弟数据。
因此，`-0` 改为 `+0`，即使布局几何和 Scene 相等，也会刷新保留子控件的类型化
数据。这不改变 `Scene`／`Mark` 的普通数值相等语义，也不将它们的标量变成
原始源数据快照。

例如，源 `b` 的精确类型化值更新为 `+0` 后，保留的规范业务行仍可能显示标量
文本 `-0`。浏览器 fixture 对这个数值零兼容标量仅允许严格集合 `"0"`／`"-0"`，
另行精确核对原始 source bits。非零值必须精确更新：fixture 还把叶 `c` 从 30
改为 35，把根／`a` 的聚合值从 40 改为 45。不接受陈旧的非零标量或任意替代
文本。不要从规范控件文本或 `Mark.value` 推断原值位表示。

## 完整受控应用

以下程序与[可运行示例](../examples/tree_widgets.rs)逐字一致。它从同一拥有
所有权的受控 source 派生 Scene／binding 与混合组合包，创建真实 Button，
更新非零叶值并检查局部点击状态保留。Memory dispatch 验证 Widget 协议，
不代表操作系统输入。

```rust
//! A complete Tree application with retained node and link components.
use glory::render::{Event, EventKind, NodeId, Props};
use glory::{App, Widget};
use kartis::*;
#[derive(Clone, Debug)]
struct Source {
    frame: SceneInteraction,
    composition: PlotWidgetsComposition<TreeWidgetData>,
}
fn source(data: &[HierarchyDatum]) -> Result<Source> {
    let layout = tree_layout(&ChartConfig::default(), data, &TreeOptions::default())?;
    let widgets = tree_widgets(&layout, 0, |ctx| {
        let offset = match ctx {
            TreeWidgetContext::Node(ctx) if ctx.datum.id == "visits" => [-84., -12.],
            TreeWidgetContext::Link(ctx) if ctx.target_datum.id == "sales" => [-40., -12.],
            _ => return Ok(None),
        };
        Ok(Some(PlotWidgetPlacement::AtDatum {
            offset,
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
        .map(|s| s.mark_id.clone())
        .collect();
    Ok(Source {
        frame: tree_widget_frame(&layout)?,
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
            .any(|c| c == "tree-demo")
        {
            result.push(id);
        }
        pending.extend(node.children.iter().copied());
    }
    result
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut data = vec![
        HierarchyDatum::new("root", None, 0.),
        HierarchyDatum::new("visits", Some("root".into()), 80.),
        HierarchyDatum::new("sales", Some("root".into()), 40.),
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
                .class("tree-demo")
                .label(scope.bond(move || {
                    frame.with(|f| match &f.data {
                        TreeWidgetData::Node(d) => format!(
                            "Node {}: raw={}, subtree={}, clicks={}",
                            d.source[d.source_index].label,
                            d.node.raw_value,
                            d.node.value,
                            count.get()
                        ),
                        TreeWidgetData::Link(d) => format!(
                            "Link {} -> {}, clicks={}",
                            d.source[d.link.source_index].label,
                            d.source[d.link.target_index].label,
                            count.get()
                        ),
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
    data[1].value = 60.;
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

运行方式为 `cargo run --example tree_widgets`。

在子控件 Bond 和事件处理器内读取已接受的 `context.frame`。直接捕获候选 source
会绕过拒绝候选的语义。相同 ID／revision 保留子控件，显式新 revision 请求
新作用域。source／frame／packet 应一起更新。选择或 hover 不会自动重建 Tree
数据，也不会自动复现 Nivo 的 active-size modifier；受控更新由应用管理。

## 组合、宿主与导出

示例终结式的 `Chart::plot_widgets_composed` 使用位于 Scene 绘制上方的严格组合。
先配置回调、业务控件、字体和 raster publisher，再调用该方法。普通
`plot_widgets_with_data` 是覆盖层，保留既有局部子控件 revision 失败策略。
严格组合一起暂存 source、data 和子控件构建，仅在绘制发布成功后提交。
非法 model／packet／placement、工厂／构建错误、CPU PNG publisher 拒绝都会
保留先前接受的 frame 和子控件。用户回调的外部副作用不参与事务。

`replace_ink` 显式选择已有测量插槽的源 Mark 进行替换，空集合保留墨迹，
不会移除独立标签或规范节点数据。浏览器 `mount_with_plot_widgets_in_scene`
及对应 setter 使用实际 Scene 顺序，默认先 links 再 nodes；后续绘制可覆盖
Widget。`mount_with_plot_widgets_composed` 使用顶层。两者都不表示提供任意
Nivo 图层列表 API。裁剪、资源限制、状态保留和宿主输入范围详见
[类型化包](typed-plot-widgets.md)、[严格组合](widget-composition.md)和
[Scene 顺序 Widget](ordered-widgets.md)。

CPU 图像 Chart 使用 `chart_layout(frame).raster(fonts, options)` 和相同数据包，
字体及 PNG publisher 由应用提供。Scene 导出保留 Scene 绘制，不包含实时
Widget、局部状态或浏览器 DOM。本功能不承诺 React／Spring 调度、Canvas 函数
等价、任意嵌套原数据恢复、操作系统输入等价或 GPU 后端。

## 证据范围与绘制边界

定向原生检查覆盖回调之前的完整 source／拓扑冲突、共享池、零尺寸／透明／
无墨迹角色、关闭连线、偏移 Path 锚点、测量标签、仅原始位值更新、节点／连线
状态保留、显式 revision、严格拒绝和两种 DPR 下的 CPU PNG 发布。旧 Scene／SVG
输出逐字节对照。浏览器用例覆盖可信点击与键盘输入、真实 DOM 身份／count／focus
保留、源选择、全部方向、模式／曲线变化、hidden、resize、camera、仅零值和
非零源更新。这里描述验收范围，不填写最终矩阵通过计数。

浏览器 fixture 将源标签设为 20px、Widget 内容设为 0px，用于绘制纯色测量控件，
同时保留 DOM 文本／元数据和局部状态，不验收 Widget 字形栅格。前景与声明的
源内部采样逐通道 RGBA 差值不超过 1，实际 DOM 边界差值不超过 `1/32` Scene
像素。独特的源标签／连线颜色使用同一 RGBA 容差做全局计数，另保留精确色值
计数和像素见证；它只证明全局可见源画笔，不证明每个字形轮廓或每条连线覆盖。
整视口 PNG 哈希仅作诊断，拒绝候选另行比较已接受 model／data／focus、完整
DOM、计算后的 SVG 画笔、Canvas 后台 PNG 字节和 capture 几何，不宣称整视口
截图逐位相同的严格验收。

