# Bullet 部件的实时 Widget

[English](bullet-widgets.md) · [类型化插槽协议](typed-plot-widgets.zh-CN.md)

此开发版 API 尚未包含在已发布的 0.1.0 包中。
`bullet_part_widgets(&layout, revision, placement)` 为实际绘制的 range、
measure、marker 建立测量插槽和拥有所有权的元数据，返回值可交给
`Chart::plot_widgets_with_data`、`BrowserChart::mount_with_plot_widgets_data`
或 `BrowserHandle::set_layout_with_plot_widgets_data`。

测量回调借用当前 row、part 和实际 mark。返回 `Some(PlotWidgetPlacement)`
建立覆盖层，返回 `None` 省略该部件。应用需明确测量子控件的行内内容；marker
路径可能宽度为零，仅用墨迹边界不能得到可用的控件矩形。坐标使用 Scene 像素，
宿主统一应用相机、响应式 CSS 尺寸、裁剪与输入资格，并独立处理 Canvas／位图 DPR。

已接受的 `TypedPlotWidgetFrame<BulletPartWidgetData>` 同时包含
`PlotWidgetFrame` 与当前 part、可见行索引、同类排名、行数值域和轨道。
part 保留原始源索引、原始值、原始区间、可见区间与几何。画笔、可见性和输出矩形
从 `frame` 读取。按 `data.part.kind` 分派三个角色的组件；仅元数据变化时，
同一子组件继续挂载。稳定 mark ID 与 revision 保留内部状态，明确变更 revision
才请求新子组件。

同类排名包含没有实际 mark 的零值及被裁掉部件；这些部件不调用测量回调。
自动补足到域上界的 range 使用 `source_index: None`。常量域也受支持，包括仅有
marker 的 `[5, 5]`。行重排只更新 `row_index`，保留源身份。入口不会从格式化标签
解析元数据、逆推数值尺度或生成虚假数据 mark。

Scene 与插槽包应来自同一份当前布局。Chart 的响应式用法应像类型化插槽指南一样，
从共同来源派生两个输入；浏览器通过 `set_layout_with_plot_widgets_data` 共同提交。
元数据键不匹配、与挂载工厂类型不同、非法矩形、不支持的裁剪或非法 Scene，均保留
上一份已接受帧和数据。测量回调失败时不返回部分包，也不修改布局；应用回调对外的
副作用由应用负责。子组件工厂／构建失败则遵循既有的局部修订协议：保留旧子组件，
旧组件读取新接受帧与元数据；不会因此回滚整份已接受 Scene。

这些是真实 Glory 控件，位于原始 Bullet 墨迹上方。它们不替换部件绘制，Scene 的
SVG／Canvas／PNG 导出也不包含控件内容。Nivo 0.99 Bullet 提供 SVG React 组件，
传入计算后的矩形或 marker 数据及 spring `animatedProps`，没有 Bullet Canvas
组件。Kartis 提供自己的稳定源 ID、排名、测量和 Widget 生命周期；不将此入口
描述为 React spring、任意 React 内容或原图层替换语义的完整等价实现。

`bullet_part_widgets` API 文档包含可执行示例。
`examples/shared/bullet_widgets.rs` 和 browser-host 示例覆盖三个角色、原数据更新、
行重排、仅元数据更新、内部状态／焦点、隐藏保留、resize／CSS 缩放、拒绝候选、
失败组件修订、清理和重新挂载。详见[验收记录](verification.md#development-typed-bullet-widget-overlays)。
