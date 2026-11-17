# 真实测量驱动的自动折行

[English](text-wrapping.md)

这些 API 自 Kartis 0.2.0 提供。
自动折行属于 Rust 扩展；原 Nivo 的普通 SVG/Canvas 文字仍是单行入口。
本实现不等于完整 CSS 排版或完整 UAX #14 断行。

`wrap_label` 接收 Label、解析后的字体、`TextWrapOptions` 和借用的真实测量回调。
`wrap_scene_label` 仅在整份候选测量、重建并校验成功后替换一个 Scene 标签。
返回的 `WrappedLabelLines` 包含可直接绘制的各行 Label、绑定原请求的度量及截断状态。

启用 `raster` 后，`FontRegistry::wrap_label` 和
`FontRegistry::wrap_scene_label` 使用实际字体字节和 OpenType shaping 测量；最后一个参数
是 `MissingGlyphPolicy`。注册、绑定所需字体后可以执行：

```rust
# use kartis::*;
# use kartis::render::raster::*;
# fn apply(scene: &mut Scene, fonts: &FontRegistry) -> Result<()> {
let options = TextWrapOptions {
    max_width: 120.,
    max_lines: 4,
    overflow: TextWrapOverflow::Ellipsis,
    ..Default::default()
};
let lines = fonts.wrap_scene_label(scene, 0, &options, MissingGlyphPolicy::Error)?;
assert!(lines.labels.len() <= 4);
# Ok(())
# }
```

单词和字素模式均保留完整 Unicode 扩展字素。空白折叠/保留、长词、显式换行、行数上限及
错误/裁去/省略号溢出均显式配置。宽度约束针对真实 shaping **advance**，不保证字体墨迹
或旋转包围盒完全在宽度内；bearing 和下伸部保留在每行的真实 ink bounds 中。
单个字素过宽时遵守长词策略，不拆成无效的 Unicode 片段。

`line_height` 以字号比率显式指定相邻基线间距，旋转文字沿旋转后的块方向移动后续基线。
FontRegistry 适配器使用主选择字体真实的 ascent/descent 和居中 leading 构建所请求的行盒；
这是字体定义的度量，不是 HTML/CSS 行盒测量。显式 family/weight 与 CPU 绘制共用选择规则；
未声明字体的 Scene 保留原注册顺序。空字体注册表即使面对空行也返回错误，且不提交半帧。
不会查询系统字体或合成粗体。

Scene 替换同步重建各标签字体、series 归属、clip 与显式绘制顺序。接受后的文本是各行原始
文字；需要完整原段落的应用必须另行保存源文本，并在数据、宽度、字体或资源 revision
变化后从原文本重新排版，不能反复折行已缩短的结果。测量次数/字节预算限制重复 shaping，
整份 Scene 和复制后的 clip 预算在提交前验证。拒绝任何候选都保持原 Scene，回调的外部副作用
不提供撤销保证。

各行可直接经过已有 SVG、Canvas、CPU、Glory 标签路径绘制。还可用
`scene_with_painted_labels` 输出与系统字体无关的路径副本，原文字 Scene 用于编辑与可访问性。

自动竖排折行明确拒绝。水平 `raster-bidi` 对每行逻辑文字重新 shaping，每行视为独立 bidi
段落，`Auto` 也会重新判断方向；不声称实现“整段 UBA 后按 CSS 断行”的完整语义。
普通 SVG/Canvas 文字仍受浏览器字体和 bidi 行为影响，路径导出使用所提供字体的明确原生 profile。
`wrap_scene_label` 现在自动给生成行设置显式 `LabelWhitespace::Preserve` 协议，独立/保留式
SVG、Canvas、CPU 和普通 Glory Text 会保留规范化后的单行空格，原 Label 字符串仍保留。
独立 `wrap_label` 不修改 Scene，手动追加结果时须给相应索引设置 Preserve。每个分隔标量
分别映射一个空格（CRLF 为两个）；折行器在生成各行之前处理段落硬换行。这不是 CSS tab-stop
布局，也不修改全局宿主 CSS。测量身份、后端能力、源码迁移和实际浏览器门槛见
[显式空白协议](text-whitespace.zh-CN.md)。原生字体 advance 仍可不同；路径导出提供固定的
原生 shaping profile，不以此证明普通文字像素完全相同。

`text_wrapping` 可执行示例输出实际 SVG/Canvas/CPU。完整 CSS/段落排版、自动竖排列和更广宿主/
设备验收仍留在差距清单中。
