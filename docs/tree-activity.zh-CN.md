# Tree 活动状态与颜色继承

此 API 自 Kartis 0.2.0 提供。
`tree_layout_with_activity` 将[五类基础函数](tree-functions.zh-CN.md)与四个活动态修改器、
固定深度颜色继承组合。函数仅在一次构造中借用，不被返回的布局保留。
[English](tree-activity.md)

`TreeActivityOptions.focus_id` 使用原始来源 ID，`None` 清除活动状态；未知 ID 在任何函数
运行前拒绝。`highlight` 复用 `TreeRelationOptions` 的祖先/后代节点与连线四个独立开关。
当前节点自身始终 active；默认高亮祖先节点和祖先连线，与原版 Tree 策略一致。

没有当前节点时，`TreeActivityState.nodes`、`.links` 的全部条目为 `None`，四个修改器
均不调用。有当前节点后，每个条目为 `Some(true)` 或 `Some(false)`。关闭全部连线高亮
会令全部连线 inactive，不会恢复 neutral 状态。

| `TreeActivityFactories` 字段 | 输入 | 输出 |
| --- | --- | --- |
| `active_node_size` | `TreeNodeModifierContext` | active 节点直径 |
| `inactive_node_size` | `TreeNodeModifierContext` | inactive 节点直径 |
| `active_link_thickness` | `TreeLinkModifierContext` | active 连线宽度 |
| `inactive_link_thickness` | `TreeLinkModifierContext` | inactive 连线宽度 |

每个角色只调用被选中的修改器；未设置的修改器保留基础尺寸/线宽，返回常量的函数即可表达
数值修改器。节点上下文含完整基础节点、原始 datum/来源、精确保留的基础直径 `size`、
`is_active`、完整活动状态和 plot。连线上下文含基础 `thickness`、`color`、两个最终节点及
同一状态。直径必须有限且在 `0..=32768` 内；线宽为零或有限且在 `0.000001..=10000` 内。
错误不产生候选布局，函数对调用者状态的副作用不回滚。

## 计算顺序与继承

`fix_node_color_at_depth: None` 表示不限制深度，`Some(depth)` 使用非负整数阈值。
根始终解析基础颜色，深度不大于阈值的节点也解析自己的颜色；更深节点继承已解析的父色，
不调用自己的基础颜色函数。父节点的 None 填充会如实继承，不回退到子节点旧调色板颜色。
此类型化阈值区别于 JavaScript 的小数、负数、NaN 语法。

完整来源、选项及静态路径预检后，节点按广度优先顺序逐个解析基础尺寸、基础或继承颜色、
活动分类及被选中的尺寸修改器；全部最终节点随后供连线使用。连线依次解析基础粗细/颜色及
被选中的线宽修改器，即使关闭绘制仍计算。符合条件的标签最后解析，读取最终尺寸。
此构造入口的基础连线与标签上下文提供 `activity: Some(&TreeActivityState)`；普通
`tree_layout_with_factories` 则为 None。原有五字段 `TreeFactories` 及 Tree 节点/连线/布局
结构保持不变。

保留原静态样式的投影 inset。修改器改变真实几何及标签间距，不重新投影位置；绘制标签
依旧不回写原始来源或 canonical 业务控件标签。

## 完整 Memory 应用

[独立示例](../examples/tree_activity.rs) 驱动真实 Glory plot-pointer 协议，并将 hover 提案
构造成已接受的活动布局。Memory 验证宿主协议，不作为物理操作系统输入证据。

