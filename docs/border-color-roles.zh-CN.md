# Bar 与 HeatMap 边框继承颜色

配置 `BarOptions.border_color_profile` 或 `HeatmapLayoutOptions.border_color_profile`，即可为已有边框复用共享 `InheritedColor` 角色。`None` 保留固定的 `border.color`；`Some(InheritedColor::Theme)` 也解析为这个固定颜色。普通默认值仍为 `border: None`，单独设置 profile 不会创建边框。宽度和端帽仍来自显式配置的 `GraphicStroke`。

下面的完整示例涵盖普通 Bar、组合文字与几何工厂的样式化 Bar，以及明确区分 null、零值和非零来源的 HeatMap。几何工厂把可见填充改为洋红色，边框继承色仍取自执行这些回调之前解析的数值色盘。

```rust
use kartis::*;
use glory::render::{GraphicStroke, StrokeCap};

fn stroke(color: Color) -> GraphicStroke {
    GraphicStroke {
        color,
        width: 2.5,
        cap: StrokeCap::Round,
    }
}
fn mark<'a>(scene: &'a Scene, id: &str) -> &'a Mark {
    scene.marks.iter().find(|mark| mark.id == id).unwrap()
}

fn main() -> kartis::Result<()> {
    let mut config = ChartConfig::default();
    config.theme.colors = vec![Color(20, 40, 60, 160)];
    let fixed = stroke(Color(9, 8, 7, 137));
    let profile = InheritedColor::FromContext(vec![ColorModifier::Darker(1.)]);
    let categories = ["A", "B"].map(str::to_string);
    let data = [BarSeries::new("sales", vec![Some(4.), Some(8.)])];

    let fixed_options = BarOptions {
        border: Some(fixed),
        ..Default::default()
    };
    let classic_fixed = bar(&config, &categories, &data, &fixed_options)?;
    let classic_theme = bar(
        &config,
        &categories,
        &data,
        &BarOptions {
            border_color_profile: Some(InheritedColor::Theme),
            ..fixed_options.clone()
        },
    )?;
    assert_eq!(classic_theme, classic_fixed);
    let options = BarOptions {
        border_color_profile: Some(profile.clone()),
        ..fixed_options
    };
    let classic = bar(&config, &categories, &data, &options)?;
    for source in classic
        .marks
        .iter()
        .filter(|mark| mark.id.starts_with("bar-"))
    {
        assert_eq!(source.stroke, Some(stroke(Color(14, 28, 42, 160))));
    }
    let no_border = bar(
        &config,
        &categories,
        &data,
        &BarOptions {
            border: None,
            ..options.clone()
        },
    )?;
    assert!(
        no_border
            .marks
            .iter()
            .filter(|mark| mark.id.starts_with("bar-"))
            .all(|mark| mark.stroke.is_none())
    );

    let style = BarStyleOptions {
        colors: [("sales".into(), Color(20, 40, 60, 160))].into(),
        labels: BarLabels {
            enabled: true,
            color_profile: Some(InheritedColor::Fixed(Color(40, 50, 60, 101))),
            ..Default::default()
        },
        ..Default::default()
    };
    let mut styled = {
        let mut label = |context: &BarLabelContext<'_>| Ok(context.default_text.to_owned());
        let mut geometry = |context: &BarDatumContext<'_>| {
            assert_eq!(context.color, Color(20, 40, 60, 160));
            assert_eq!(context.stroke, Some(stroke(Color(14, 28, 42, 160))));
            let mut node = context.inherit(context.geometry.clone());
            node.fill = Some(Color(240, 30, 180, 255));
            node.opacity = 0.5;
            if context.category == "B" {
                node.stroke = Some(stroke(Color(1, 2, 3, 80)));
            }
            Ok(node)
        };
        bar_layout_with_factories(
            &config,
            &categories,
            &data,
            &options,
            &style,
            BarFactories {
                bar: Some(&mut geometry),
                label: Some(&mut label),
            },
        )?
    };
    let styled_base = styled.layout.scene.clone();
    let active_id = styled.layout.bars[0].id.clone();
    let second_id = styled.layout.bars[1].id.clone();
    assert_eq!(
        mark(&styled_base, &active_id).stroke,
        Some(stroke(Color(14, 28, 42, 160)))
    );
    assert_eq!(
        mark(&styled_base, &second_id).stroke,
        Some(stroke(Color(1, 2, 3, 80)))
    );
    let first_label = styled.layout.bars[0].label_index.unwrap();
    let second_label = styled.layout.bars[1].label_index.unwrap();
    assert_eq!(styled_base.labels[first_label].color, Color(40, 50, 60, 51));
    styled.set_active(Some(&active_id), 0.2)?;
    assert_eq!(mark(&styled.layout.scene, &second_id).opacity, 0.1);
    assert_eq!(
        mark(&styled.layout.scene, &second_id).stroke,
        mark(&styled_base, &second_id).stroke
    );
    assert_eq!(
        styled.layout.scene.labels[second_label].color,
        Color(40, 50, 60, 10)
    );
    styled.set_active(None, 0.2)?;
    assert_eq!(styled.layout.scene, styled_base);
    let _bar_binding = styled.interaction_binding()?;

    let heat_data = [
        HeatCell {
            row: "row".into(),
            column: "empty".into(),
            value: None,
        },
        HeatCell::new("row", "zero", 0.),
        HeatCell::new("row", "value", 10.),
    ];
    let heat_options = HeatmapLayoutOptions {
        border: Some(fixed),
        border_color_profile: Some(profile),
        axes: HeatmapAxes::none(),
        hover_target: HeatmapHoverTarget::Cell,
        colors: ColorScaleOptions {
            domain: Some([0., 10.]),
            colors: vec![Color(20, 40, 60, 160), Color(200, 180, 160, 220)],
            unknown: Color(90, 100, 110, 64),
            ..Default::default()
        },
        label_color_profile: Some(InheritedColor::Fixed(Color(40, 50, 60, 101))),
        empty_label: "NA".into(),
        opacity: 0.8,
        active_opacity: 0.9,
        inactive_opacity: 0.2,
        ..Default::default()
    };
    let heat = heatmap_layout_with_cell_factory(&config, &heat_data, &heat_options, |context| {
        let (palette, border_color) = match context.datum.value {
            None => (Color(90, 100, 110, 64), Color(63, 70, 77, 64)),
            Some(0.) => (Color(20, 40, 60, 160), Color(14, 28, 42, 160)),
            Some(_) => (Color(200, 180, 160, 220), Color(140, 126, 112, 220)),
        };
        assert_eq!(context.color, palette);
        assert_eq!(context.stroke, Some(stroke(border_color)));
        let mut node = context.inherit(context.geometry.clone());
        node.fill = Some(Color(240, 30, 180, 255));
        node.opacity = 0.5;
        Ok(node)
    })?;
    assert_eq!(
        heat.cells
            .iter()
            .filter(|cell| cell.value.is_none())
            .count(),
        1
    );
    let zero = heat
        .cells
        .iter()
        .find(|cell| cell.value == Some(0.))
        .unwrap();
    assert_eq!(zero.tooltip_datum().value, Some(0.));
    assert_eq!(
        mark(&heat.scene, &zero.mark_id).stroke,
        Some(stroke(Color(14, 28, 42, 160)))
    );
    assert_eq!(mark(&heat.scene, &zero.mark_id).opacity, 0.4);
    assert_eq!(
        heat.scene
            .labels
            .iter()
            .find(|label| label.text == "0")
            .unwrap()
            .color,
        Color(40, 50, 60, 40)
    );
    let active = heat.with_active(Some(HeatmapCellKey::new("row", "empty")))?;
    assert_eq!(mark(&active.scene, &zero.mark_id).opacity, 0.1);
    assert_eq!(
        mark(&active.scene, &zero.mark_id).stroke,
        mark(&heat.scene, &zero.mark_id).stroke
    );
    assert_eq!(
        active
            .scene
            .labels
            .iter()
            .find(|label| label.text == "0")
            .unwrap()
            .color,
        Color(40, 50, 60, 10)
    );
    assert_eq!(active.with_active(None)?.scene, heat.scene);
    let _heat_binding = heat.interaction_binding()?;
    let _classic_svg = render::to_svg(&classic)?;
    let _styled_svg = render::to_svg(&styled.layout.scene)?;
    let _heat_canvas = render::to_canvas_script(&heat.scene, "heatmap")?;
    Ok(())
}
```

