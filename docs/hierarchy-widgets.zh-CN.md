# 层次图实时节点组件

此 API 自 Kartis 0.2.0 提供。`hierarchy_node_widgets` 将 Treemap、
CirclePacking、Icicle 中真实节点 Mark 接入类型化、可测量 Glory 子组件。
`hierarchy_widget_frame` 同时提供原 Scene、准确 plot 与显式实际绘制拾取。

一个包共享一份拥有所有权的原始扁平 `Vec<HierarchyDatum>`。通过 `source[source_index]`
读取原 datum；`node.value` 是计算后的子树聚合值，不是原 datum 自身权重。包还保留图族、
根/焦点与祖先、当前节点元数据、聚合格式值，以及当前实际标签和裁剪。当前几何、画笔
从接受帧 Mark 读取。标签仍由 Scene 独立拥有和绘制，元数据不会自动移除或重建标签。

构建器保留私有原源与构建时角色资格快照。首次定位回调前校验完整拓扑、聚合、索引、
焦点祖先、显隐、真实几何/Mark 与受管标签所有权。隐藏、零权重、焦点外和被配置排除的
父节点/焦点节点不造角色；透明或无画笔但有真实 Mark 的节点仍有角色。frame helper 为
这些未画墨迹的实际角色也提供类型化站点，PaintedGeometry 物理拾取仍只返回实际墨迹，
旧交互方法不变。返回 `None` 跳过组件，错误不返回部分包；回调外部副作用无法回滚。
定位与裁剪能力由既有宿主共同准入。

入口接受当前静态 `HierarchyLayout` 或 `HierarchyFocusMotion::target_layout`，不能将
目标包与另一个运动采样 Scene 混用；本 API 不自动跟随 spring 采样或退出中的标记。
焦点、数据、视图合法更新后重建包，按需保留稳定 ID 和 revision。支持先执行既有受管
标签布局/隐藏，再构建组件包；过期或损坏的标签所有权拒绝。

完整 Memory 示例对 CirclePacking 标签显式采用 plot 矩形裁剪，因为语义 Graphic 文字
支持矩形 clip；SVG/Canvas 与 raster 的圆形/路径标签裁剪能力另行保留。

```rust
//! Owned original tree and live Glory nodes across all hierarchy views.
use glory::{App, Widget};
use kartis::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let data = vec![
        HierarchyDatum::new("root", None, 0.),
        HierarchyDatum::new("a", Some("root".into()), 2.),
        HierarchyDatum::new("b", Some("root".into()), 3.),
    ];
    for view in [
        HierarchyView::Treemap(Default::default()),
        HierarchyView::CirclePacking(CirclePackingLayoutOptions {
            style: HierarchyStyleOptions {
                leaf_labels: HierarchyLabelStyle {
                    clip_to_node: false,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        }),
        HierarchyView::Icicle(Default::default()),
    ] {
        let mut controller = HierarchyFocusController::new(&data)?;
        let layout = controller.layout(&ChartConfig::default(), &view)?;
        let widgets = hierarchy_node_widgets(&layout, 0, |_| {
            Ok(Some(PlotWidgetPlacement::AtDatum {
                offset: [-20., -10.],
                measured: TextSize {
                    width: 40.,
                    height: 20.,
                },
            }))
        })?;
        let replace_ink = widgets
            .layout
            .slots
            .iter()
            .map(|s| s.mark_id.clone())
            .collect();
        let frame = hierarchy_widget_frame(&layout)?;
        let app = App::memory();
        app.mount(chart_layout(frame).plot_widgets_composed(
            PlotWidgetsComposition {
                widgets,
                replace_ink,
            },
            |ctx: PlotWidgetCompositionContext<HierarchyNodeWidgetData>| {
                let frame = ctx.frame;
                Ok(glory::ui::widget(move |scope| {
                    let clicks = scope.cage(0u32);
                    glory::ui::button()
                        .label(scope.bond(move || {
                            frame.with(|f| {
                                format!(
                                    "{}: raw {}, total {} ({} clicks)",
                                    f.data.node.label,
                                    f.data.source[f.data.source_index].value,
                                    f.data.node.value,
                                    clicks.get()
                                )
                            })
                        }))
                        .on(glory::render::EventKind::Activate, move |_| {
                            clicks.revise(|v| *v += 1)
                        })
                        .show_in(scope)
                }))
            },
        ))?;
        app.flush()?;
        app.clear()?;
    }
    Ok(())
}
```

Nivo 三家族契约不同：Treemap 的 node/animatedProps、CirclePacking 的 node/style/
handlers，以及 Icicle 的共享 Rect wrapper 不能混称同一协议。上游零面积或不可见计算
节点可能仍创建组件；本适配遵守 Kartis 真实 Mark 资格，不为它们制造代理标记。
Rust 输入为全局唯一扁平 ID、内部节点自身权重为零；Nivo 嵌套 datum/path 身份和聚合
范围不同。原权重和聚合值分别提供。冻结组件表达式核验不执行 React reconciliation、
spring 调度或完整 D3 布局。

[事务组合](widget-composition.zh-CN.md)提供共同准入、替换与回滚；浏览器可使用
[Scene 顺序组合](ordered-widgets.zh-CN.md)交错绘制。独立 Scene 导出不含任意实时 Widget
快照，原生 OS 输入及 GPU/worker 继续保留。浏览器颜色夹具使用保留 JSON 数据的实际
font0 Button，不代表任意字形的严格栅格等价。

专项验收覆盖 24 个浏览器/DPR/图族上下文、624 帧与 20,832 项硬颜色采样；
原生/父节点/标签的具体范围、保留失败与后续工作见[验收记录](verification.md#development-live-hierarchy-node-components)。
