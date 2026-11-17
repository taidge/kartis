# Waffle 分组事件区域（Kartis 0.2.0）

`waffle_area_interaction(&layout)` 返回受检 `SceneInteraction`：每个可见且实际
占用格位的组只有一个逻辑站点，事件区域包含完整格位及单元格间隙。函数逐值保留
Scene，SVG、Canvas 和 raster 导出继续使用实际单元格、空格画笔、颜色与自定义几何。

```rust
use kartis::*;
let layout = waffle_layout(
    &ChartConfig::default(),
    &[RadialDatum::new("completed", 6.), RadialDatum::new("pending", 3.)],
    &WaffleLayoutOptions { total: Some(10.), ..Default::default() },
)?;
let frame = waffle_area_interaction(&layout)?;
let widget = chart_layout(frame).legend(false);
# Ok::<(), kartis::ChartError>(())
```

把完整帧交给 `chart_layout`、`BrowserChart::set_layout` 或 RasterSurface 对应的
layout 入口。浏览器初次挂载时，将 `frame.scene` 传作 Scene，`frame.interaction`
传入 `BrowserOptions::interaction_data`，`frame.plot` 传入
`BrowserOptions::interaction.plot`。CPU 图像入口为
`chart_layout(frame).raster(fonts, options)`。公共 Surface 模型处理相机逆变换、
悬停、激活、隐藏系列及候选事务。原生测试向 Memory 注入 Glory 有限 PlotPointer
协议，不能据此宣称已验证各平台 OS 指针传输。

Cells 模式保留组内首个占用格的真实 Mark 作为 canonical 站点，继续使用组 ID、原值、
格式化文字及已接受锚点。其它单元格成为该站点的物理别名，每组只有一个键盘控制。
Areas 模式使用已有的实心并集 Mark。空格、零分配和容量耗尽的组不生成站点。
重新分配可能改变 canonical 格位 ID；原始组身份应使用站点的 `series`。

逻辑区域选择与实际画笔观测分别报告。在间隙或零透明度单元格内可以选择组，
同时 `physical_hit_at` 为 `None`。别名单元格实际有墨迹时仍报告真实物理 ID 和
canonical 逻辑 ID。超出所属区域的墨迹仍可被物理观测，但不会选择该组。
隐藏系列同时禁用两者。区域独立于实时 Widget 替换：替换源墨迹不会删除显式声明的
逻辑事件层；浏览器组件自身的原生事件仍按组合宿主规则接收。

其它已有站点也可通过 `InteractionBinding::regions` 声明 `InteractionRegion`
矩形。该入口要求 `PaintedGeometry` 和 `Picking::Geometry`，不能与 polar 或
nearest/slice 模式组合。owner 必须唯一且引用真实绑定站点；所有矩形累计不得超过
`MAX_MARKS`。发布前验证坐标、正宽高及端点。左、上边包含，右、下边排除；重叠时
最后声明的区域优先。矩形使用 Scene 坐标，随已接受相机投影，但不会自动跟随几何
spring；重新布局后需同时发布新的 Scene 与区域。Mark 自身透明度及裁剪不定义这个
独立事件层，已接受的输入 plot 与 viewport 仍限制输入。

适配前完整验证已有 Waffle 布局、分配、生成标记与绑定；拒绝预存的区域、别名、
part metadata 和 polar 绑定。Waffle 公开元数据仍允许按已有 cell 适配器约定进行
完全一致的编辑，没有私有不可变原输入快照。自定义 cell 工厂和 Path 锚点可直接组合，
无需伪造透明物理 Mark。

Nivo 的 SVG/HTML area 层使用透明事件几何，Canvas 按完整 cell 查询。本入口落实其
占用组事件角色，但 Nivo 的共享边包含与源顺序优先不等同于 Kartis 的半开矩形及
显式逆声明优先。`WaffleMode::Areas` 继续表示实心并集绘制。Nivo 没有
`areaComponent` prop；本入口未新增组 Widget 工厂、React spring 属性、任意实时
组件导出或 GPU/worker 渲染。

浏览器验收脚本为 `scripts/waffle-regions-acceptance.mjs`，`--chromium-only`
执行 CI 子集；共享场景见 `examples/shared/waffle_regions.rs`。
这些 API 自 Kartis 0.2.0 提供。
