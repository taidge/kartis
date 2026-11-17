# Swarm 实时节点组件

自 Kartis 0.2.0 提供。`swarm_node_widgets` 为实际打包的 Swarm 节点创建测量后的类型化 Glory 组件；`swarm_widget_frame` 提供显式按实际墨迹拾取的帧，可用于语义 Chart、CPU 图像及 SVG/Canvas 浏览器宿主。普通 Swarm 绘制与旧转换入口保持原行为。

```rust,ignore
let packet = swarm_node_widgets(&layout, &original_points, revision, |node| {
    Ok(Some(PlotWidgetPlacement::AtDatum {
        offset: [-16.0, -11.0],
        measured: TextSize { width: 32.0, height: 22.0 },
    }))
})?;
```

每个 `SwarmNodeWidgetData` 包含共享的 `Arc<Vec<SwarmPoint>>`、原始 `source_index`、打包 `site_index`、当前 `SwarmSite`/`SwarmGroupLayout`、准确 plot 和解析后的值尺度。使用 `source[source_index]` 读取当前原 datum；实际画笔/几何从宿主已接受的 `TypedPlotWidgetFrame.frame.mark` 读取。原始数值及可空 size 的相等比较保留有符号零，包括隐藏或未选中的源记录。普通业务身份和几何继续按数值比较。独立分配但内容相同的来源仍相等；选中节点共享一次 owned 输入复制。

借用定位上下文提供布局、原输入/datum、分组、打包站点与实际 Mark。全部原记录、分组、元数据、值投影、打包顺序、实际标记及锚点在第一次回调前完成预检。返回 `None` 跳过角色；错误不返回数据包。外部回调副作用由调用者负责。宿主检查定位、clip 能力及最多 128 个选中插槽；空包仍保留完整 plot。

隐藏分组、完全裁剪节点没有角色。实际零尺寸、透明、零 opacity 或无画笔 Mark 保留角色；宿主按当前实际资格决定物理输入和可见性。回调采用打包绘制顺序，可能不同于原输入顺序。自定义几何保留原碰撞圆盘与打包 datum 位置。偏移 Path 使用类型化锚点，Exact 拾取依据墨迹，不把 Path 包围盒中心猜作数据位置。

来源契约是显式的：将原始 points 与公开布局一起传入，校验它们与当前元数据一致。布局没有保存原 orientation、打包 spacing、shape、fit 或 formatter 配置；完全自洽的公开编辑仍可接受。frame helper 没有原输入参数，只检查当前结构/绑定；packet adapter 再检查全部原始记录。这不恢复原 force 模拟或原始配置。

## 完整应用

下面的可运行 Memory 应用在仅有符号零变化及源顺序改变时，保留实际 Button 的身份和点击计数：

