# Bar 实时组件（Kartis 0.2.0）

`bar_widgets` 从 `CustomBarLayout` 构建类型化测量组件包。
`bar_layout_with_factories(..., BarFactories::default())` 可继承普通 Bar 画笔，
支持分组、堆叠、百分比、横纵方向、分类窗口、标签及总计。几何/文字工厂和配置尺度的
工厂入口共用此组件 API，已有公开 `StyledBarLayout` 结构保持不变。

```rust
use kartis::*;
let categories = vec!["A".into(), "B".into()];
let series = [BarSeries::new("observed", vec![Some(3.), Some(0.)])];
let layout = bar_layout_with_factories(
    &ChartConfig::default(), &categories, &series,
    &BarOptions::default(), &BarStyleOptions::default(), BarFactories::default(),
)?;
let widgets = bar_widgets(&layout, 0, |_| {
    Ok(Some(PlotWidgetPlacement::AtDatum {
        offset: [-12., -5.],
        measured: TextSize { width: 24., height: 10. },
    }))
})?;
let frame = bar_widget_frame(&layout)?;
# Ok::<(), kartis::ChartError>(())
```

每份 `BarWidgetData` 包含共享 `Arc<BarSeries>`、共享完整分类向量、当前
`StyledBarDatum`、逻辑锚点与 plot、未裁剪矩形、未 clamp 的端点投影以及方向。
完整系列保留 None、隐藏和裁掉的源值，原数值符号位也保留。
`bar.displayed_value` 和 `bar.interval` 单独表达堆叠/百分比结果；
`bar.color` 是数值布局源色，当前自定义画笔来自宿主帧的实际 Mark。
同一系列选中的柱共用一次原系列克隆，所有角色和布局克隆共用完整分类向量。

`label`、`label_clip` 提供当前由 Scene 拥有的标签及裁剪，保留标签工厂结果、
空文字及已接受 `set_active` 产生的标签颜色变化。`None` 表示 Rust 布局未生成该标签。
替换标记墨迹后，标签和总计仍保持原 Scene 所有权；读取元数据不会自动重复绘制标签。
定位上下文提供这些值的借用版本，以及原系列、分类、当前柱与实际 Mark。

未裁剪矩形无法在有限坐标约定内表示时，`original_rect` 为 `None`；原端点也可以
分别为 `None`，不会补成裁边矩形或零值。`bar.visible_rect` 是普通布局的裁后矩形，
自定义 Path 可在别处绘制。`AtDatum` 使用已接受的原逻辑锚点，不从墨迹包络猜位置。

工厂布局构造时即捕获私有原始来源。所有定位回调前，完整验证已接受 Scene/元数据、
原始索引、值、分类、位置、实际标记和类型化绑定。需要修改受管公开帧时，应通过布局
入口重新构建。`set_active` 继续提供合法的受控透明度更新，保留原值和位置。
Scene 兼容数值标量用于画笔，不替代拥有所有权的原 datum。

只有实际生成的柱调用定位回调。None、隐藏系列和完全裁掉的柱不造角色；已有 Some(0)、
零面积、零透明度柱仍有组件角色，由宿主裁剪和可见性决定绘制与输入。
空布局也通过 `bar_widget_frame` 保留准确 plot。返回 `None` 可跳过选中角色，
最多选择 128 个插槽；回调借用且不留存，错误不返回部分包，外部回调副作用不回滚。

使用 `plot_widgets_with_data` 添加覆盖层，或用 `PlotWidgetsComposition` 和
`plot_widgets_composed` 事务替换源墨迹。浏览器原绘制顺序组合使用
`mount_with_plot_widgets_in_scene` 及对应 setter。`bar_widget_frame` 把 Scene、
原业务绑定、Path 锚点及 plot 一起返回，显式使用实色 `PaintedGeometry`。
CPU 图像入口为 `chart_layout(frame).raster(fonts, options)`，插槽随已接受相机投影，
与图像 DPR 独立。宿主执行最终画笔和裁剪准入。数据包、子组件构建或 PNG 发布失败时，
保留已接受源、控制、组件状态及焦点；显式 revision 改变请求新的子作用域。

Nivo 在创建 `barComponent` 前排除 null，保留零值。bars 图层开关决定角色是否创建，
交互和标签可见性是独立参数；原版过渡表达式分别收起竖柱高度和横柱宽度。
冻结验证器执行真实过渡表达式、公共参数、组件创建参数、标签布局及默认矩形 clamp，
不运行 React 协调或 spring 时序。Rust 的系列/分类数据模型、裁剪资格和零标签策略
继续作为明确的 Rust 合同，不冒称逐 prop 等价。

`cargo run --example bar_widgets` 运行完整 Memory 应用。浏览器验收脚本为
`scripts/bar-widgets-acceptance.mjs`，CI 使用 `--chromium-only`。
`node scripts/bar-widget-reference.mjs --frozen` 无需 checkout 或网络即可核验原源码。
Scene/SVG/PNG 导出保留原画笔，不包含任意实时组件。自动 spring、React animated props、
任意组件导出、原生同层输入及 GPU/worker 仍保留。已发布 0.1.0 尚不包含这些 API。

浏览器像素样本使用显式纯色 Button（font-size 为零），JSON 文字仍提供受检数据和状态。
独立标签阶段检查原文字、裁剪、dim 元数据及实际 Widget 禁用状态。
此前 font8 同色字形样本在实际 Canvas compositor 中有通道差 2，继续作为未验收文字诊断。
font8/0/8 可逆对照保留相同边界及状态，独立空白原生 Button 未复现；纯色矩阵不代表
文字栅格等价，颜色门槛仍为 ±1。
