# Tree 节点提示组件（Unreleased）

[English](tree-tooltip.md)

BrowserTree 展示接口需要 WebAssembly 上的 `web`；`TreeTooltipData` 也可作为纯 Rust
数据使用。这些 API 保持 Unreleased，不包含在已发布的 0.1.0 中。受检 opt-in 浏览器
范围覆盖 Chrome、Firefox、WebKit、Edge 和独立 Chromium 的 DPR1/2，以及 SVG／Canvas
和原生 Canvas 源程序；实际范围见[验收说明](verification.md)。

`BrowserTree` 提供显式启用的提示展示通路，与共享图表的实际墨迹命中分开。
普通挂载保持原提示默认行为。在 handle 上调用 `set_node_tooltip_options` 启用
安全 Tree 文本，或用 `set_node_tooltip_component` 提供真实保留式 Glory Widget；
后者也会启用默认 Tree 提示选项。`BrowserOptions.tooltip` 必须保持启用。

## 目标选择与完整数据

启用后，由**新的可信主鼠标或笔 Move** 建立目标，不回填先前未配置时的指针，
也不根据活动焦点推断目标。`BrowserTreePicking::Nearest` 查询已接受的计算节点
中心并采用配置距离；`Geometry` 保持原有严格源交互。Leave 通过同一接纳边界请求清除目标。

提示选择与关系高亮、焦点、选中状态及 `SurfacePhysicalHit` 分开。显式设置 activity
不会创建提示，直接派发共享图表输入也不冒充原生提示意图。Nearest 可展示零半径、
透明或空业务标签节点，不增加 Mark、canonical 控件或虚假物理命中。隐藏的源 ID
不参与选择；原始顺序变化不会把一个已选择 ID 偷换到另一索引。

`TreeTooltipData` 持有经过校验的源／布局／活动快照，提供 `source`、`source_index`、
`datum`、`node`、`layout`、`activity_layout`、`activity`、`config`、`anchor` 和
`is_active`。布局可读取完整节点／连线池、最终连线样式及构造器已计算的 Canvas
标签。读取这些数据不会重放布局工厂。

`datum().value` 保留原始输入及正负零位型；`node().value` 是计算聚合值。
安全 `text(&TooltipOptions)` 使用聚合值，与原 Tree 文本提示一致；模板仍是纯文本。
即使只改变未选择兄弟节点的零符号，类型化数据也保持可观察更新。

## 真实保留组件

下面为已经挂载的 owner 配置组件。必须继续保留 `BrowserTree`，handle 是弱引用。
setter 排队更新，`flush` 同步处理。示例核对了真实公开签名，保留式 frame 协议
由本文所述 opt-in 浏览器范围验证。

```rust
use glory::Widget;
use kartis::browser::{
    BrowserTreeHandle, BrowserTreeTooltipAnchor, BrowserTreeTooltipOptions,
    BrowserTreeTooltipPosition,
};

fn configure_tooltip(owner: &BrowserTreeHandle) -> kartis::Result<()> {
    owner.set_node_tooltip_options(BrowserTreeTooltipOptions {
        position: BrowserTreeTooltipPosition::Cursor,
        anchor: BrowserTreeTooltipAnchor::Top,
        gap: 14.,
    })?;
    owner.set_node_tooltip_component(0, |context| {
        let frame = context.frame;
        Ok(glory::ui::widget(move |scope| {
            glory::ui::text()
                .content(scope.bond(move || {
                    frame.get().map_or_else(String::new, |frame| {
                        format!(
                            "{}: {} (raw bits: {:016x})",
                            frame.data.datum().label,
                            frame.data.node().value,
                            frame.data.datum().value.to_bits(),
                        )
                    })
                }))
                .show_in(scope)
        }))
    })?;
    owner.flush()
}
```

