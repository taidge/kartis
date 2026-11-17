# 按 Scene 顺序嵌入真实 Widget

[English](ordered-widgets.md)

开发中的 API，已发布的 0.1.0 包不包含此功能。

`BrowserChart::mount_with_plot_widgets_in_scene(root, scene, options,
composition, factory)` 在 Scene 原始 Mark 的绘制位置插入真正的 Glory Widget。
SVG 与 Canvas 使用同一套顺序编译器。组合包和工厂类型沿用
[严格组合接口](widget-composition.zh-CN.md)。
`set_plot_widgets_in_scene` 更新组合包；
`set_layout_with_plot_widgets_in_scene` 同时更新 Scene、交互绑定和组合包。
`BrowserHandle` 也提供这两个 setter。宿主保持挂载时的元数据类型和组合模式，空包也不例外。

## 原始顺序与绘制

插槽紧接其源 Mark 插入。ID 属于 `replace_ink` 时，Widget 占据该 Mark 的位置，
原物理墨迹不再绘制。标签保持原有位置，不根据文本或名称推断标签归属。
顺序以 `Scene::draw_order` 及其默认 item 顺序为准。需要 Bullet 的
ranges、measures、axis、markers、title 等角色顺序时，应用必须显式编排 Scene。

`widget_paint_plan` 是不绑定宿主的受检编译器。不可变的 `WidgetPaintPlan`
包含透明 `Paint` 绘制段、`Widget` ID 和不相交的 `Group` 节点。
段内 Scene 保留所需字体、文字声明及裁剪，不重复背景或源组透明度。
宿主只绘制一次背景，先合成组内所有子项，再对合成结果应用一次组透明度。
浏览器的两种 renderer 均遵守此约定，真实 Widget 也参与组合。

非 Normal 混色的源 Mark 独占一个绘制段，通过 CSS `mix-blend-mode`
与其前方的源图形及 Widget 内容合成。源 Mark 的混色和透明度不会自动成为
替代 Widget 的样式；控件外观由工厂负责，外层 Scene 组透明度同时作用于两者。
运行时辅助图形最后绘制，不拦截指针。

## 浏览器原生命中

后绘制的源墨迹同时遮挡前方 Widget 的画面和原生指针命中。
每个源绘制段都有透明 SVG 指针伴随层，裁剪到实际 Canvas 中 alpha 大于零的像素。
浏览器决定真实指针目标和 CSS `:hover`，宿主不合成点击。
透明填充、路径孔洞、虚线间隙和裁剪孔洞允许穿透。
没有业务交互的装饰墨迹和标签也能遮挡 Widget。
子组件的键盘和指针操作归组件自身，不会同时激活源图表控制器。

命中精度量化到已接受的 Canvas 设备像素比，SVG 输出也使用此命中规则，
不承诺精确复现 SVG 亚像素边缘。Canvas 的小数尺寸按实际取整后的 backing
宽高映射到显示视口。挂载或更新前应准备好字体资源；源 Canvas 与 SVG
使用一致的显式从左至右段落方向，仍继承语言属性。

替换的物理几何不再参与源指针拾取。原始 Scene、类型化元数据、逻辑控制和
程序化选择继续保留。部分裁剪插槽维持可见但 inert 的策略。
隐藏和零透明度源组保留组件状态，同时隐藏组件并禁止输入，不删除逻辑源。

## 身份与拒绝恢复

ID 和 revision 不变时，改变顺序或组父节点仍保留组件作用域、实际 DOM 身份和本地状态。
接受后的类型化数据与几何一起更新。revision 改变会构造新子组件；
删除插槽在接受后销毁其组件。

候选绘制面和新子组件先在隐藏状态准备。工厂、子组件 build、校验、绘制或分配失败
会拒绝整个候选，保留此前接受的源、画面、控制、数据和子组件。
准备阶段，已有子组件只读取此前接受的数据。
已有组件成功移动前保持旧父节点存活；移动失败恢复旧父节点和顺序。
应用在候选作用域外产生的副作用无法回滚。
提交后才观察到的动态子绑定错误属于普通 UI 错误，不撤回已经提交的源。

计划最多接受 128 个插槽、512 个源绘制段、1,024 个总节点和 200,000 条原始几何命令，
被替换的墨迹也计入命令预算。浏览器的存活及待提交绘制面、组透明度和混色 backing、
alpha 读回共享 67,108,864 像素预算，命中伴随层共享 200,000 个矩形。
pattern 准备另限制累计 tile 像素为 16,777,216、tile 操作为 268,435,456。
待提交子组件的既有配额继续适用。满足某个限额不代表满足其他限额；
拒绝时保留此前画面。

## 范围

此入口仅用于浏览器。共享编译器和受检组件移动为其他宿主提供基础，
原生宿主同层精确输入遮挡仍未完成。语义与 CPU 图像宿主的
`Chart::plot_widgets_composed` 继续使用顶部层。

Scene、SVG、PNG 导出不捕获任意真实 Widget 内容，导出原始 Scene 仍包含原墨迹。
Nivo 自动 spring、`animatedProps`、GPU 和 worker 渲染继续保留为独立缺口。
参见[功能差距](feature-gaps.md)及[验收记录](verification.md)。