对 Bar，色盘 RGBA `(20,40,60,160)` 经 `Darker(1)` 得到边框 RGBA `(14,28,42,160)`。几何工厂的 `0.5` 因子保留在 `Mark.opacity` 中，不会把保存的边框 alpha 改为 80。非活动因子 `0.2` 将标记透明度改为 `0.1`，边框 RGBA 保持不变。自有标签使用独立的固定 RGBA `(40,50,60,101)`，基础帧最终 alpha 为 51，非活动时为 10。清除活动 ID 会恢复已接受的基础帧。

组合标签与几何工厂的路径，与普通布局及仅几何工厂的 Bar 路径接收相同的已解析描边。`BarLabelContext` 提供原始来源和文字；配套的 `BarDatumContext` 提供 `stroke`、`color` 和 `inherit`。显式返回自定义描边优先，示例中的 B 分类展示了这种覆盖。自定义纯色填充以及渐变、图案的代表色都不会反向改变边框角色。原始值、tooltip 文字、标签归属、逻辑交互位置、系列与分类身份，以及既有裁切和零长度策略，与颜色覆盖独立。尺寸、隐藏状态、数据或 profile 改变时需重建布局；悬停不会重新调用工厂。

对 HeatMap，显式 null 来源使用数值色标的 `unknown` 颜色，零值使用真实的零端点；不会补造稀疏数据中不存在的配对。示例中三个边框颜色保留 alpha 64、160 和 220，同时工厂因子 `0.5` 与基础透明度 `0.8` 合成标记透明度 `0.4`。激活 null 单元格后，零值单元格的非活动透明度为 `0.1`，零值标签 alpha 从 40 变为 10。null 单元格自身使用活动透明度 `0.9 × 0.5 = 0.45`。该角色不会为既有布局已抑制描边的零尺寸 HeatMap 图元重新添加描边。

