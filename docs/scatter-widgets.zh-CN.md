# Scatter 实时节点组件（Kartis 0.2.0）

`scatter_node_widgets(&layout, &series, revision, placement)` 为真实 Scatter
节点构建经过测量、拥有类型化数据的组件包。需要显式传入原始 `ScatterSeries`，
因为现有公开 `ScatterLayout` 未保留每个原 datum 字段。这样既保持其公开结构及已有
编辑约定，也能向子组件提供完整原数据。

```rust
use kartis::*;
let series = [ScatterSeries::new("measurements", vec![
    ScatterDatum::new("first", 1., 4.),
    ScatterDatum::new("zero", 2., 0.),
])];
let layout = scatter_layout(&ChartConfig::default(), &series, &Default::default())?;
let widgets = scatter_node_widgets(&layout, &series, 0, |_| {
    Ok(Some(PlotWidgetPlacement::AtDatum {
        offset: [-16., -11.],
        measured: TextSize { width: 32., height: 22. },
    }))
})?;
# Ok::<(), kartis::ChartError>(())
```

每份 `ScatterNodeWidgetData` 包含共享 `Arc<ScatterSeries>`、当前 `ScatterSite`
及 plot。原 datum 可从 `data.series.data[data.site.source_index]` 读取。同一系列
选中的节点共用一次完整原系列克隆，未选中的系列不克隆。原始 XY 保留 Number/String
类型及数值符号位；可选尺寸、原标签和显式源色都保留。计算索引、格式化 XY、解析后的
尺寸和单位、映射位置及边界来自 site，当前几何、填充、描边和透明度来自宿主已接受帧
的实际 Mark。原始源色与自定义节点画笔的代表色可以不同。

借用的定位上下文提供 layout、series、原 datum、computed site 和实际 Mark。
所有定位回调前先完成整帧一致性检查：每个原记录、元数据和全局索引、完整尺度配置、
投影、站点、标记、边界及类型化锚点都须一致，包括未选中和后部记录。
缺失尺寸跳过、隐藏系列和显式 Point 域省略只保留元数据，不造实际节点角色。
已有零尺寸、零透明度和 plot 外节点仍有组件角色；测量裁剪与可见性决定其能否绘制、
能否接收输入。plot 外子组件可以保持挂载但处于 inert 状态。

公开布局元数据仍允许完全一致的编辑。这是对显式提供的当前源做一致性检查，
不是私有不可变原输入快照。布局未保存原尺寸 accessor 和数值格式选项，适配器检查
解析尺寸和拼装标签，不能从任意公开编辑推断那些未保存的配置。site 颜色是当前
元数据，不会覆盖真实 Mark 的画笔。Scene 的兼容数值标量也不替代 raw XY；字符串 Y
可以使用兼容零标量，类型化数据仍保留原字符串。

定位回调返回 `None` 可跳过节点，最多选择 128 个插槽。回调仅借用、不留存，
错误不返回部分数据包，外部回调副作用不回滚。显式 revision 改变请求新的子作用域；
普通元数据更新按既有宿主规则保留身份、状态和焦点。

通过 `Chart::plot_widgets_with_data` 添加覆盖层，或用 `PlotWidgetsComposition`
及 `plot_widgets_composed` 事务替换选中的源墨迹。CPU 图像 Chart 入口为
`chart_layout(frame).raster(fonts, options)`，已接受相机投影插槽，图像 DPR 与此独立。
语义 `chart_layout` 要求显式 `PaintedGeometry`：Scatter layout 转为
`SceneInteraction` 后，在挂载前设置其 binding profile。浏览器原绘制顺序组合使用
`mount_with_plot_widgets_in_scene` 及对应 setter。类型化数据、子组件构建或 PNG 发布
失败时，保留此前接受的源、控制和子组件；诊断用工厂计数仍属于外部副作用。

Nivo `Nodes` 为每个 transition 项调用 `nodeComponent`，包括零尺寸。
`isInteractive` 移除事件处理器，不移除角色；常规 style 使用节点 XY/size/color，
离场 style 把 size 设为零。冻结原源码验收执行这些参数表达式、真实元素创建回调、
`computePoints` 及默认半径计算，不运行 React 协调、SpringValues 或过渡时序。
Nivo `serieIndex` 实际表示系列内 datum 索引；Kartis 使用独立的 `source_index`
和 `series_index`。Rust 的隐藏、缺失尺寸和 Point 域省略是明确的 Rust 约定，
不冒称原 Nivo 使用同样的过滤规则。

`cargo run --example scatter_widgets` 可运行完整 Memory 应用。
浏览器验收脚本为 `scripts/scatter-widgets-acceptance.mjs`，CI 使用
`--chromium-only`。原版核验命令为
`node scripts/scatter-node-widget-reference.mjs --frozen`，无需 checkout 或网络。
Scene 的 SVG/PNG 导出继续绘制原墨迹，不包含任意实时组件。自动 spring、React
animated props、任意 Widget 导出、原生同层输入及 GPU/worker 仍保留。
已发布 0.1.0 尚不包含该 API。
