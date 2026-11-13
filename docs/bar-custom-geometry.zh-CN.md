# 自定义 Bar 几何与绘制样式

Typed Bar factory 为每个已计算的柱体生成一个与渲染器无关的 `Scene` 标记，复用 Scatter/Line 自定义点的 `Geometry`、`FillPaint`、描边、虚线与锚点契约。既有 Bar 入口保留默认行为；工厂接口的集成和验收状态另见[验收记录](verification.md)。

```rust
use kartis::*;
use kartis::scales::BandScaleOptions;

fn main() -> kartis::Result<()> {
    let config = ChartConfig {
        width: 420.,
        height: 260.,
        ..Default::default()
    };
    let categories = ["A", "B", "C"].map(str::to_string);
    let series = [BarSeries::new("sales", vec![Some(4.), None, Some(0.)])];
    let style = BarStyleOptions {
        category_scale: Some(BandScaleOptions {
            padding_inner: 0.2,
            padding_outer: 0.2,
            round: true,
            ..Default::default()
        }),
        labels: BarLabels {
            enabled: true,
            clip_to_plot: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut frame = bar_layout_with_bar_factory(
        &config,
        &categories,
        &series,
        &BarOptions::default(),
        &style,
        |context| {
            let [x, y] = context.position;
            let glyph = glyph_geometry(
                [x + 18., y],
                &GlyphOptions {
                    shape: GlyphShape::Filled(FilledGlyph::Diamond),
                    size: 280.,
                    ..Default::default()
                },
            )?;
            let mut node = context.inherit(glyph.geometry);
            node.anchor_commands = Some(glyph.anchor_indices);
            node.opacity = 0.8;
            node.fill_paint = Some(FillPaint::Linear {
                start: [x + 8., y],
                end: [x + 28., y],
                stops: vec![
                    GradientStop::new(0., Color(230, 60, 20, 255)),
                    GradientStop::new(1., Color(20, 80, 220, 255)),
                ],
            });
            Ok(node)
        },
    )?;
    let binding = frame.interaction_binding()?;
    assert_eq!(binding.sites.len(), 2);
    assert!(binding.sites.iter().any(|site| site.value == Some(0.)));
    if let Some(id) = binding.sites.first().map(|site| site.mark_id.clone()) {
        frame.set_active(Some(&id), 0.25)?;
        frame.set_active(None, 0.25)?;
    }
    let _svg = render::to_svg(&frame.layout.scene)?;
    let _canvas_script = render::to_canvas_script(&frame.layout.scene, "bars")?;
    Ok(())
}
```

示例把可见菱形向数据站点右侧偏移 18 个逻辑像素。生成的菱形是 Path，因此逻辑站点仍位于 `context.position`。`Some(0.)` 仍调用工厂，并在绑定中保留真实零值；`None` 不产生柱体、回调或站点。数值标签仍遵循原有显示条件：替换几何不会强制原本零高度的柱体显示数值标签。

## 源数据与几何契约

`bar_layout_with_bar_factory` 返回 `CustomBarLayout`，其公开 `layout` 是普通 `StyledBarLayout`。`bar_layout_with_scale_options_and_bar_factory` 返回附带已解析值尺度的 `ScaledCustomBarLayout`。两者与原有 Bar 函数共用数值布局、类别映射、裁剪、源身份及区间计算；[类别带状尺度配置](bar-category-scale.zh-CN.md) 也适用于这些入口。

Context 借用原始 `BarSeries` 和类别，提供稳定 mark ID、原始系列/类别索引、原值、显示值、区间、类别槽位/宽度、可见矩形、原有默认几何、继承颜色/边框和 plot。无法表示未裁剪矩形时 `original_rect` 为 `None`；`endpoints` 分别保留可表示的有限、未钳制值投影。这些信息与可见矩形分开。回调沿用既有可见系列的字典序和作者类别顺序，并保留原始索引。隐藏系列、缺失值、完全裁掉的柱体和零带宽槽位不调用工厂。继承操作复制原有纯色填充和边框，opacity 为 1；自定义 paint 不改变数值域或原值。

逻辑锚点在自定义墨迹生成前固定：普通 Rect 使用自身中心，普通几何为 Path 时使用可见矩形中心。零面积圆角 Rect 占位几何在类别裁剪后仍保留其作者中心。这个锚点不冒称未裁剪源区间的中点；未裁剪信息请读取 `original_rect` 和 `endpoints`。不会根据自定义标记的包围框重新推断 `context.position`。

自定义 Rect 和 Circle 必须以这个逻辑位置为中心，并遵守共享的受检中心舍入规则。偏移墨迹、空心形状、多轮廓或不对称外形请使用 Path。可选 `anchor_commands` 沿用 Scatter/Line 受检 Path 端点支持规则，兼容动画按这些端点的平均位移跟随；已接受帧使用精确的给定逻辑位置。`Scene` 实际命中测试使用真实几何、描边/虚线及 plot 裁剪，最近点绑定则使用逻辑站点。逻辑站点不是隐形填充命中矩形。

## Paint、标签与受控 hover

工厂负责一个标记的几何、纯色填充、线性/径向渐变或图案、描边、虚线、opacity 和可选 Path 锚点支持。布局负责 mark ID、系列、原始值、提示文字和 plot 裁剪。既有数值标签与合计布局继续负责位置、格式、跳过条件和源归属。柱体数值标签的 alpha 为配置的原始 alpha 乘工厂 opacity；坐标轴、图例和合计标签仍各自独立。`StyledBarDatum.color` 保留数值色盘颜色，typed 交互站点使用已准备 paint 的代表色。

`set_active(Some(mark_id), inactive_factor)` 将非活动柱体的原始 opacity 乘给定因子，活动柱体保留原始 opacity。因子必须为 [0,1] 内有限值，未知 ID 返回 `Err`。`set_active(None, factor)` 恢复基础帧。每次更新都从原始配置颜色和基础 opacity 重算数值标签 alpha，避免连续累乘或舍入；paint、几何、源值和裁剪保持原样。

自定义布局私有保存完整已接受帧、源站点和锚点。`interaction_binding()` 与 `set_active()` 共用这份私有帧的校验。直接修改公开几何、paint、源 metadata、opacity、标签、`label_series`、`label_clips` 或绘制顺序不是支持的更新方式，会在发布新的受控帧前报错。Hover 更新先验证完整候选，再替换当前帧。Resize、隐藏系列、新数据或样式变化应重新调用工厂布局，以重算槽位、区间、合计和站点，并保留稳定源身份。仅过滤 Scene 的通用图例功能不能完成这些数值重布局。

## 范围与限制

这是单个 typed 标记的几何/paint 回调，不是任意 Glory Widget 插槽、React `barComponent`、Canvas context 回调或子控件生命周期。布局不保存应用回调供以后调用；应用将已接受 Scene 与绑定接入现有宿主，并显式驱动受控 hover。

工厂返回错误或非法输出时拒绝整个候选布局，既有有限坐标和 paint 规则仍适用。准入累计既有坐标轴与全部自定义输出：源 Path 命令最多 200,000，渐变 stops 最多 16,384；共享虚线展开最多生成 200,000 个命令/展平线段、消耗 4,000,000 工作量。保留下一个回调结果前先检查虚线预算，最终仍由 `Scene` 校验共享几何、裁剪和渲染契约。这些限制约束返回资源，不沙箱化调用者 Rust 代码，也不拦截回调 panic。
