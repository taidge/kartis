# Stream 点组件

此开发 API 尚未包含在已发布的 0.1.0 中。`stream_dot_widgets` 用真实 Glory 子组件
和拥有所有权的 Rust 数据承接 Nivo Stream 的 `dotComponent` 角色，可用于类型化
覆盖层、严格替换、CPU 图像 Chart 及浏览器 Scene 顺序组合。
[完整可运行示例](../examples/stream_widgets.rs)使用 Memory，应用可选择实际 Glory 宿主。
完整构建代码见 [English guide](stream-widgets.md)。

`StreamDotWidgetData` 保留共享的 `Arc<BarSeries>`、原系列索引、图层身份、文字和源颜色、堆叠
顺序、`StreamSample` 及 plot。采样保留数字/字符串分类身份、原列索引、原始值、格式化
文字、未缩放的堆叠区间、投影边界和配置后的点位置。同系列插槽共享一份原始快照。
源图层颜色与自定义点的实际画笔分开。当前几何、画笔与透明度从宿主已接受帧的 Mark 读取，
不把兼容画笔标量冒充空值来源。

定位回调借用 `StreamDotWidgetContext`，可返回 `None` 跳过该角色。第一次回调前会完整
核对原来源、计算布局、完整尺度配置、真实 Stream 标记及绑定。修改来源外观或几何时，
应重新布局，必要时使用 `stream_layout_with_dot_factory`；构建后篡改生成的 Stream
标记或来源元数据会被拒绝。回调外部副作用仍由应用负责，宿主最多接纳 128 个已选插槽。

启用且非空的采样才是点组件角色；`Some(0.0)`、零尺寸和自定义零透明度点仍可交给组件。
布局隐藏的系列不生成点组件。`dots: None` 和空值仅保留采样占位及 slice 业务数据，
不生成点组件，完整原系列快照仍保留 `None`。空值变成零时会新增此前不存在的子组件。
这是 Rust 的显式空值约定；原 Nivo Stream 公共数据类型为数值。Start/Center/End 使用
配置的第一边界、中点或第二边界，屏幕范围反向时也不改成按上下排序。

Mark ID 与 revision 不变时保留组件状态，并更新共同接受的数据；revision 改变时重新
挂载，角色禁用或删除时释放所有者。这与 Nivo 的 React 列索引 key 约定不同。宿主级
隐藏系列保留已有作用域，同时隐藏并禁用组件。按共享插槽策略，部分裁剪的组件可见但
不接收输入。

绑定 Chart 的 `AtDatum` 现通过已接受的 `InteractionBinding` 解析锚点，支持墨迹偏移
的 Path；先确定原始采样锚点，再应用已接受图像相机。只有 Scene、没有显式锚点的
Path 仍须使用 `Rect`。仅绑定发生变化时，也与 Scene、组件包共同校验。严格组合或
PNG 发布失败会保留原来源、控制、组件身份、焦点与像素。

Nivo 参考检查实际执行原 StreamDots 参数与启用门禁，不代表 React 生命周期或像素等价。
Memory 人工派发只证明端口协议，不等于操作系统输入。自动弹簧、React `animatedProps`、
任意实时组件导出及 GPU/worker 仍为独立任务。

[English](stream-widgets.md)