`FromContext` 以各路径在工厂执行前解析的数值色盘同时作为 `inherited` 和对比度的 `surface`。Bar 保留普通或配置式的系列/分类色盘选择，HeatMap 保留数值色标与未知值颜色。已有描边时，`Theme` 取该描边的固定颜色，并非文字主题，也不是任意 Nivo 主题属性路径。图表背景作为对比度的 backdrop。`Fixed` 以及有序的 `Brighter`、`Darker`、`Opacity` modifier 复用现有类型化颜色实现。`ColorModifier::Opacity` 替换角色颜色的 alpha，不替换标记透明度。边框与标签 profile 是两个独立角色。

即使没有边框、数据已隐藏或标签已禁用，profile 的内在配置也会在工厂回调前校验。依赖实际上下文的检查还会预先覆盖真实来源，包括隐藏、裁切和零值来源。Bar 的 `None` 没有柱形表面；HeatMap 的显式 null 单元格则有未知值颜色表面。真正的空来源只校验配置，不补造色盘上下文。非法 profile 或实际上下文会在回调前返回错误，不产生已接受帧；其他后续失败不会撤销回调已造成的外部副作用。

`Contrast` 是 Rust 扩展。它根据数值色盘表面与图表背景，在两个候选色中选择，要求最终背景不透明；这不保证任意自定义画笔的每个像素都符合对比度要求，也不表示 Nivo 有同名配置。类型化 RGBA 在 alpha 为零时保留隐藏的 RGB 通道；Nivo 通过 d3-color 解析 alpha 为零的 CSS 颜色时，可能在执行 modifier 前丢失这些通道。后续 `Opacity` 将 alpha 恢复为非零时，差异会变为可见颜色：原 CSS 路径可能恢复黑色，而 Rust 保留并恢复原 RGB。参考值保留了这项差异，本 API 沿用 Rust 现有类型化颜色行为，不修改共享解析器或算法。

Normal 混合下，`Mark.opacity` 应用于整个标记。对于同时可能绘制填充和描边、且标记透明度介于零和一之间的图元，Canvas 与 CPU raster 先在有界透明源中绘制，保留各画笔颜色自身的 alpha，再按标记透明度整体合成一次，与 SVG 和 Glory 的对象透明度语义一致。仍沿用既有临时像素和逐帧工作预算，目标裁剪只在合成时应用一次。单次绘制输入、归一化点描边及零面积基本图元保留直接绘制路径。普通 Path 的潜在填充按保守规则判断，仍可能需要隔离，因此不能笼统承诺所有最终只产生一种墨迹的路径都保持原 Canvas 输出字节。

自定义 `Canvas` 适配器必须显式启用默认值为 `false` 的 `supports_group_opacity`，并同时实现 `prepare_mark_blends`、`begin_mark_blend` 与 `end_mark_blend`。契约包括：在 `begin` 前检查有界资源，在透明源中以单位标记透明度绘制，然后在目标裁剪下整体合成一次，或在失败时丢弃源。Normal 帧需要这项能力但适配器不支持时，会在调用准备钩子和 `begin` 之前拒绝，不接触目标。

未配置边框 profile 时，固定边框布局、业务元数据和 SVG 保持兼容。对象透明度的渲染修复会改变受影响的已有标记的 Canvas 代码和 CPU 像素；修复前仅针对颜色角色的比较，不能证明最终 Canvas/CPU 输出仍逐字节相同。继承色计算与 Nivo 一致，也不表示其原有 Canvas 和 SVG 透明度行为一致：原 HeatMap Canvas 路径同样存在透明度差异。颜色角色与对象合成是两个独立契约。

Glory `Chart` 现在通过真实的 overflow-hidden 容器支持 `Scene.label_clips` 中的 `Geometry::Rect`。原始标签文字、来源坐标、锚点、字体、RGBA 与业务身份保持不变，只平移容器内的坐标原点。仅修改裁剪框，以及在有裁剪和无裁剪之间切换，都会响应更新；无裁剪标签保留原有结构。裁剪不会创建交互命中区域。Circle 和 Path 标签遮罩仍不支持，非法响应式提案会保留已接受帧。普通文字仍要求零旋转；`outline_labels` 与 `painted_labels` 在使用相同矩形容器时，继续遵守原有字体、旋转与字形预算规则。

选项结构体可使用 `..Default::default()`。`GraphicStroke` 没有 `Default` 实现，需显式提供 `color`、`width` 与 `cap`。[标签继承颜色](inherited-label-colors.zh-CN.md) · [类型化标签](typed-labels.zh-CN.md) · [自定义 Bar 几何](bar-custom-geometry.zh-CN.md) · [自定义 HeatMap 单元格](heatmap-custom.zh-CN.md)
