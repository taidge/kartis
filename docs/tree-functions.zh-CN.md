# Tree 函数配置

此 API 自 Kartis 0.2.0 提供。
`tree_layout_with_factories` 通过 `TreeFactories` 接收五个同步借用的 `FnMut`。
未设置的函数沿用静态选项，返回的布局不保留函数及其捕获状态。
[English](tree-functions.md)

| 字段 | 上下文 | 返回结果 |
| --- | --- | --- |
| `node_size` | `TreeNodeFunctionContext` | Scene 像素单位的直径 |
| `node_color` | `TreeNodeFunctionContext` | 节点实际填充 `Color` |
| `link_thickness` | `TreeLinkFunctionContext` | Scene 像素单位的线宽 |
| `link_color` | `TreeLinkFunctionContext` | 连线描边 `Color` |
| `label` | `TreeLabelFunctionContext` | 绘制标签 `String` |

所有函数返回 `Result`。节点上下文含完整扁平来源、原始 datum 与索引、父子索引、
深度、高度、聚合值、投影位置和 plot。原权重及其符号位保留在 `datum.value` 中。
尺寸与颜色共享中间上下文，不伪造尚未解析的最终尺寸或颜色。连线上下文含计算连线、
两个最终计算端点、完整来源和 plot。标签上下文含原始 datum 与最终计算节点。

## 完整应用

[独立示例](../examples/tree_functions.rs) 挂载 Glory Memory 图表并重建已接受的来源。
Memory 验证宿主协议，操作系统输入需单独验收。

```rust
//! A complete Glory application with Tree's five borrowed base functions.
use glory::App;
use kartis::*;

fn source(visits: f64) -> Result<SceneInteraction> {
    let data = [
        HierarchyDatum::new("visits", Some("root".into()), visits),
        HierarchyDatum::new("root", None, 0.),
        HierarchyDatum::new("sales", Some("root".into()), 10.),
    ];
    let options = TreeOptions::default();
    // Size is a diameter; node.radius is half the accepted result.
    let mut size = |ctx: &TreeNodeFunctionContext<'_>| Ok(16. + ctx.depth as f64 * 8.);
    let mut color = |ctx: &TreeNodeFunctionContext<'_>| {
        Ok(if ctx.children.is_empty() {
            Color(30, 140, 90, 255)
        } else {
            Color(40, 100, 180, 255)
        })
    };
    let mut thickness =
        |ctx: &TreeLinkFunctionContext<'_>| Ok(if ctx.target_node.value >= 20. { 4. } else { 2. });
    let mut link_color = |ctx: &TreeLinkFunctionContext<'_>| {
        Ok(ctx.target_node.color.unwrap_or(Color(100, 100, 100, 255)))
    };
    let mut label =
        |ctx: &TreeLabelFunctionContext<'_>| Ok(format!("{}: {:.0}", ctx.datum.id, ctx.node.value));
    let layout = tree_layout_with_factories(
        &ChartConfig::default(),
        &data,
        &options,
        &mut TreeFactories {
            node_size: Some(&mut size),
            node_color: Some(&mut color),
            link_thickness: Some(&mut thickness),
            link_color: Some(&mut link_color),
            label: Some(&mut label),
        },
    )?;
    assert_eq!(layout.nodes[0].radius, 12.);
    assert_eq!(layout.nodes[1].value, visits + 10.);
    assert!(
        layout
            .scene
            .labels
            .iter()
            .any(|l| l.text == format!("visits: {visits:.0}"))
    );
    // Drawing labels do not rewrite original source or business-control labels.
    let frame = tree_widget_frame(&layout)?;
    assert_eq!(frame.interaction.as_ref().unwrap().sites[0].label, "visits");
    Ok(frame)
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let app = App::memory();
    let input = app.scope().cage(source(20.)?);
    let frame = app.scope().bond(move || input.get());
    app.mount(chart_layout(frame).on_reject(|e| eprintln!("Tree example rejected: {e:?}")))?;
    app.flush()?;
    input.revise(|old| *old = source(30.).unwrap());
    app.flush()?;
    assert!(input.with(|s| s.scene.labels.iter().any(|l| l.text == "visits: 30")));
    app.clear()?;
    Ok(())
}
```

## 计算与绘制

静态选项、原始层级、全部位置及原路径在第一个函数调用前预检。节点按层级广度优先顺序
逐个执行尺寸、颜色；来源索引仍指向原扁平输入，即使输入中的子节点先于父节点。
全部节点解析后，连线逐个执行粗细、颜色，函数能看到两个完整端点。
禁用连线仍计算函数并保留拓扑元数据，但不生成 Mark；启用而线宽为零的连线保留 Mark，
不设置描边。

标签函数在节点与连线解析后运行，仅在层级标签开关及 All/Leaves/Internal 可见性允许时
调用。返回空串仍保留真实 Label 及其归属。这只修改绘制标签，不回写原始 datum、节点
Mark 的业务标签或 canonical 控件。[Tree Widget 适配器](tree-widgets.zh-CN.md)
读取同一份已接受标签池及计算节点的颜色、半径。

尺寸函数返回直径，区别于 `TreeOptions.node_radius`；结果更新 `TreeNode.radius` 和
解析后的样式。显式颜色函数覆盖 Palette、Fixed、None 填充声明；未设置函数时保留原声明。
零尺寸和透明节点仍有真实计算角色。无效输出或函数错误不返回候选布局；函数对调用者状态
产生的副作用不回滚，宿主应在构造成功后发布。

直径必须有限且在 `0..=32768` 内；线宽必须为零，或有限且在 `0.000001..=10000` 内。
函数标签沿用 1024 字节 UTF-8 预算。

## 与 Nivo 的对应及边界

冻结参考执行原版五个函数属性、透传工具、节点/连线计算顺序及标签规则，并分别记录
active/inactive 修改器、固定深度颜色继承、SVG/Canvas 标签门禁。参考中的比例尺、
层级坐标、hooks 和 Spring 使用显式依赖探针，不作为 D3 数字结果或 React 生命周期证据。

Rust 保留根据静态节点样式计算投影 inset 的旧行为，函数尺寸不会重新投影位置。
Nivo 的投影独立于节点尺寸，因此改变静态半径会让 Rust 的 inset 产生不同结果。
函数输出仍决定实际节点几何与标签间距。Rust 为两种 renderer 统一采用可见性门禁后的
标签计算；原版 Nivo Canvas 即使关闭标签绘制层，也会先计算标签。

泛型嵌套 JavaScript 数据、路径 UID 与 Rust 的扁平全局唯一 ID、显式原权重/聚合权重
不同。active/inactive 节点尺寸及连线粗细修改器与固定深度颜色继承另由
[Tree 活动配置](tree-activity.zh-CN.md) 提供。[标签组件](tree-label-components.zh-CN.md)
和[原生 Canvas 层及完整标签池](tree-canvas.zh-CN.md)由后续专用 API 提供；旧构造器的标签门禁不变。
任意 SVG 函数层和自动退出动画仍待补。
GPU/worker 与任意实时 Widget 导出继续后置；本次函数配置不冒称
这些更广的能力或严格整页像素等价。
