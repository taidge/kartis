# SVG Tree 连线提示（Kartis 0.2.0）

[English](tree-link-tooltip.md)

本 SVG 适配自 Kartis 0.2.0 提供，并已完成受检浏览器矩阵，范围见
[验收记录](verification.md#development-tree-svg-link-tooltips)。
浏览器 API 需要 WebAssembly 的 `web` 功能；纯数据
`TreeLinkTooltipData` 也可用于原生 Rust。

## 配置与数据

`BrowserTreeHandle` 与 owner 提供 `set_link_tooltip_options`、
`set_link_tooltip_component`、`clear_link_tooltip_component`、
`reset_link_tooltip` 与已接受的 `link_tooltip` getter。handle 为弱引用，须保持
`BrowserTree` 存活；setter 排队，`flush()` 处理来源与提示的共同事务。

`BrowserTreeLinkTooltipOptions` 只有 `anchor` 与 `gap`，默认 Top、14 CSS 像素。
连线提示跟随实际原生光标，不提供固定连线中点定位。`BrowserOptions.tooltip` 继续
作为总开关。Canvas 与原生 Canvas 程序会在连线组件工厂执行前拒绝此配置。

配置组件同时启用连线选项；只配置选项时使用安全纯文本，按 source、target 两行显示
各端点的计算聚合值，不给连线编造一个 scalar value。这是显式 Rust 便利功能，
不是上游 Nivo 默认自带的连线组件。reset 移除连线托管；只清除组件则保留选项并返回文本。

`TreeLinkTooltipFrame` 提供 `data`、已采用的 `source_generation`、真实
`pointer_client`、逻辑 `viewport_anchor`、请求的 `requested_anchor`、解析后的
`anchor`、可空 SVG `occurrence` 及真实 `origin`。
`TreeLinkTooltipData` 共享 `TreeLinkEventData` 的完整池：配置、主题、原始来源、计算端点、
原始来源索引、计算连线索引、最终构造样式、活动与可空原始 Mark。原始值保留正负零，
聚合值单列；克隆和格式化不重放布局或样式工厂。

## 保留式 Glory 组件

组件的 `Bond<Option<TreeLinkTooltipFrame>>` 在构造时为 None，只在共同采用后公布数据。
相同 revision 保留局部状态，新 revision 请求重建。节点、连线共享一个采用边界，
两类组件各自保留状态，同时最多一个托管角色可见；只配置连线时保留原来的通用节点提示。

```rust
use glory::Widget;
use kartis::browser::{
    BrowserTreeHandle, BrowserTreeLinkTooltipOptions, BrowserTreeTooltipAnchor,
};

fn configure_links(owner: &BrowserTreeHandle) -> kartis::Result<()> {
    owner.set_link_tooltip_options(BrowserTreeLinkTooltipOptions {
        anchor: BrowserTreeTooltipAnchor::Top,
        gap: 14.,
    })?;
    owner.set_link_tooltip_component(1, |context| {
        let frame = context.frame;
        Ok(glory::ui::widget(move |scope| {
            glory::ui::text()
                .content(scope.bond(move || {
                    frame.get().map_or_else(String::new, |frame| {
                        format!(
                            "{} → {} (width {})",
                            frame.data.source_datum().label,
                            frame.data.target_datum().label,
                            frame.data.style().thickness,
                        )
                    })
                }))
                .into_widget()
        }))
    })?;
    owner.flush()
}
```

## 原生选择与顺序

可信的 SVG Link Enter/Move 在独立应用鼠标回调前预留连线提示；回调内 flush 可读取
新采用的 card，但事件 payload 仍保留本事件原已接受帧。Leave 只清除匹配 lease。
Down/Up/Click/DoubleClick 不建立或移动提示。连线提示本身不设置 node current，
也不要求用户注册空操作的公开鼠标回调。

资格来自实际可用路径、字面层顺序或显式拥有的 Glory 绑定。上层 Mesh／自定义 UI 阻挡
下层连线；省略、disabled、零宽、透明画笔与移除来源不制造自动命中。重复连线保留业务
身份，并报告真实 key/index/publication。捕获中的几何按本次坐标重新查询，不回放 Down。

显式 SVG 连线绑定是 `BoundComponent` 逻辑路线；原始 Mark 可为空，有 Mark 也不代表
当前有可见墨迹。默认 Tree 连线是 series 为空的装饰 Mark，隐藏端点节点 ID 不会隐藏
仍然绘制的连线。本 API 不改变该 series 过滤合同，也不新造 canonical link Site。

## Anchor 与错误边界

原版五项为 Top、Right、Bottom、Left、Center。新连线 API 对请求 Left／Right
按 Nivo 规则解析：先算父容器 CSS 空间 X，再与父容器实际 bounding box 宽度的一半比较，
左侧选 Right，否则 Left；请求值和结果各自可读。缩放时保留上游单位不一致的比较行为，
不将其称为完整碰撞避免。四个 corner 是固定的 Rust 扩展，旧节点 anchor 语义不变。

坐标分别处理 X/Y CSS 缩放，以局部坐标和 gap 取整，组件使用自然尺寸。rotation、skew、
perspective、视口钳制与自动 Spring 退出属于其它工作。

来源组件、SVG 层及两类提示共同采用。desired 工厂拒绝后保留 accepted 配置，
后续自动输入不持续重试失败工厂；显式配置／refresh 才重试。stop 与来源、occurrence、
binding 退休后不能发布旧 card。应用回调错误不回滚应用自己的副作用。

Glory 延后受管 DOM 写入的 fault／重新挂载边界继续保留。本功能不承诺 Canvas 连线拾取、
任意 React 子树／泛型 Datum、OS 设备认证、实时 Widget 导出、GPU 或 worker。
节点与原生事件的独立行为见[节点提示](tree-tooltip.zh-CN.md)和[原生事件](tree-events.zh-CN.md)。

## 受检配置

矩阵覆盖 Chrome、Edge、Firefox、WebKit 与 Chromium CI，两种 DPR，每个上下文六个场景。
Chromium 上下文显式使用 `--disable-gpu`，报告记录实际 launch options；该配置限定本轮
像素证据的适用范围。GPU 与 worker 继续后置。

检查包括真实 SVG 路径、两角色保留状态、来源与当前 frame 同源、自然 shell 尺寸、
九种 anchor、CSS 缩放、重排、墨迹资格、拒绝恢复与清理。新增应用回调观察实际覆盖
Move 及同步 public `flush()`；原七槽原生事件矩阵另有当前源码回归。
未新测路线与诊断失败原件的边界见验收记录。