```rust
//! A Glory Memory application driven by real plot pointer move/leave events.
use std::cell::Cell;

use glory::App;
use glory::render::{Event, NodeId, PlotPointer, PlotPointerDevice, PlotPointerPhase, Rect};
use kartis::*;

fn source(focus_id: Option<String>) -> Result<TreeActivityLayout> {
    let data = [
        HierarchyDatum::new("visits", Some("group".into()), 30.),
        HierarchyDatum::new("group", Some("root".into()), 0.),
        HierarchyDatum::new("root", None, 0.),
        HierarchyDatum::new("returns", Some("root".into()), 10.),
    ];
    let activity = TreeActivityOptions {
        focus_id,
        fix_node_color_at_depth: Some(1),
        ..Default::default()
    };
    let colors = Cell::new(0);
    let modifiers = Cell::new(0);
    let mut size = |_: &TreeNodeFunctionContext<'_>| Ok(16.);
    let mut color = |ctx: &TreeNodeFunctionContext<'_>| {
        colors.set(colors.get() + 1);
        Ok(if ctx.datum.id == "root" {
            Color(40, 100, 180, 255)
        } else {
            Color(30, 140, 90, 255)
        })
    };
    let mut width = |_: &TreeLinkFunctionContext<'_>| Ok(2.);
    let mut link_color = |ctx: &TreeLinkFunctionContext<'_>| {
        Ok(ctx.source_node.color.unwrap_or(Color(100, 100, 100, 255)))
    };
    let mut label = |ctx: &TreeLabelFunctionContext<'_>| {
        Ok(format!(
            "{}: {:?}",
            ctx.datum.id,
            ctx.activity.unwrap().nodes[ctx.source_index]
        ))
    };
    let mut active_size = |ctx: &TreeNodeModifierContext<'_>| {
        modifiers.set(modifiers.get() + 1);
        Ok(ctx.size + 8.)
    };
    let mut inactive_size = |ctx: &TreeNodeModifierContext<'_>| {
        modifiers.set(modifiers.get() + 1);
        Ok(ctx.size / 2.)
    };
    let mut active_width = |ctx: &TreeLinkModifierContext<'_>| {
        modifiers.set(modifiers.get() + 1);
        Ok(ctx.thickness + 3.)
    };
    let mut inactive_width = |ctx: &TreeLinkModifierContext<'_>| {
        modifiers.set(modifiers.get() + 1);
        Ok(ctx.thickness / 2.)
    };
    let layout = tree_layout_with_activity(
        &ChartConfig::default(),
        &data,
        &TreeOptions::default(),
        &mut TreeFactories {
            node_size: Some(&mut size),
            node_color: Some(&mut color),
            link_thickness: Some(&mut width),
            link_color: Some(&mut link_color),
            label: Some(&mut label),
        },
        &activity,
        &mut TreeActivityFactories {
            active_node_size: Some(&mut active_size),
            inactive_node_size: Some(&mut inactive_size),
            active_link_thickness: Some(&mut active_width),
            inactive_link_thickness: Some(&mut inactive_width),
        },
    )?;
    assert_eq!(colors.get(), 3); // Depth-two visits inherits its parent's color.
    assert_eq!(
        modifiers.get(),
        if activity.focus_id.is_some() { 7 } else { 0 }
    );
    Ok(layout)
}

fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut pending = tree.roots().to_vec();
    let mut result = vec![];
    while let Some(id) = pending.pop() {
        let node = tree.node(id).unwrap();
        if node.descriptor.view().classes.iter().any(|c| c == class) {
            result.push(id);
        }
        pending.extend(node.children.iter().copied());
    }
    result
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let app = App::memory();
    let accepted = app.scope().cage(source(None)?);
    let frame = app
        .scope()
        .bond(move || accepted.with(|a| tree_activity_frame(a).unwrap()));
    app.mount(chart_layout(frame).highlight(false).on_hover(move |mark| {
        let focus =
            mark.and_then(|m| accepted.with(|a| a.node_id_from_mark(&m.id).map(str::to_owned)));
        if accepted.with(|a| a.activity.focus_id != focus) {
            accepted.revise(|old| *old = source(focus).unwrap());
        }
    }))?;
    let original = nodes(&app, "kartis-datum");
    let plot = nodes(&app, "kartis-plot")[0];
    let position = accepted.with(|a| a.layout.nodes[0].position);
    let config = ChartConfig::default();
    let pointer = |phase, point: [f64; 2]| Event::PlotPointer {
        from: plot,
        pointer: PlotPointer {
            pointer_id: 1,
            device: PlotPointerDevice::Mouse,
            phase,
            x: point[0],
            y: point[1],
            viewport: Rect {
                x: 0.,
                y: 0.,
                width: config.width,
                height: config.height,
            },
        },
    };
    app.dispatch(plot, pointer(PlotPointerPhase::Move, position))?;
    app.flush()?;
    assert_eq!(
        accepted.with(|a| a.activity.focus_id.clone()).as_deref(),
        Some("visits")
    );
    assert_eq!(nodes(&app, "kartis-datum"), original);
    app.dispatch(plot, pointer(PlotPointerPhase::Leave, [0., 0.]))?;
    app.flush()?;
    assert!(accepted.with(|a| a.activity.nodes.iter().all(Option::is_none)));
    app.clear()?;
    Ok(())
}
```

