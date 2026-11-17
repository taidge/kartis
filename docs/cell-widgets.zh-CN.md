# HeatMap 与 Waffle 的类型化单元格控件

[English](cell-widgets.md)

开发中的 API，不包含在已发布的 0.1.0 包内。
这些适配器为 Nivo HeatMap、Waffle `cellComponent` 对应的单元格角色提供真正的
Glory 组件，不保存 React 回调，不生成 `animatedProps` 或自动 spring 轨迹。

## 原始数据与测量位置

`heatmap_cell_widgets(&layout, revision, placement)` 返回
`PlotWidgetsData<HeatmapCellWidgetData>`。借用的回调上下文包含 `layout`、原始
`datum: &HeatCell`、原始 `source_index`、当前计算后的 `cell` 和实际 `mark`。
返回数据拥有原始 datum、输入索引、当前计算单元格和 plot。
调整行列顺序不会改写原始输入索引；`None` 与 `Some(0)` 始终不同，稀疏输入中不存在的
行列对不会补造。隐藏单元格没有实际 Mark，不调用回调；已有的零尺寸、零透明度 Mark 仍会调用。

`waffle_cell_widgets(&layout, revision, placement)` 返回
`PlotWidgetsData<WaffleCellWidgetData>`。上下文包含 `layout`、完整格位 `cell`、可选
所属 `group` 和实际 `mark`。返回数据包含 cell、`Option<Arc<WaffleGroup>>`、total、unit、
grid 和 cell_size。空格没有源 group，不用虚构的零值分组代替。原始数值保持数值类型，
不依赖格式化文字。同组已选单元格共享一份拥有所有权的快照，元数据相等性按值比较。

只有实际单元格 Mark 调用 Waffle 回调。在 `WaffleMode::Areas` 中，占用格属于实心组区域，
不再有独立单元格 Mark，因此跳过；空格仍有实际 Mark，可挂载组件。
此接口不提供组区域组件，也不把 Kartis Areas 改成 Nivo 的透明交互层。

两种适配器都先完成整份输入校验，再调用第一个 placement 回调。
返回 `None` 跳过该格，最多选择 128 个插槽；大布局可以只选子集。
数据包携带布局的实际 plot，宿主继续检查测量尺寸、裁剪和能力支持。
回调不保留，错误不返回半包，但回调在应用外部产生的副作用不能撤销。

## 当前画笔与原始业务数据

实际几何、填充、描边、透明度以 `context.mark` 或实时 `frame.frame.mark` 为准。
自定义单元格可以是受检 Path 或 Circle，不必还是原矩形；band、完整格位和逻辑锚点不会
随偏移墨迹一起改变。应用选择显式矩形或测量后的 `AtDatum` 位置，Path datum 定位需要
其已接受的类型化锚点绑定。

HeatMap 通过私有来源保留原始空值，并私有保存原输入顺序。
默认布局沿用既有公开外观编辑约定：计算单元格的几何必须与实际 Mark 一致，透明度必须与
已接受 profile 一致。palette 元数据与装饰后的实际 fill 分开；公开行列顺序可编辑，
但不能含未知或重复身份。自定义工厂布局继续遵守既有更严格的几何、画笔和所属标签保护。
Waffle 校验当前公开分配、网格、分组、实际 Mark 与类型化站点。
两者都不从显示文字或 Mark 的兼容默认标量反推原始数据。

显式类型化数据包允许绑定空标签的真实装饰 Mark，因而能挂载 Waffle 空格组件。
原始 Mark 不变，也不生成业务 datum、canonical 控制或拾取站点。
旧无类型插槽入口继续保留 data-Mark 准入规则。子组件自行处理指针与键盘操作，
激活不会自动选择其下方的源数据。

## 组合宿主

需要保留原墨迹时，使用 `Chart::plot_widgets_with_data` 或浏览器类型化覆盖层接口。
需要替换时，包装为 `PlotWidgetsComposition { widgets, replace_ink }`，
替换集合只能包含真实已选插槽 ID，再交给[严格组合](widget-composition.zh-CN.md)，
或支持 SVG/Canvas 与组件交错的[浏览器 Scene 顺序组合](ordered-widgets.zh-CN.md)。
原有 Scene 标签保持原始位置，替换单元格不会自动推断、删除其标签。

ID 与 revision 不变时，已接受数据变化仍保留组件作用域和本地状态。
Waffle ID 表示物理网格位置：重新分配后，同一组件可以变更所属业务组。
应用需要重置本地状态时应改变 revision。重建布局删除实际 Mark 和插槽会销毁对应组件；
宿主图例隐藏或零透明度则可保留仍存在的插槽及状态，同时禁止输入。
正透明度和源颜色不会自动成为任意组件的样式，应读取已接受帧显式设置。

原生语义 `chart_layout` 可使用 HeatMap 的 `into_painted_scene_interaction()`，
它预检受支持的实色 painted geometry，并保留空值站点和自定义锚点。
普通 `into_scene_interaction()` 为浏览器及 Surface 保留 legacy 几何拾取。
语义 Graphic 和文字宿主的能力限制继续适用。
浏览器首次挂载应传 frame 的 Scene，并将绑定放入 `BrowserOptions::interaction_data`，
将 `options.interaction.plot` 设为相同 plot。这样第一次组件工厂执行前就有真实空值控制和
自定义锚点。后续更新使用 layout 与 packet 共同提交的 setter。

现有 `raster_chart` 接收 Scene 和测量后的类型化组件，不接收 `SceneInteraction` 绑定。
PNG 发布闸门同时控制严格组件更新，子组件元数据仍保留空值。
Scene-only 源控制和提示不能从兼容标量零推断空值，因此空值 HeatMap 图像示例关闭这些旧控制，
使用真实子组件自行显示业务值。`RasterSurface` 另外支持类型化绑定，但没有实时组件组合入口。
需要显式 PaintedGeometry 绑定时，新增的
[`chart_layout(frame).raster(...)`](raster-chart-bindings.zh-CN.md)可共同保留类型化源指针
与实时组件，无需从 Scene 标量推断数据。

## 完整示例

[HeatMap 控件示例](../examples/heatmap_widgets.rs)与
[Waffle 控件示例](../examples/waffle_widgets.rs)在真实 Glory Memory App 中挂载组件，
无需可选特性即可编译运行：

```sh
cargo run --example heatmap_widgets
cargo run --example waffle_widgets
```

应用选择实际 Glory 宿主，通过组件工厂读取 `context.frame`，即已接受位置和元数据的 Bond。
Scene、绑定与数据包需要共同变化时，应从同一份应用控制的源构建候选。
严格失败保留此前接受的源和子组件，不会改写应用尚未接受的提案。

Scene、SVG、PNG 导出不包含任意真实组件，导出原始 Scene 仍绘制原墨迹。
浏览器沿用有界 Canvas-alpha 命中 profile，SVG 也使用此规则。
原生同层输入、任意组件导出、自动 spring、GPU/worker 仍未完成。
参见[功能差距](feature-gaps.md)与[验收记录](verification.md)。

独立的 [Waffle 分组事件绑定](waffle-regions.zh-CN.md)补充完整格位业务拾取，保留此 cell Widget 适配器及实心 Areas 绘制模式。
