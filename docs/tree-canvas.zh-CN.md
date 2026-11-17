# Tree 原生 Canvas 绘制回调

[English](tree-canvas.md)

这些 API 保持 **Unreleased**，不属于 crates.io 已发布的 0.1.0。
原生绘制接口需要 WebAssembly 上的 `canvas` 功能，保留浏览器所有者需要 `web`。

`TreeCanvasCallbacks` 为节点、连线、标签和命名自定义层提供真实
`CanvasRenderingContext2d`。同一轮绘制共享一个原生上下文，用户回调之间保留
变换、当前路径、裁剪、透明度和合成状态；框架不逐次包裹 `save`/`restore`。

`TreeCanvasPlan` 按原样执行层顺序，允许重复内置层和自定义键；默认依次为
links、nodes、labels、mesh，省略的层不绘制。网格诊断同时受 debug、interactive
和 use-mesh 门禁控制，诊断多边形最多支持 512 个可见站点；普通最近节点查询
使用另一套有界空间索引。

## 完整数据和标签资格

使用 `tree_canvas_layout`、`tree_canvas_layout_with_factories` 或
`tree_canvas_layout_with_activity` 构造权威 Canvas 数据。每个节点的标签函数在
最终活动尺寸和颜色确定后按广度优先调用一次。关闭的标签仍保存在完整计算池中，
普通 Scene 仍只绘制符合门禁的标签。关闭、透明和零宽连线也保留最终粗细及颜色。

结合实际调用者配置，通过 `TreeCanvasFrame::from_all_computed` 或
`TreeCanvasFrame::from_all_computed_activity` 建立绘制帧。回调可读取原始输入、
拓扑、计算节点、连线端点、活动、主题和当前 Scene 绘制信息。原始正负零保持原位型；
绘制过程不会重放布局工厂来恢复元数据。

标签上下文区分权威计算的 label/font/clip，以及可选的当前 `scene_label` 和
`scene_index`。编辑 Scene 标签不会改写构造时的计算池。`TreeCanvasFrame::new`
和 `with_activity` 则明确采用 `CurrentScene` 标签资格，无法恢复旧构造器省略的标签。

## 直接绘制和封存准备

`draw_tree_canvas` 直接绘制到调用者提供的 Canvas；返回错误时，目标可能已有部分新像素。

`prepare_tree_canvas` 在私有候选 Canvas 上执行回调，检查尺寸和同源读回，再复制到
封存位图。封存上下文从不交给应用回调。`PreparedTreeCanvas::to_png_data_url`
直接导出已有像素，不再次运行回调。无论成功或失败，候选上下文都会退役；保留它不能
改写封存位图。

初始上下文只应用一次 DPR 和相机。Kartis 坐标已经包含绘图区边距，不要再平移一次。
每遇到 links 层就绑定真实连线生成器，包括空 links 层；每遇到启用的 labels 层，
只在该层入口初始化一次 Rust 主题字体。

连线生成器接受任意合法有限端点，支持修改上下文、曲线和方向。未绑定上下文时返回
SVG 路径；绑定后只追加原生路径，不自行 beginPath 或 stroke。径向端点采用显式
polar 方法。

## 保留浏览器所有者

`BrowserTree::mount_with_canvas` 将 `BrowserTreeCanvasProgram` 与布局构造器分别保留。
工厂和类型化组件变体可将权威活动构造器与同一绘制程序结合。`refresh` 重算显式候选；
即使 Scene 未变，`set_canvas_program` 也能更换回调和层并形成新画面。

绘制准备在 Engine 状态借用之外运行，回调可读取所有者已接受的快照。在自身候选回调中
修改或 flush 同一所有者会被拒绝；stop 阻止后续回调及提交。错误恢复直接使用已接受的
封存像素，不重放有状态回调来重建旧画面。

原生相机、DPR 和隐藏状态变化重新准备真实像素，不重跑布局工厂。空闲 flush 不重绘。
PNG 导出包含自定义 Canvas 像素，排除 HTML 组件，且不重新调用绘制函数。
将 `frame.layout().scene` 导出为 SVG、CPU 位图或 CanvasScript，只会导出源 Scene，
这些格式不会执行或复现原生回调。

原始上下文模式要求 Canvas、受控高亮且没有源 crosshair。类型化组件必须使用严格顶层
叠加：`source_ordered: false`、`replace_ink: false`。不支持的组合直接拒绝；
任意原生合成无法读取或擦除真实保留的 HTML Widget。原有 SVG 和源顺序组件接口分别保留。

命中使用源 Scene 几何，或独立配置的节点中心活动网格。绘制不同形状不会产生新的物理
命中几何。按源 ID 隐藏时，节点、标签及关联连线一起省略。网格诊断选项不替代
`BrowserTreeOptions.picking`。

## 预算和与 Nivo 的差异

- 默认最多 128 次层出现及 200,000 次角色／源指令工作；这与 128 个测量 Widget 槽位不同。
- 累计 Canvas 像素准入包含候选、封存及调用者预留缓冲区。默认纹理 tile 和图元透明度
  离屏缓冲消耗剩余绘制预算；单图元还受边长 8,192、16M 像素限制。回调改变变换后，
  可能在绘制阶段触发有界分配错误。
- 回调属于受信任应用代码。真实上下文无法计量其自行分配、无限循环，也不能回滚 DOM
  修改和其他外部副作用。
- 默认绘制保留 Rust 完整 Scene 几何、填充及字体，并恢复自身样式；不复制 Nivo 默认
  绘制器的全部样式残留。默认连线使用当前 Mark 几何，不随通用生成器的后续修改而改变。
- Rust 保留 alphabetic 文字基线、主题子集、投影默认值、扁平源 ID 和纵向 step 语义。
  每轮绘制建立新连线生成器，不保留跨轮 D3 上下文对象身份。
- 这些功能不等于 React/Spring、任意 JS accessor、D3 数值同一、自定义像素命中、
  Widget 导出或 GPU/worker 渲染。

实际验收结果和剩余工作分别记录在[验收说明](verification.md)与
[差距清单](feature-gaps.md)中。
