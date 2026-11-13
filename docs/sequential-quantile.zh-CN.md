# 在连续色带上使用样本排名

[English](sequential-quantile.md) · [库概览](../README.zh-CN.md)

`ColorScaleKind::SequentialQuantile` 将观测分布映射到现有连续颜色插值器或命名色带，
保留所有重复观测。输入到排名的映射是阶梯函数：颜色插值器连续，
但两个观测之间的数值不会获得线性插值的排名。
这是 D3 样本排名配置，不是新增的 Nivo 配置属性，也不是归一化直方图。

## 排名、分位统计和序号颜色

对于长度 `n >= 2` 的已排序样本 `d`，有限查询使用
`(bisectRight(d, x, 1) - 1) / (n - 1)`。
等值查询取重复组的最后一个排名，低于或高于样本范围的有限查询饱和到 0 或 1。
它不同于 `count(d <= x) / n`，也不同于 R7 分位插值。
对 `[0, 0, 5, 10, 10]`，0、5、10 的排名分别是 0.25、0.5、1。

```rust
use kartis::*;

fn main() -> Result<()> {
    let scale = ColorScaleOptions {
        kind: ColorScaleKind::SequentialQuantile,
        colors: vec![Color(0, 0, 0, 255), Color(255, 255, 255, 255)],
        ..Default::default()
    }.resolve([Some(0.), Some(0.), Some(5.), Some(10.), Some(10.), None])?;
    assert_eq!(scale.sample_domain(), Some([0., 0., 5., 10., 10.].as_slice()));
    assert_eq!(scale.rank(0.), Some(0.25));
    assert_eq!(scale.rank(4.), Some(0.25));
    assert_eq!(scale.quantiles(4)?, Some(vec![0., 0., 5., 10., 10.]));
    assert_ne!(scale.rank_colors().unwrap()[0], scale.map(Some(0.)));
    Ok(())
}
```

`sample_domain()` 返回私有保留的完整已排序分布。相等值保留输入顺序，包括正负零的位表示。
克隆共享不可变存储，修改原始输入向量不会改变已解析色标。
最多接受 20,000 个输入条目；`None` 计入输入预算，但不计入分布。

`quantiles(segments)` 在概率 `0..=segments` 除以 `segments` 的位置计算 R7 统计，
返回含最小值和最大值的 `segments + 1` 个值。合法数量为 1 到 256。
非法数量在计算前拒绝，对空分布和其他色标种类也一样。
原 R7 运算产生非有限结果时返回 `Err`，即使同一批有限输入的排名查询仍然有效。

`rank_colors()` 对每个已排序观测按序号位置 `i / (n - 1)` 采样色带。
它有意不同于逐个样本调用 `map`，因为 `map` 对重复值使用最右排名。
`palette()` 仍表示作者指定或预备好的色带锚点。
`bins()` 为空，`bin_extent` 和 `invert_extent` 返回 `None`；
此种类没有离散颜色桶，也没有唯一的颜色反算。

| 分布 | `rank` / `map` | `sample_domain` | `rank_colors` | 合法数量的 `quantiles` |
| --- | --- | --- | --- | --- |
| 空分布 | `None` / unknown | `Some([])` | `Some([])` | `Ok(None)` |
| 单样本 | `None` / unknown | 单个样本 | 单个 unknown 颜色 | 重复的有限样本 |
| 至少两个样本 | 最右排名 / 色带 | 完整分布 | 按序号采样色带 | R7 值或溢出错误 |
| 其他色标种类 | `None` / 自身映射 | `None` | `None` | `Ok(None)` |

两个或更多样本全部相同时，低于该值的查询排名为 0，等于或高于该值时为 1。
这不使用普通 Sequential 常值域的中点颜色规则。

## 显示窗口、缺失值和受检统计

此种类要求 `ColorScaleTransform::Linear`、空 `thresholds` 列表，
以及升序、有限的 `domain` 显示窗口；允许常值窗口。
作者配色在所选插值空间中接受 2 到 256 个颜色锚点，
也可通过 `resolve_with_profile` 选择命名色带。
完整样本分布独立于这个双端点窗口。未指定窗口时使用观测范围；
空输入沿用现有 `[0, 1]` 后备范围。可反转颜色锚点来改变色带方向，
但这不会反转已排序分布。

`rank` 在窗口外仍是分布统计。默认 `clamp=true` 时，`map` 在作者窗口外仍使用该排名。
使用作者配色插值且 `clamp=false` 时，`map` 对有限窗口外的值返回 unknown。
这是 Kartis 显式的有限窗口扩展，不是 D3 SequentialQuantile 的 clamp 方法。
命名色带仍要求现有的 `clamp=true`。

