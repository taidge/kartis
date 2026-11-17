# 力导向 Network 实时节点组件

自 Kartis 0.2.0 提供。`network_node_widgets` 为 `NetworkGlyphLayout` 的视觉节点提供测量后的类型化 Glory 组件，`network_widget_frame` 保留作者 Scene 和原 canonical 节点绑定并显式启用 PaintedGeometry。`BrowserNetwork::mount_with_node_widgets` 让真实组件跟随 force tick、源节点拖动、响应式 resize 及图替换；`mount_with_link_factory_and_node_widgets` 可同时保留可导出的连线几何工厂。

## 节点数据与定位

`NetworkNodeWidgetData` 包含共享 `Arc<Vec<ForceNodeState>>`、源索引、当前 `NetworkGlyphSite`、world→Scene 偏移、累计 ticks 和热度。这是完整的当前节点状态快照，不包含任意调用者 RawDatum 字段或全部模拟/边选项。通过 `source[source_index]` 读取标签、物理半径、world 位置、速度和固定位置。原状态浮点位参与相等比较，含隐藏/未选中兄弟节点。实际画笔/几何来自已接受宿主 Mark。节点 Mark 的兼容值保持零；物理半径单独保留，不能当作视觉 glyph 尺寸或 bounds。

借用上下文提供私有受检布局、当前 simulation/node、源索引、site、实际 Mark 和偏移。整帧与来源校验先于第一次回调。返回 `None` 跳过角色；错误不返回数据包；外部回调副作用由应用负责。选中节点共享一次 owned 输入复制，最多接受 128 个插槽。

实际空标签、透明、零视觉尺寸节点都可以有角色，但不会伪造标签、canonical 来源控件或拾取站点；原绑定资格不变。适配器按保存的真实 glyph 原点，将 `AtDatum` 解析成显式矩形，也支持 bounds 中心不等于原点的无标签 Path。每次接受模拟快照时重建 packet；相同 revision 保留子组件局部状态。Scene 导出包含作者 marks/labels，不含实时 Widget。

## 完整应用

下面的独立 Memory 示例由应用拥有 tick 循环，在实际模拟一步后保留同一个 Button 和点击计数：

```rust
//! Explicit application-owned force ticks and retained Glory node components.
use glory::render::{Event, EventKind, NodeId, Props};
use glory::{App, Widget};
use kartis::*;
#[derive(Clone, Debug)]
struct Source {
    frame: SceneInteraction,
    composition: PlotWidgetsComposition<NetworkNodeWidgetData>,
}
fn source(sim: &ForceSimulation) -> Result<Source> {
    let layout = sim.scene_with_glyphs(&ChartConfig::default(), &NetworkGlyphOptions::default())?;
    let widgets = network_node_widgets(&layout, sim, 0, |ctx| {
        Ok(
            (ctx.node.id == "a").then_some(PlotWidgetPlacement::AtDatum {
                offset: [-40., -12.],
                measured: TextSize {
                    width: 80.,
                    height: 24.,
                },
            }),
        )
    })?;
    let replace_ink = widgets
        .layout
        .slots
        .iter()
        .map(|s| s.mark_id.clone())
        .collect();
    Ok(Source {
        frame: network_widget_frame(&layout)?,
        composition: PlotWidgetsComposition {
            widgets,
            replace_ink,
        },
    })
}
fn child(app: &App) -> NodeId {
    let tree = app.tree();
    let mut pending = tree.roots().to_vec();
    while let Some(id) = pending.pop() {
        let n = tree.node(id).unwrap();
        if n.descriptor
            .view()
            .classes
            .iter()
            .any(|c| c == "network-example-child")
        {
            return id;
        }
        pending.extend(n.children.iter().copied());
    }
    panic!("node component missing")
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut simulation = ForceSimulation::new(
        &[NetworkNode::new("a"), NetworkNode::new("b")],
        &[NetworkLink::new("a", "b", 2.)],
        ForceOptions {
            labels: false,
            ..Default::default()
        },
    )?;
    let app = App::memory();
    let input = app.scope().cage(source(&simulation)?);
    let frame = app.scope().bond(move || input.with(|s| s.frame.clone()));
    let packet = app
        .scope()
        .bond(move || input.with(|s| s.composition.clone()));
    app.mount(chart_layout(frame).plot_widgets_composed(packet, |ctx| {
        let frame = ctx.frame;
        Ok(glory::ui::widget(move |scope| {
            let count = scope.cage(0u32);
            glory::ui::button()
                .class("network-example-child")
                .label(scope.bond(move || {
                    frame.with(|f| {
                        let node = &f.data.source[f.data.source_index];
                        format!(
                            "{}: tick={}, x={:.3}, clicks={}",
                            node.label,
                            f.data.ticks,
                            node.position[0],
                            count.get()
                        )
                    })
                }))
                .on(EventKind::Activate, move |_| count.revise(|n| *n += 1))
                .show_in(scope)
        }))
    }))?;
    let node = child(&app);
    app.dispatch(node, Event::Activate)?;
    app.flush()?;
    simulation.step(1)?;
    let next = source(&simulation)?;
    input.revise(|old| *old = next);
    app.flush()?;
    assert_eq!(child(&app), node);
    {
        let tree = app.tree();
        let Props::Button(button) = tree.node(node).unwrap().descriptor.view().props else {
            panic!("not a Button")
        };
        assert!(button.label.contains("tick=1"));
        assert!(button.label.contains("clicks=1"));
    }
    app.clear()?;
    Ok(())
}
```

