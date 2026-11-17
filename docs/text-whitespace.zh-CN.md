# 显式标签空白协议

[English](text-whitespace.md)

此 API 自 Kartis 0.2.0 提供。
它定义单行绘制协议，不等于 CSS 空白或段落排版引擎。

`LabelWhitespace::Legacy` 为默认值，缺省条目保留各后端的旧行为，不给普通场景
额外添加空白样式。`LabelWhitespace::Preserve` 保留首尾及连续空格；测量、绘制前，
每个 tab、CR、LF、VT、FF、NEL、行分隔符及段落分隔符分别规范化为一个 U+0020。
因此，一个 Label 中的 CRLF 会变成**两个空格**。NBSP、窄 NBSP、组合字符与 joiner
不被改写，Scene 中的原始文字也不被修改。

通过标签索引选择协议：

```rust
# use kartis::*;
# fn main() -> Result<()> {
let mut scene = ChartConfig::default().scene()?;
scene.labels.push(Label {
    x: 30., y: 40., text: "  Revenue\tforecast  ".into(),
    size: 18., color: Color(20, 40, 75, 255),
    anchor: TextAnchor::Start, rotation: 0.,
});
scene.typography.label_whitespace.insert(0, LabelWhitespace::Preserve);
scene.validate()?;
# Ok(())
# }
```

`SceneTypography::label_whitespace(index)` 将缺省索引解析为 Legacy。
`LabelWhitespace::normalize` 和 `drawable_label` 可在不改原数据的情况下执行相同规范化。
稀疏条目必须引用真实标签，并受 Scene 标签预算限制；`remap_labels` 同时重映射字体和
空白条目，被移除的标签不会留下悬空配置。

独立及保留式 SVG 仅在 Preserve 时设置保留空白的文字样式，切回 Legacy 会撤销。
Canvas 和 CPU 接收同一份规范化后的单行文字。普通 Glory Text 保持真实 Text 节点，
并按该模式使用显式保留样式。模式参与保留缓存，即使只改模式也会更新接受帧。
自定义 Canvas 后端必须声明 `supports_text_whitespace` 能力；否则 Preserve 场景在
`begin` 之前拒绝，不会静默折叠空格。

## 折行与测量

`wrap_scene_label` 及 FontRegistry 包装会自动将生成的各行设为 Preserve，并事务性
重排其余标签模式。折行器在生成单行标签**之前**把段落分隔符作为硬换行处理，因此
与直接绘制含 CRLF 的单个 Label 不同。独立 `wrap_label` 只返回行数据，不修改 Scene；
手动追加这些行时，还需给对应索引设置 Preserve。详见[测量折行指南](text-wrapping.zh-CN.md)。

`SemanticTextRequest.whitespace` 是测量身份的一部分。旧 `for_label` 生成 Legacy 请求，
`for_label_with_whitespace` 生成显式模式请求。即使源文字与字体相同，另一个模式的度量
也不能满足当前请求。字体资源变化时仍需更新 revision；实际宿主行盒和原生字体 advance
依旧由宿主决定。

AreaBump、Hierarchy、Sunburst 提供测量布局前使用的 `with_label_whitespace`。
候选公开模式供真实测量器使用，签名绑定该模式，隐藏及恢复后仍保留。直接修改受控
标签的 Scene 模式不会刷新旧测量，后续布局会拒绝；应使用布局的模式选择方法并重新测量。

## 兼容性与实际受检范围

新增公开字段包括 `SceneTypography.label_whitespace`、`SemanticTextRequest.whitespace`
及测量候选的空白字段，可能使完整 struct literal 需要迁移。SceneTypography 可使用
`Default` 后设置字段；请求和布局优先使用构造函数，其余完整 literal 需补新字段。
这里不声称与已发布 0.1.0 的所有源码完全兼容。

实际四浏览器引擎 × DPR 1/2 已通过 8 个上下文、32 个阶段的 SVG、Canvas 和普通
Glory Text 验收，包含仅模式变化。声明的门槛分别为 1 逻辑像素宽度误差及 1.5 逻辑
像素的 SVG-image/Canvas 墨迹包围盒误差；**不是**整帧 RGBA 一致性，也没有修复先前
观察到的 Firefox/CPU advance 差异。字体选择、kerning、bidi 与完整 CSS 排版仍是独立
事项；需要固定原生 shaping profile 时仍可使用所提供字体的路径导出。
