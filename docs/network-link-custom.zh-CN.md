# 自定义力导向网络连线

[English](network-link-custom.md)

**Unreleased。** `network_glyph_layout_with_link_factory` 将自定义连线几何与画笔和
已有节点 glyph 组合。回调在物理布局后执行，不修改模拟、节点物理半径或力的端点。
生成的 Scene 由 SVG、Canvas、CPU raster 和支持实色的语义 Graphic 共用。

```rust
use kartis::*;
use glory::render::{GraphicPoint as P, PathCommand as C};

let simulation = ForceSimulation::new(
    &[NetworkNode::new("source"), NetworkNode::new("target")],
    &[NetworkLink::new("source", "target", 4.)],
    ForceOptions::default(),
)?;
let frame = network_glyph_layout_with_link_factory(
    &ChartConfig::default(), &simulation, &NetworkGlyphOptions::default(),
    &mut |ctx| {
        let [x0, y0] = ctx.source_position;
        let [x1, y1] = ctx.target_position;
        Ok(ctx.inherit(Geometry::Path(vec![
            C::Move(P::new(x0, y0)),
            C::Cubic {
                first: P::new(x0, y0 - 30.),
                second: P::new(x1, y1 - 30.),
                end: P::new(x1, y1),
            },
        ])))
    },
)?;
let semantic_frame = frame.into_painted_scene_interaction()?;
# Ok::<(), ChartError>(())
```

借用上下文提供解析后的 `ForceEdge`、输入顺序索引、当前源／目标 `ForceNodeState`、
原几何与画笔、实际 Scene 端点、原始中点、世界平移和绘图区。坐标已经包含一次平移，
调用者无需再加。`ctx.inherit` 保留原画笔；返回受检的 `NetworkLinkGeometry` 可改颜色、
描边、dash、渐变或轮廓。Path 可偏移、弯曲或含多个子路径；Rect、Circle 使用原中点
作为逻辑原点。连线 bounds 不包含描边延伸。

工厂按原边顺序调用，保留零权重、平行边和自环。完整继承保持原 glyph Scene 和节点
绑定相同，原值、文字、ID、绘制顺序和节点 glyph 不变。普通力导向连线沿用位置 ID，
仍为装饰；自定义墨迹不会增加节点控件或变成节点拖动目标。
`network_glyph_relation_layout_with_link_factory` 另接显式稳定边 ID，保留关系图、
归属及高亮所用作者画笔；它沿用既有关系入口的交互协议，不隐式增加最近边数据站点。

原生借用工厂不会被保留。回调 Err 或非法几何拒绝整个候选，不改变输入模拟或旧帧。
全部节点与连线共享 Scene 的 200,000 条路径命令和 16,384 个渐变停点预算，每个渐变
还须遵守自身停点限制。

## 保留式浏览器驱动

`BrowserNetwork::mount_with_link_factory` 接受相同 glyph 配置及显式保留的 `'static`
工厂。tick、拖动、resize 和源替换候选均从当前模拟状态重建连线；Scene 与节点绑定
发布成功后才提交物理位置。失败候选保留已接受的帧和位置，释放失败拖动，并可通过
已有 reheat／替换接口恢复。

源替换预检可能先调用回调，稍后的 RAF 再调用；空闲驱动帧也可能重建，不能把调用次数
当作数据变更订阅。外部回调副作用不回滚。回调可以读取已接受状态或停止宿主；嵌套
连线渲染返回受检错误。停止只释放工厂一次，包含在源替换预检内部发起停止的情况。

## 边界与证据

此入口对应 Nivo 0.99 的 `linkComponent`／`renderLink` 场景，提供共享 Scene 绘制钩子；
不复制 React 组件、任意用户 datum 字段、spring 生命周期或另一力布局的最终坐标。
原 Nivo Canvas 默认绘制继承 lineCap，零线宽赋值由真实 Canvas2D 忽略；SVG 使用 round。
Rust 的继承配置保持既有 round 端帽和零权重线宽规则。

语义 Graphic 宿主要求受支持的实色画笔及单标记不超过 256 条命令；渐变和更长路径使用
SVG／Canvas／raster。任意实时 Widget 子组件及快照仍属独立协议。

测试覆盖完整默认继承、来源身份、自环／平行边、关系归属、累计预算、真实 Glory 笔输入
及 CPU 像素。浏览器专项验证非等比 CSS 下的可信节点拖动、tick 后的连线端点、替换／
resize、来源与绘制回滚、回调重入和释放。原 Nivo 源码、完整 MIT 及真实 Canvas 函数
参考保存在[参考夹具](../tests/fixtures/node-link-custom/)。最终范围和首轮失败见
[验收记录](verification.md)。