运行 `cargo run --example network_widgets`。

## BrowserNetwork 接线

`BrowserNetworkNodeWidgetOptions` 包含 `revision`、`replace_ink`、`source_ordered`，默认 `0/true/true` 在原 Scene 顺序替换选中节点墨迹。`source_ordered: false` 使用严格顶部组件层；`replace_ink: false` 保留原节点墨迹。两种模式都严格预构建候选，几何、来源元数据或子组件构建失败时整候选拒绝。

使用保留的借用定位闭包及常规 `PlotWidgetCompositionContext<NetworkNodeWidgetData>` 工厂。通过 `set_node_widget_revision` 重建子组件；普通 tick、拖动、resize 和数据替换更新现有组件。待处理 revision/热度请求与对应 force/Scene/Widget 一起接受。`reheat(alpha)` 按 `max(current, alpha)` 升温，不能用于降温；连续排队请求保留最大值，最终热度应用于最新排队的图候选。

布局尺寸来自未变换的 CSS 内容尺寸，指针映射使用实际正向轴对齐 client bounds。纯 CSS 缩放改变屏幕映射，保留逻辑 world 单位；真正内容尺寸改变则共同重建节点与插槽。外部 CSS 宽度本身不可回滚；resize 失败时，上一逻辑 Scene、节点数据及响应式组件帧保持一致，等待重试。

Widget 拥有自身原生激活/焦点，不合成来源选择或 force 拖动。未替换的可见节点继续按真实轮廓拖动；替换墨迹在拾取前排除。隐藏节点 series 会隐藏对应节点/组件，但不会自动移除装饰连线；连线是独立几何/组件角色。

定位/工厂构建时读取上一已接受 owner 状态，并接收 tentative 构建上下文。共享 Widget stage 真正采用后，保留组件 Bond 内读取到相匹配的 owner force 快照；预绘制失败不暴露候选。回调可读取状态或停止 owner；定位中停止后，不再调用后续用户定位回调。布局期间嵌套替换图会拒绝。停止释放 RAF、监听、capture 和活动组件 scope；外部回调副作用无法回滚。

Nivo0.99 `nodeComponent` 接收计算节点、animated style 和四个 handler；默认 SVG 半径为视觉 size/2，使用 translate/scale transition，Canvas renderer 独立。Rust 保留类型化有限状态、String 身份、显式宿主输入和物理/视觉尺寸区别。原组件/style/handler 配方单独冻结核验，不冒称任意 raw 字段、React/Spring 调度或 D3 force 轨迹逐值相同。

浏览器用例显式使用 6px 装饰连线，提供实色内部取样；此前 2px profile 的颜色失败仍保留，不声称已修复。拖动故障注入与已接受状态基线在同一个同步边界读取，避免合法 RAF tick 插入造成旧基线。拖动预期按实际可信 pointerdown/pointerup、稳定的实测 client bounds 及保留的抓取偏移计算，自动化请求坐标单独记录。输入投递限定在请求的一个 client 像素内，不替代独立的 1/32 world/DOM 几何门禁。另检查空闲来源 hover 的 ticks、revision 保持不变且没有待处理 RAF。

浏览器用例使用 font-zero 纯色 Button、实际 DOM 几何、独立初始 force 输入、严格已接受状态/DOM/SVG 样式/Canvas backing 回滚及原浮点位检查；整视口 PNG 哈希单独诊断。任意字形等价、历史严格 compositor 失败、实时 Widget 导出、原生同层输入、自动退出节点动画及 GPU/worker 仍属未完成范围。

参见[视觉节点几何](../src/network_glyphs.rs)、[连线工厂](network-link-custom.zh-CN.md)、[类型化数据包](typed-plot-widgets.zh-CN.md)和 [English guide](network-widgets.md)。
