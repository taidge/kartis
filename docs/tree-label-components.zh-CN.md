# Tree 标签组件（Unreleased）

[English](tree-label-components.md)

开发版增加真正的 Glory 标签 Widget，与 Tree 节点、连线组件共用事务。该接口尚未包含
在已发布的 0.1.0 中，原有 Tree 绘制默认值及旧版双角色 `TreeWidgetData` 保持兼容。

`tree_components` 按实际 Scene 绘制顺序返回一个
`SceneWidgetsData<TreeComponentData>`，包含 `Node`、`Link`、`Label`。
放置函数返回 `Ok(None)` 时，该角色继续使用普通 Scene 绘制。
`tree_label_widgets` 只选择标签，返回强类型 `TreeLabelWidgetData`。
两者接收 revision 和借用的放置函数，三类组件合计最多 128 个槽位。

标签直接引用真实 Scene Label，不使用伪造节点。稳定 key 为
`tree-label:<原始 source ID>`，与当前标签索引无关。上下文包含原始完整输入、对应原始
数据、计算节点、当前 Label、解析字体、空白策略、标签裁剪及绘图区；拥有型数据还包含
完整节点、连线和标签池。空文字、透明文字、节点尺寸为零仍保留真实标签角色。
未归属节点的注释不纳入 Tree 标签组件，同一源节点的多个受管标签会在任何放置回调前
拒绝。构建组件包不重新调用基础样式函数或活动修改器。

下面仅选择标签，以真实标签锚点为原点放置调用者已测量的内容：

```rust
use kartis::*;

fn label_components(layout: &TreeLayout) -> Result<SceneWidgetsData<TreeLabelWidgetData>> {
    tree_label_widgets(layout, 0, |_label| {
        Ok(Some(PlotWidgetPlacement::AtDatum {
            offset: [-24., -9.],
            measured: TextSize { width: 48., height: 18. },
        }))
    })
}
```

将其装入 `SceneWidgetsComposition`，`replace_ink` 使用**组件槽位 key**。
替换标签只移除该 Label 的绘制文字，保留节点和连线几何、业务数值、原始锚点及物理命中。
组件仍读取原始标签元数据。叠加模式把组件放在普通绘制表面之上；源顺序模式在实际绘制
项所在位置插入组件，可在其他绘制项之间仅替换标签。

`Chart::scene_widgets_composed` 在语义和原生宿主挂载轴对齐组件。
BrowserChart 提供 `mount_with_scene_widgets_composed`、
`mount_with_scene_widgets_in_scene` 及 Handle 上对应的更新方法。
工厂接收 `SceneWidgetCompositionContext<T>`，其中保留的 Bond 发布
`TypedSceneWidgetFrame<T>`；frame 为 `Mark` 或 `Label`，工厂可返回任意 Glory Widget。
稳定 key/revision 保留组件本地状态，已接受数据通过 Bond 更新。
同 key/revision 在 Mark 与 Label frame 之间切换会被拒绝，需新 revision；
节点和连线都属于 Mark frame。
节点、连线、标签共用 prepare/commit：采用前发生的工厂、构建、放置或绘制错误均保留
原已接受帧和组件数据。

BrowserTree 的 `mount_with_components` 和 `mount_with_factory_and_components`
将三类角色接入现有所有者及活动事务。放置函数接收 `TreeActivityComponentContext`，
组件工厂接收 `SceneWidgetCompositionContext<TreeActivityComponentData>`。
标签活动状态使用对应源节点的活动状态。旧 `mount_with_widgets` 入口仍可使用。

测量值描述组件的局部盒。按钮需要填满该盒时，将根设为 `Display::Block`，
宽高设为 `Length::Percent(100.)`。inline 按钮参与文字行的基线排版，即使容器位置
正确，按钮本身也可能在其中产生基线偏移。

源标签 carrier 的自动仿射定位要求已解析的 CSS 尺寸。`display:none` 下，`100%`
等百分比可能保持未解析并拒绝联合源候选；已接受的源／组件数据继续保留。恢复可见性
后显式 refresh owner。该要求不同于 `None` 或 unmanaged 的 [Tree 提示](tree-tooltip.zh-CN.md)，
后者本身不要求展示盒。持续源错误不能借 tooltip-only 更新绕过联合事务。

标签 `AtDatum` 围绕真实标签锚点，旋转测量盒和局部偏移，再应用当前相机。
浏览器通过真实 CSS 仿射变换承载整个子树，分别处理表面的横向、纵向 CSS 缩放；
Canvas DPR 不参与此比例。`Rect` 表示显式轴对齐盒，不继承标签旋转。
现有原生普通 Widget 宿主不能旋转子树，因此自动变换标签在该宿主会明确拒绝，
不能把 AABB 或文字回调视为完成了旋转组件。

持续启用指针时，在子控件的 Glory Style 中声明 `PointerEvents::Auto`，或使用应用样式表。
直接修改 Widget 受管理的 inline style，可能在下一次响应式 Style 更新时被替换。

标签容器默认 `pointer-events:none`，对应核对的 Nivo SVG Labels 容器。
这不等于 `inert`：完整可见的自定义内容仍可通过键盘聚焦，子组件也可显式启用指针事件。
部分裁剪内容维持保守的被动策略。Kartis 不给自定义标签自动附加节点数据事件；
调用者增加行为时须读取实际 frame 可见性及元数据。

Nivo SVG `labelComponent` 接收计算标签和动画位置、旋转；Kartis 提供已接受静态及实时帧，
尚未实现 React/Spring 退出动画。Nivo Canvas 的 `renderLabel(ctx, {label, theme})`
仍是不同扩展点，由后续[原生 Canvas 程序](tree-canvas.zh-CN.md)提供：Canvas 上方的 Glory DOM 组件不等于原始 Canvas 回调，也不会自动导出
为 SVG 或 raster 控件。现有 Rust 标签继续使用 alphabetic 基线和投影内边距，
不声称与 Nivo 的所有基线行为一致。

相关契约和检查记录见 [Tree 节点连线组件](tree-widgets.zh-CN.md)、
[Tree 活动状态](tree-activity.zh-CN.md)及[验收记录](verification.md)。

所核对的上游契约见 [Labels.tsx](https://github.com/plouc/nivo/blob/0dae2c32052a573f9f1d66ec1b453b572119b265/packages/tree/src/Labels.tsx) 及 [TreeCanvas.tsx](https://github.com/plouc/nivo/blob/0dae2c32052a573f9f1d66ec1b453b572119b265/packages/tree/src/TreeCanvas.tsx).
