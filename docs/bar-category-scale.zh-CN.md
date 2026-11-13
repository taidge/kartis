# 配置类别带状尺度

`BandScale::with_options` 提供受检的 D3 带状尺度舍入、对齐与独立内外间距。`BarStyleOptions::category_scale` 将同一尺度用于柱位置、类别刻度、总计和返回的类别元数据。

```rust
use kartis::*;
use kartis::scales::{BandScale, BandScaleOptions};

let categories = ["A", "B", "C"].map(str::to_string);
let profile = BandScaleOptions {
    padding_inner: 0.2,
    padding_outer: 0.2,
    round: true,
    ..Default::default()
};
let scale = BandScale::with_options(categories.to_vec(), [0., 101.], &profile)?;
assert_eq!(scale.map("B"), Some(38.));
assert_eq!(scale.bandwidth, 25.);
assert_eq!(scale.step(), 31.);

let layout = bar_layout_with_style(
    &ChartConfig::default(),
    &categories,
    &[BarSeries::new("values", vec![Some(0.), None, Some(3.)])],
    &BarOptions::default(),
    &BarStyleOptions {
        category_scale: Some(profile),
        ..Default::default()
    },
)?;
let svg = render::to_svg(&layout.scene)?;
# Ok::<(), kartis::ChartError>(())
```

显式配置先在完整类别范围上完成舍入，类别窗口随后连续投影已确定的位置，不再次舍入。`StyledBarLayout::category_scale` 返回已应用窗口的尺度。分组、堆叠和百分比布局共享其 step 与 bandwidth；分组内部的像素间隔仍由 `inner_padding` 控制。

显式配置配合 `category_reverse: true` 可反转类别范围，同时保留原类别与来源身份。横向柱图可由此对应 Nivo 的反向 y-band，无需重排输入数据。`category_reverse` 要求同时设置 `category_scale`；显式配置不能与旧 `category_inner_padding`、`category_outer_padding` 覆盖项混用。`category_scale: None` 和 `BandScale::new` 保留既有行为。

内间距和对齐值须为 [0,1] 内的有限数，外间距须为 [0,4] 内的有限数。域最多包含 20000 个唯一非空字符串，每项最多 1024 字节；范围及投影结果须可表示。内间距 1 产生零带宽。`invert` 沿用既有带区间约定，零带宽也可按精确逻辑位置反查，它不代表实际墨迹命中。分组像素间隔超过可用带宽时拒绝。

运行 `cargo run --example bar_category_band -- output-directory` 可生成横纵方向、分组与堆叠的四幅实际 SVG/Canvas 场景。原 D3/Nivo 数值参考及完整许可证位于 `tests/fixtures/bar-category-band`。新增公开配置和布局字段属于源码 API 扩展；构造配置时可使用 `..Default::default()`。