`position(value)` 和 `value_at(offset)` 描述线性**数据轴**及其受检反算，
不返回排名，也不反算排名。因此 `rank(value_at(p))` 不保证等于 `p`，
`map(quantiles(...)[i])` 也不保证等于对应分位概率处的色带颜色。

```rust
use kartis::*;

fn main() -> Result<()> {
    let options = ColorScaleOptions {
        kind: ColorScaleKind::SequentialQuantile,
        domain: Some([2., 8.]),
        clamp: false,
        ..Default::default()
    };
    let scale = options.resolve([Some(0.), Some(0.), Some(5.), Some(10.), Some(10.)])?;
    assert_eq!(scale.rank(0.), Some(0.25));
    assert_eq!(scale.map(Some(0.)), scale.unknown());
    assert_eq!(scale.position(5.), Some(0.5));
    assert_eq!(scale.value_at(0.5)?, 5.);

    let single = options.resolve([Some(7.)])?;
    assert_eq!(single.map(Some(7.)), single.unknown());
    assert_eq!(single.quantiles(2)?, Some(vec![7., 7., 7.]));
    let extremes = options.resolve([Some(-f64::MAX), Some(f64::MAX)])?;
    assert_eq!(extremes.rank(0.), Some(0.));
    assert!(extremes.quantiles(2).is_err());
    Ok(())
}
```

非缺失的源样本必须有限。`None` 不贡献观测；缺失和非有限查询映射到配置的 unknown 颜色，
非有限排名查询返回 `None`。不进行字符串、布尔或对象的隐式转换。
原 D3 的空域将 -0 传给插值器，单样本域传入 NaN。
Kartis 的 unknown 处理属于明确的受检差异，不取决于 JavaScript 插值器碰巧怎样绘制这些参数。

## 精确数据轴图例有独立准入限制

将已接受的色标传给 `append_color_legend`。`stops(steps)` 在真实数据轴位置，
用有序的重复 offset 保留每次有效颜色跳变。
`steps` 仍须为 2 到 256，但不会截断或平滑掉跳变。
相邻且 RGBA 完全相同的平台可以合并，保留的边界不会移动。
使用一个渐变矩形，不把色带拆成可能产生抗锯齿接缝的小条。

```rust
use kartis::*;

fn main() -> Result<()> {
    let scale = ColorScaleOptions {
        kind: ColorScaleKind::SequentialQuantile,
        colors: vec![Color(0, 0, 0, 255), Color(255, 255, 255, 255)],
        ..Default::default()
    }.resolve([Some(0.), Some(10.)])?;
    let mut scene = ChartConfig::default().scene()?;
    let legend = append_color_legend(&mut scene, &scale, &ColorLegendOptions {
        tick_values: Some(vec![0., 10.]),
        ..Default::default()
    })?;
    assert_eq!(legend.stops.iter().map(|stop| stop.offset).collect::<Vec<_>>(), vec![0., 1., 1.]);
    assert_eq!(legend.stops[1].color, Color(0, 0, 0, 255));
    assert_eq!(legend.stops[2].color, Color(255, 255, 255, 255));
    Ok(())
}
```

上例最后的白色只存在于端点，没有正宽度的白色带，也不保证最后一个物理像素变白。
空分布和单样本绘制纯 unknown 色条；常值窗口使用该位置实际映射的颜色。

每个 paint 最多 **512 个 stops**，每个 Scene 的 stops 总数最多 **16,384**。
精确跳变必须在 f64 offset 和后端 f32 位置中都保持独立且有序。
内部跳变若舍入到端点，或与另一有效跳变坍缩重合，会拒绝；普通 f32 舍入本身不是错误。
接口不会提高预算、下采样观测或加宽色带来强行通过图例准入。
包含 20,000 个样本的分布仍可正常映射颜色，而其可选的精确图例可能无法绘制。
`append_color_legend` 事务式拒绝并保留旧 Scene。

自动图例刻度使用数据轴。若要使用 R7 标签，可获取 `quantiles`、去除重复值、
保留窗口内的值，再传入不超过 32 个 `tick_values`。
百分位排名轴属于不同坐标合同，不会因为使用这些标签就自动建立。

## HeatMap、Calendar 和共享色标 Choropleth

将 `HeatmapLayoutOptions::colors` 传给 `heatmap_layout`，
将 `CalendarLayoutOptions::colors` 传给 `calendar_layout`，
或使用显式的 `choropleth_with_color_scale` / `choropleth_with_color_profile` 入口。
较早的独立 `choropleth_layout` 不会自动采用共享色标。
`*_with_color_profile` 接受与独立色标相同的插值空间或命名色带。

