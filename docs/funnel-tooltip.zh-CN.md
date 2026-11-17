# Funnel 提示框（Kartis 0.2.0）

[English](funnel-tooltip.md)

此 SVG API 自 Kartis 0.2.0 提供，并已通过原生事件与 tooltip 浏览器验收：10 个配置、40 个 profile、
2,000 帧、160 次拒绝及 20 个真实 DOM 负控。产物与 Chromium 固定 `--disable-gpu`
测试配置见[验收记录](verification.md#development-funnel-native-events-and-tooltips)。
它扩展专用 `BrowserFunnel` owner，不代表通用 Canvas 图表或全部 Nivo Funnel 功能已经完成。

## 显式启用

`BrowserFunnel` 与 `BrowserFunnelHandle` 提供 `set_tooltip_options`、
`set_tooltip_component`、`clear_tooltip_component`、`reset_tooltip` 和读取已接受
数据的 `tooltip`。必须保留 owner；handle 是弱引用。setter 排队，`flush()` 处理
同一来源、图层和提示框事务。原生鼠标回调内不允许 flush，排队内容在回调后处理。

```rust
use kartis::browser::{
    BrowserFunnelHandle, BrowserFunnelTooltipAnchor, BrowserFunnelTooltipOptions,
};

fn enable_tooltip(owner: &BrowserFunnelHandle) -> kartis::Result<()> {
    owner.set_tooltip_options(BrowserFunnelTooltipOptions {
        enabled: true,
        anchor: BrowserFunnelTooltipAnchor::Top,
        gap: 14.,
    })?;
    owner.flush()
}
```

既有 mount 默认没有 managed tooltip。设置组件会同时启用默认选项。只设置选项时，
标准提示显示色块、作者 label、冒号和已计算的 formatted value。空 label 仍显示
`: value`，不回退为业务 ID。所有文字通过 DOM 文本节点插入，HTML 字符不会成为
标记。`FunnelDatum.label` 为 `String`，本接口不保留原始对象中字段缺失与空字符串
的区别。清除组件保留选项并返回标准提示；reset 移除管理并清选择。
`enabled: false` 隐藏提示，但已配置组件可以保留 Scope。

## 完整数据与真实组件

`FunnelTooltipFrame` 包含不可变 `FunnelSvgFrame`、`part_index`、`source_index`、
已接受的 `source_generation` 和 `publication`、真实 Parts occurrence 的
key/index/publication、`pointer_client`、逻辑 `viewport_anchor`、
`requested_anchor` 与解析后的 `anchor`。`part()`、`datum()`、`style()` 是检查型
访问器，分别给出计算元数据、原始数据和构造时捕获的样式。当前 Scene 墨迹另行保留；
读取和克隆提示数据不重放布局、颜色或格式函数。

```rust
use glory::Widget;
use kartis::browser::BrowserFunnelHandle;

fn install_component(owner: &BrowserFunnelHandle) -> kartis::Result<()> {
    owner.set_tooltip_component(1, |context| {
        let frame = context.frame;
        Ok(glory::ui::widget(move |scope| {
            glory::ui::text()
                .content(scope.bond(move || {
                    frame.get().map_or_else(String::new, |frame| {
                        frame.text().unwrap_or_else(|error| error.to_string())
                    })
                }))
                .show_in(scope)
        }))
    })?;
    owner.flush()
}
```

这是拥有真实 retained Glory Widget 的接口；上下文提供
`Bond<Option<FunnelTooltipFrame>>`。构造期值为 `None`，构造回调不是已接受来源通知。
共同 adopt 在 pump 各 Runtime 前更新来源、图层与提示框 Cages。每个 Runtime 拥有完整
Cage；诊断时跨 Runtime 读取 Bond 应用 untracked 方式，不跨 Runtime 订阅 effect。
相同组件 revision 保留 Scope 和本地状态，即使新传入另一个 factory；新 revision
请求替换。

## 原生选择与定位

实际 owned body 的 Enter/Move 在应用 hook 前预留提示选择。Leave 只清除匹配的
occurrence/body lease；Click 不创建或移动提示。无需注册空的公共 mouse handler。
事件 `isTrusted` 原样保留，不仅因事件为 synthetic 就拒绝实际已连接 owned DOM 输入。

若未由应用的 mouse handler 显式选择 Owner policy，启用提示会使用 native-component
活动策略：Enter 排队 current 与提示，Move 只改提示，Leave 排队清除二者。
手动 current 与提示选择独立。不会创建虚假 scalar site、companion Mark，也不会把
组件点击伪称为原生 body 命中。

省略 Parts、移除 body、隐藏 series 都不能继续保留自动提示。重复图层报告实际
occurrence。原始事件目标由浏览器 SVG `visiblePainted` 决定，不是只支持纯色或
非零 alpha 的像素判定；透明 SVG paint 仍可能成为原生 pointer target。边框、分隔线
和原生 label 保持被动。上方自定义图层可以截获输入。来源或程序提交会按真实候选 plan
重新验证选择并刷新 generation/publication；只有指针 Move 时不重建 SVG 来源或提高
publication。

提示跟随真实 cursor。默认 Top，间隔 14 个本地 CSS 像素。共享九锚点包括 Center 和
四角。请求 Left/Right 按共享 provider 的容器水平半区规则解析方向；Top、Bottom、
Center 与四角固定。这不是视口避障。每个物理 CSS 轴独立映射回父容器，随后本地坐标
取整并按自然尺寸进行百分比平移，支持轴对齐非均匀 CSS 缩放，不承诺旋转或斜切祖先。
提示 overlay 独立于裁剪的 SVG 内容。

## 拒绝与生命周期

来源、图层和提示框的准备、owned carrier 移动与已接受数据共享事务。构建器、factory
或 precommit owned DOM 写入被拒绝时保留先前配置、来源和提示；失败的 desired 配置
不能污染后续原生 Move/Leave。应用回调自身的外部副作用不能回滚。

标准文字不创建 Glory Runtime。自定义提示与自定义图层共享组件预算，计算 live、
pending、retiring 记录。保留的组件只计一次，替换时暂时同时计算旧、新记录。
共同数据 adopt 后出现 managed backend/effect 错误属于 postcommit UI 错误。
失败提示隐藏并设为 inert，保持锁定，直到新 revision 替换或重新 mount；不把已接受
来源伪称为原生 backend 已回滚。stop 退役 owned presentation，弱 handle 拒绝后续操作。

本功能不新增 Canvas 提示、来源 item 组件、annotations、Spring 动画、泛型 raw datum
或任意可变 D3 generator。