工厂接收 `BrowserTreeTooltipComponentContext { revision, frame }`，其中 `frame` 为
`Bond<Option<TreeTooltipFrame>>`，初始是 `None`。帧包含类型化数据、源 generation、
最后一次原生 client 坐标、逻辑视口锚点及对齐。每个响应式视图都应处理 `None`；
没有目标或展示盒当前不可用时也会出现该值。

切换节点或源数值保留组件局部状态。同 revision 保留原工厂和 Widget，新 revision
才重建。`clear_node_tooltip_component` 返回安全文本；该清除被接纳后，再安装旧
revision 会创建新组件。`reset_node_tooltip` 清除受管提示选择并恢复原 Core 提示模式，
不会设置或清除 activity。

请求中的配置与已接受配置分开。新组件或配置被拒绝后，原生 Leave／重入使用已接受
配置；显式源更新或 `refresh` 才重试请求中的配置。被拒绝的 reset 不会清除已接受的
目标或缓存，只有该 reset 被接纳后才切换为 Unmanaged。

包装层默认 `pointer-events:none`，不会自动给自定义内容绑定节点激活处理器。
键盘焦点和直接输入不建立原生提示意图；本接口不新增触摸或 OS 焦点合同。

## 定位与提交

默认使用 `Fixed`、`Top` 和 14 个局部 CSS 像素间距。Fixed 将计算节点中心通过
已接受的相机定位，Cursor 跟随最后一次实际投递的原生指针。内容采用自然尺寸，
最大宽高受源 surface 范围约束。九种对齐为 `Top`、`TopRight`、`Right`、
`BottomRight`、`Bottom`、`BottomLeft`、`Left`、`TopLeft` 和 `Center`；Center
不添加方向间距。

锚点与间距在局部 CSS 空间一起取整。共同祖先或 surface 自身的轴对齐缩放在定位时
转换，DPR 单独处理；任意 CSS 旋转、斜切或透视不在此范围内。没有自动碰撞避让，
提示盒可以延伸到绘图区之外。

`None` 或 unmanaged 提示本身不要求已解析的 CSS 展示盒。联合源标签组件有另一项
要求：自动仿射 carrier 需要已解析的 CSS 尺寸。`display:none` 可能使 `100%` 等尺寸
未解析，从而拒绝联合源候选。已接受数据继续保留；恢复源的可见性后显式 `refresh`
再试。

同一目标内移动复用不可变数据缓存，只更新提示展示，不重放源布局工厂、源组件定位
或原生 Canvas 回调。程序 current 与原生提示目标继续分开。原按压生命周期仍会将
自动更新延后至 release 及其派生 click 完成。

源与提示数据和保留式源组件共用接纳边界。接纳前的构造／定位／绘制错误保留旧源
和旧提示；接纳后的动态 Widget 错误归属于已接受数据，失败的展示会隐藏，不把旧源
画面继续当成新数据。应用自身外部副作用不可回滚。持续的源构造器、源定位或原生
绘制错误保留共同已接受的源和提示；原生 Leave 不会通过 tooltip-only 更新绕过这些
错误。修复源错误后显式 refresh。`node_tooltip()` 返回已接受的
逻辑数据，不保证弹出框当时可见。空安全文本可以隐藏提示，自定义内容仍可使用该
类型化帧。stop 释放所属组件。

即使覆盖在原生 Canvas 程序之上，提示 Widget 也是 HTML 展示，不进入原生 Canvas
PNG 或源 Scene 导出。

本接口适配 Nivo 可选的 `nodeTooltip({ node })` 内容契约，不执行 React/Spring，
不补造任意泛型 Datum 字段或嵌套路径 UID，也不承诺 Delaunay 平局行为。Rust 保留
扁平源 ID、有界最近距离和原绘图区输入范围。GPU／worker 继续后置。相关边界与
实际证据分别见 [Tree 活动](tree-activity.zh-CN.md)、[原生 Canvas 程序](tree-canvas.zh-CN.md)
和[验收说明](verification.md)。