```rust
//! Retained Glory node components with actual packed Swarm source data.
use glory::render::{Event, EventKind, NodeId, Props};
use glory::{App, Widget};
use kartis::*;
#[derive(Clone, Debug)]
struct Source {
    frame: SceneInteraction,
    composition: PlotWidgetsComposition<SwarmNodeWidgetData>,
}
fn source(positive_zero: bool, reverse_input: bool) -> Result<Source> {
    let mut b = SwarmPoint::new("b", "north", if positive_zero { 0. } else { -0. });
    b.size = Some(0.);
    let mut data = vec![SwarmPoint::new("a", "north", 2.), b];
    data[0].label = Some("Alpha".into());
    if reverse_input {
        data.reverse();
    }
    let layout = swarm_layout(
        &ChartConfig {
            width: 600.,
            height: 260.,
            ..Default::default()
        },
        &data,
        &SwarmLayoutOptions {
            orientation: SwarmOrientation::Horizontal,
            value_domain: Some([-1., 10.]),
            value_axis: None,
            group_axis: None,
            ..Default::default()
        },
    )?;
    let widgets = swarm_node_widgets(&layout, &data, 0, |_| {
        Ok(Some(PlotWidgetPlacement::AtDatum {
            offset: [-16., -11.],
            measured: TextSize {
                width: 32.,
                height: 22.,
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
        frame: swarm_widget_frame(&layout)?,
        composition: PlotWidgetsComposition {
            widgets,
            replace_ink,
        },
    })
}
fn main_node(app: &App) -> NodeId {
    let tree = app.tree();
    let mut pending = tree.roots().to_vec();
    while let Some(id) = pending.pop() {
        let n = tree.node(id).unwrap();
        if n.descriptor
            .view()
            .classes
            .iter()
            .any(|c| c == "swarm-example-main")
        {
            return id;
        }
        pending.extend(n.children.iter().copied());
    }
    panic!("main node component missing")
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let app = App::memory();
    let input = app.scope().cage(source(false, false)?);
    let frame = app.scope().bond(move || input.with(|s| s.frame.clone()));
    let packet = app
        .scope()
        .bond(move || input.with(|s| s.composition.clone()));
    app.mount(
        chart_layout(frame).plot_widgets_composed(packet, |context| {
            let frame = context.frame;
            let main = frame.with(|f| f.data.site.node_id == "a");
            Ok(glory::ui::widget(move |scope| {
                let count = scope.cage(0u32);
                glory::ui::button()
                    .class(if main {
                        "swarm-example-main"
                    } else {
                        "swarm-example-node"
                    })
                    .label(scope.bond(move || {
                        frame.with(|f| {
                            let d = &f.data.source[f.data.source_index];
                            let zero = f.data.source.iter().find(|v| v.id == "b").unwrap();
                            format!(
                                "{} · {}: {} (zero={:016x}, clicks={})",
                                f.data.group.label,
                                d.label.as_deref().unwrap_or(&d.id),
                                f.data.site.formatted_value,
                                zero.value.to_bits(),
                                count.get()
                            )
                        })
                    }))
                    .on(EventKind::Activate, move |_| count.revise(|v| *v += 1))
                    .show_in(scope)
            }))
        }),
    )?;
    let child = main_node(&app);
    app.dispatch(child, Event::Activate)?;
    app.flush()?;
    input.revise(|s| *s = source(true, false).unwrap());
    app.flush()?;
    assert_eq!(main_node(&app), child);
    {
        let tree = app.tree();
        let Props::Button(button) = tree.node(child).unwrap().descriptor.view().props else {
            panic!("not a Button")
        };
        assert!(button.label.contains("zero=0000000000000000, clicks=1"));
    }
    input.revise(|s| *s = source(false, true).unwrap());
    app.flush()?;
    assert_eq!(main_node(&app), child);
    app.clear()?;
    Ok(())
}
```

运行 `cargo run --example swarm_widgets`。覆盖层保留原墨迹；需要替换时，显式将选中 mark ID 放入 `PlotWidgetsComposition.replace_ink`，并使用 `plot_widgets_composed` 或 BrowserChart 的 Scene 顺序组合入口。替换排除对应物理来源墨迹，同时保留源数据和逻辑控件。Scene 导出包含作者图表标记和文字，不包含实时子组件。改变 revision 重建子组件；相同 revision 更新已接受数据并保留局部状态。

## 与 Nivo 的对应关系

Nivo SwarmPlot 的 SVG `circleComponent` 接收计算节点、动画 style、border width 及七种鼠标 handler；transition 以节点 ID 为键，将 radius 限为非负，也可创建零尺寸或非交互节点。Canvas 另用 `renderCircle`。Kartis 通过共享类型化宿主提供实际 Glory 组件，通过独立几何工厂提供可导出的墨迹。确定性单轴打包、String 业务 ID、有限数值/整数 UTC 和显式宿主事件，与 Nivo force 模拟、任意 raw 字段、React/Spring 调度及 JavaScript Date 保持区别。受控 hidden/cull 资格也单独说明；这些 API 不承诺自动采样动画或退出节点组件。

浏览器用例使用 font-zero 纯色 Button、独立原输入/角色 oracle、误差不超过 1/32 逻辑像素的实际 DOM 几何和每通道允许 ±1 的前景采样。拒绝候选后，已接受状态、完整根 DOM、SVG 计算绘制样式、原生 Canvas backing 字节及视口几何严格比较。已知无脚本原生 SVG 抗锯齿像素仍可变化，整视口 PNG 哈希单独保留为诊断。任意字形等价、严格 compositor 稳定、原生同层输入与 GPU/worker 保留为差距。

参见[几何工厂](swarm-custom.zh-CN.md)、[类型化数据包](typed-plot-widgets.zh-CN.md)、[原始值更新](widget-source-values.zh-CN.md)和 [English guide](swarm-widgets.md)。