## 类型化控件与接受状态

`TreeActivityLayout` 含最终 `layout` 与共享 `activity`。`tree_activity_frame`、
`tree_activity_widgets` 会在暴露来源前共同预检。控件读取 `.tree` 中完整原 Tree 角色、
共享活动状态及自身 `is_active`，原始数值位保持精确。

活动状态绑定其原始 Tree 捕获构造。将公开 layout 整体替换为另一份独立构造的合法布局会被
拒绝，即使节点数量相同；在原构造内合法编辑当前 Scene 的画笔、Path、标签仍受支持。
因此，来源重排不能悄悄重新解释旧的活动索引向量。

## 浏览器所有权

`kartis::browser::BrowserTree` 拥有 BrowserChart 与 current-node 循环。
`mount_with_factory` 的 `BrowserTreeBuildContext` 回调内可临时创建基础/修改器函数；
`mount_with_factory_and_widgets` 则挂载保留的 `TreeActivityWidgetData` 控件。
`BrowserTreeWidgetOptions` 控制 revision、墨迹替换和源顺序。工厂、构建或绘制失败时，
旧 Scene、活动状态及控件数据共同保留；新 revision 重建子控件。弱 handle 不延长停止后
所有者的生命。

原生监听器只合并并预约指针工作，独立 owner RAF 再解析与构造候选，不从图表事件回调递归
flush。接受快照与成功绘制、Widget stage 采用同步切换。自身发布触发的 hover 变化不清除
当前节点，因此尺寸修改器移除墨迹也不会制造重建循环。纯事件或空闲帧不重跑工厂、不重复
绘制同一 Scene。

`BrowserTreePicking::Geometry` 使用具有 canonical 业务绑定的真实节点墨迹；无业务绑定的节点
仍可由独立 nearest 策略选择。默认
`Nearest { distance: 1_000_000. }` 独立索引捕获的逻辑中心，包含零尺寸、透明和空标签节点，
同时过滤隐藏节点并拒绝 plot 外点。距离使用逻辑视口像素，遵循已接受相机。
共享图表仍严格采用 PaintedGeometry；最近节点活动不伪造 canonical 控件、物理命中或
最近点提示。等距选择采用 Rust 的来源 ID 字典序策略，不冒称与 D3 tie 完全相同。

真实主鼠标/笔 move、leave 驱动活动；click 和键盘 Focus 只转发，不设置 current。
应用显式焦点通过 `set_activity_options` 发布。Touch 不属于本活动输入协议。
`dispatch(SurfaceInput::Move)` 仅作用于共享图表，不伪称原生最近点事件。
正向轴对齐 CSS 缩放通过实际 bounds 映射；任意旋转、倾斜、透视不在此范围。

来源、配置、选项、revision 更新均为候选请求。失败后 `desired_focus` 可以不同于已接受
布局；`refresh` 显式重试，不自动反复尝试失败。删除当前来源前先清除 current 并 flush。
公开事件订阅只接收 owner 转发，不替换内部图表监听。挂载期间保留 BrowserTree 值，
停止或 drop 时释放监听及 RAF 工作。

[标签组件](tree-label-components.zh-CN.md)和[原生 Canvas 程序](tree-canvas.zh-CN.md)
由后续专用 API 提供。自动 Spring/退出、任意 SVG 函数层及实时控件导出仍待补，GPU/worker 后置。
共享 SVG/Canvas Scene、有限受检输入及原生轮廓检查不等同于任意 React、D3、提示、OS 或
严格整图等价。

原活动验收先等待悬停稳定再点击，并另行暴露快速 move/down/up 竞态。后续
[指针生命周期修复](tree-input-lifecycle.zh-CN.md) 登记原生按压，将自动活动更新延后至
click 完成。早期四引擎 64 项对照记录了旧包 WebKit 失败，修复后相同参数通过，按下前
不等待悬停，也不重试。后续手动清理移除了这些原始历史产物；[验收记录](verification.md#tree-native-pointer-lifecycle)
区分历史执行和当前保存的验收证据。新增 API 不属于已发布的 0.1.0。