```rust
use kartis::*;

fn main() -> Result<()> {
    let config = ChartConfig::default();
    let colors = ColorScaleOptions { kind: ColorScaleKind::SequentialQuantile, ..Default::default() };
    let values = [Some(0.), Some(0.), Some(5.), Some(10.), Some(10.), None];
    let cells = values.iter().copied().enumerate().map(|(i, value)| HeatCell {
        row: "observed".into(), column: format!("c{i}"), value,
    }).collect::<Vec<_>>();
    let heat = heatmap_layout(&config, &cells, &HeatmapLayoutOptions {
        colors: colors.clone(), axes: HeatmapAxes::none(), labels: false, ..Default::default()
    })?;
    assert_eq!(heat.cell(&HeatmapCellKey::new("observed", "c2")).unwrap().value, Some(5.));

    let days = values.iter().copied().enumerate().map(|(i, value)| {
        CalendarCell::new((2024, 1, i as u8 + 1), value)
    }).collect::<Vec<_>>();
    let calendar = calendar_layout(&config, &days, &CalendarLayoutOptions {
        colors: colors.clone(), start: Some((2024, 1, 1)), end: Some((2024, 1, 6)), ..Default::default()
    })?;
    assert_eq!(calendar.day((2024, 1, 6)).unwrap().value, None);

    let features = values.iter().copied().enumerate().map(|(i, value)| {
        let x = -75. + i as f64 * 25.;
        GeoFeature { id: format!("region-{i}"), label: format!("Region {i}"), value,
            polygons: vec![vec![vec![[x, 0.], [x + 12., 0.], [x + 12., 12.], [x, 12.], [x, 0.]]]],
        }
    }).collect::<Vec<_>>();
    let map = choropleth_with_color_scale(&config, &features, &GeoMapOptions {
        fit: GeoFit::Data, border: None, ..Default::default()
    }, &colors)?;
    let missing = map.scene.marks.iter().find(|mark| mark.series == "region-5").unwrap();
    let original = features.iter().find(|feature| feature.id == missing.series).unwrap();
    assert_eq!(original.value, None);
    assert_eq!(missing.value, 0.);
    assert_eq!(missing.fill, Some(map.color_scale.unknown()));
    Ok(())
}
```

HeatMap 从未隐藏的行列单元中解析分布。Calendar 使用未隐藏年份中实际提供的数据，
生成的缺失日期不会变成零观测。隐藏、增删数据和改变重复次数时，需重新建立布局或色标。
重排样本不改变映射颜色；getter 中相等值的正负零位表示仍保留原输入顺序。
共享色标 Choropleth 在地理投影裁剪之前使用**全部提供的要素值**。
仅改变相机不会重新定义样本分布。
各消费者仍受自身布局预算限制，色标的 20,000 个输入条目上限不会提高这些预算。

HeatMap 和 Calendar 在计算后的单元、日期及提示数据中保留可空原始值。
共享 Geo 结果目前提供 Scene、投影和色标，没有同等的可空要素元数据或绑定辅助接口。
其较早的 `Scene` 标量字段使用 `0.` 占位缺失的 `GeoFeature::value`。
应按原始 ID 保留权威 `GeoFeature` 数据表，并据此构造应用自己的可空提示与交互数据。
不要从占位值推断真实观测为零。排名只决定颜色，不替换业务值、ID 或应用的选择身份。

## 原函数证据与当前范围

`node scripts/continuous-quantile-reference.mjs --check` 使用保留的原版
d3-scale 4.0.2 / d3-array 3.2.5 bundle，无需 npm 或其他源码检出。
不可变参考包含 124 组分布，其中有限、n>=2 的对照范围为 109 组分布和 7,021 次有限排名查询。
独立的 `continuous-quantile-overflow-reference.mjs --check` 保留六个边界案例，
包括受检 API 拒绝的 24 次原版非有限 R7 结果。
参见[源清单](../tests/fixtures/sequential-quantile-d3-source.json)与[第三方声明](third-party-notices.md)。

核心、原函数对照、消费者、最终原生及四引擎浏览器验收已在本功能范围内通过。
SVG/Canvas 与 DPR1/2 覆盖完整样本、原始可空值、精确数据轴跳点及真实发布/resize 故障。
明确排除未解决 piecewiseScales 的三引擎范围回归为 177/177；四组系统 Chrome CLI 另行通过。
SVG 图形/文字缓存回滚缺陷已修复；后续包装节点波又修复五种已复现的 clip/mask/
reparent/首次 blend 事务故障，正常四引擎 DPR1/2 共 40 项专项，新的显式范围回归为
180/180，整个未解决 piecewiseScales 专项仍排除。包装节点结果仅覆盖 SVG；
伴生 Canvas、任意持续 DOM 故障、legacy 提案缓存原子性及真实设备认证不在结论内。
详见[验收记录](verification.md)。
较早严格合成器屏幕基准仍失败并单独保留；这不等于全部 D3/JavaScript/Nivo 行为等价，
GPU/worker 继续暂缓。
