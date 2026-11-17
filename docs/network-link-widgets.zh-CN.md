# Network 实时连线与节点组件

Unreleased。`network_link_widgets` 为 `NetworkLinkLayout` 的真实连线附着类型化 Glory
组件；`network_widgets` 将连线与节点放入一个数据包，按原 Scene 的先连线、后节点顺序，
共享 128 个插槽总预算。`network_link_widget_frame` 精确复制作者 Scene，沿用原节点 binding
并显式使用 PaintedGeometry；连线仍是装饰来源，不增加假的节点控件、canonical 连线站点
或 force 拖动目标。

## 当前数据与身份

`NetworkLinkWidgetData.source` 共享完整当前 force 节点状态，`edges` 共享完整当前边状态。
`source_index` 指向边；边的 source/target 索引指向节点。数据还包含受检的
`NetworkLinkMetadata`、world 偏移、ticks 和 alpha。原数值保留浮点位，包括有符号零
及未选中兄弟数据。这是当前 force 状态，不是任意应用 RawDatum 或所有模拟选项。
宿主 frame 中的 Mark 保留实际几何、描边颜色/宽度、透明度和值。

借用的定位上下文提供真实端点状态、源索引、Mark 和原端点位置。首个回调前先验证全量
layout/节点/边。`AtDatum` 按原端点中点解析为显式 Rect，自连接、透明几何以及偏移 Path
也沿用该真实原点，不猜包围盒中心；返回 None 跳过角色。自定义 Rect 必须以其原点居中，
偏移几何使用 Path。

`NetworkWidgetData` 与 `NetworkWidgetContext` 均有 Node/Link 分支。选中角色共享一份
节点快照，选中连线另共享一份边快照；旧节点专用入口仍使用原 `NetworkNodeWidgetData`。

连线 Mark 使用 `force-link:index`，因此平行边与自连接各有独立身份。重排后继续保留的是
索引本地状态，即使该索引已放入另一条业务边。业务换边/重排时可请求新 revision 重建，
以丢弃旧本地状态，不会自动推断业务身份。Nivo 的 source.id.target.id 在平行边上会碰撞，
Kartis 不照搬这一 key 规则。

## 完整应用

下例由应用管理 tick，同时显示两种角色，并验证真实 Button 和点击计数经过实际模拟步后保留：

```rust
//! Application-owned force ticks with retained node and link components.
use glory::render::{Event, EventKind, NodeId, Props};
use glory::{App, Widget};
use kartis::*;
#[derive(Clone, Debug)]
struct Source {
    frame: SceneInteraction,
    composition: PlotWidgetsComposition<NetworkWidgetData>,
}
fn source(sim: &ForceSimulation) -> Result<Source> {
    let layout = network_glyph_layout_with_link_factory(
        &ChartConfig::default(),
        sim,
        &NetworkGlyphOptions::default(),
        &mut |ctx| Ok(ctx.inherit(ctx.geometry.clone())),
    )?;
    let widgets = network_widgets(&layout, sim, 0, |ctx| {
        let selected = match ctx {
            NetworkWidgetContext::Link(_) => true,
            NetworkWidgetContext::Node(node) => node.node.id == "a",
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
        frame: network_link_widget_frame(&layout)?,
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
            .any(|c| c == "network-mixed-child")
        {
            result.push(id);
        }
        pending.extend(node.children.iter().copied());
    }
    result
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
                .class("network-mixed-child")
                .label(scope.bond(move || {
                    frame.with(|f| {
                        let (role, ticks, position) = match &f.data {
                            NetworkWidgetData::Link(link) => {
                                ("link", link.ticks, link.site.position)
                            }
                            NetworkWidgetData::Node(node) => {
                                ("node", node.ticks, node.site.position)
                            }
                        };
                        format!(
                            "{role}: tick={ticks}, x={:.3}, clicks={}",
                            position[0],
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
    simulation.step(1)?;
    let next = source(&simulation)?;
    input.revise(|old| *old = next);
    app.flush()?;
    assert_eq!(children(&app), before);
    for id in before {
        let tree = app.tree();
        let Props::Button(button) = tree.node(id).unwrap().descriptor.view().props else {
            panic!("not a Button")
        };
        assert!(button.label.contains("tick=1"));
        assert!(button.label.contains("clicks=1"));
    }
    app.clear()?;
    Ok(())
}
```

运行 `cargo run --example network_link_widgets`。

## BrowserNetwork 接线

`BrowserNetwork::mount_with_widgets` 使用原连线几何；
`mount_with_link_factory_and_widgets` 将可导出的自定义连线墨迹与实时组件组合。
两个入口接收 `BrowserNetworkWidgetOptions`，默认 revision=0、replace_ink=true、
source_ordered=true，以及保留的借用 `NetworkWidgetContext` 定位闭包和
`PlotWidgetCompositionContext<NetworkWidgetData>` 工厂。对未选中角色返回 None；
跳过全部节点即可仅挂载连线。`set_widget_revision` 请求两类组件共同重建。

实际 force tick、来源节点拖动、CSS 逻辑 resize、图替换、热度和 revision 与对应 Scene、
两类组件共同提交。定位、工厂/构建或发布失败保留已接受帧；采用后的保留 Bond 读取匹配的
owner 快照。首个定位回调中停止后，不再调用后续角色；外部回调副作用不可回滚。正向轴对齐
CSS scale 只改变 client 映射，保留逻辑 world 单位；外部 CSS 本身不回滚。

来源顺序可见：后绘制的未替换连线或节点会盖住先前连线组件，并在覆盖位置阻挡原生激活。
source_ordered=false 改用严格顶层组件；replace_ink=false 保留原墨迹。
组件拥有自身激活与焦点，不合成来源选择或拖动；隐藏节点不会自动隐藏独立的装饰连线组件。
宿主可见性仍遵循 Mark opacity、裁剪与 series 隐藏；零尺寸或透明几何可承载测量组件。

浏览器夹具显式使用偏移实色 Path、未选中平行边的居中红色矩形及 14px 方形 B 节点。
独立绘制预期按完整原 Scene 顺序与选中替换逐项叠放，同时核验来源遮挡、组件露出像素及
覆盖/露出位置的可信激活。该有界实色 profile 不代表任意 compositor/字形等价；整视口
PNG 哈希仍单独诊断。

冻结的 Nivo0.99 参考核验真实组件 props、transition 配方、默认 SVG 线属性与 Canvas 命令。
SVG 收到 link、animated 端点/颜色/透明度及 blendMode，没有节点的四个指针 handler，
也没有配置 leave transition；独立 Canvas 函数不设置 cap、alpha、blend。
不据此声称 React/Spring 调度、D3 force 轨迹逐值相同、运行时 blend 等价、任意原 RawDatum、
Widget 导出或真实 OS 输入。自动退出组件及 GPU/worker 仍待补。

参见[节点组件](network-widgets.zh-CN.md)、[可导出连线几何](network-link-custom.zh-CN.md)
和 [English guide](network-link-widgets.md)。
