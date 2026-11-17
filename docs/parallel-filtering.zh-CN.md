# 平行坐标逐轴筛选

此 API 自 Kartis 0.2.0 提供。

[English](parallel-filtering.md)

`ParallelFilterController` 持有完整的平行坐标源数据。`layout()` 返回可直接渲染的
`ParallelCoordinatesLayout`：被筛掉的行不会生成路径、所属标签和交互站点。
坐标域、原始行/源索引、配色槽及分组图例仍根据完整源数据生成。
被拒绝的行保留在 metadata 中，`drawn == false`；筛选同组的一行不会隐藏整个组。

`new(config, dimensions, rows, options, profiles)` 的输入与
`parallel_layout_with_scale_options` 相同，空 profile map 使用维度自身配置。
`set_filter("temperature", Some(ParallelFilterRange::new([10., 30.])))`
设置原始数据单位的闭区间。多轴条件按 AND 合并，允许相等端点。
`None` 清除一个轴，`clear()` 清除所有条件。未知轴、非有限或降序数据区间拒绝。
有限但位于数据域外的区间允许存在，可以选不到任何行。

`ParallelFilterMissing::Exclude` 只拒绝正在筛选轴上的缺失值；`Include` 允许
缺失槽满足该轴条件。这与布局的 `ParallelMissing` 独立：匹配筛选的行仍可能
因为图例隐藏或 `SkipRecord` 而不绘制。

`set_pixel_filter` 接收真实已解析坐标轴方向上的 Scene 逻辑像素，自动规范化
反向端点并拒绝轴范围外的坐标。数值轴使用真实比例尺反算；UTC 保留连续毫秒
边界，避免把小数区间截断为整数后误选时间。已有源值恰好映射到端点时，即使
逆变换舍入也保持包含。被 clamp 到端点的域外源值不会成为假端点代理。
round 比例尺使用普通逆变换，不反推全部被舍入到同一像素的值。

`set_brush_filter` 可接收现有 `Brush`：Horizontal 布局使用 Y，Vertical 使用 X。
宿主须先把 client/host 坐标转换为 Scene 坐标。此入口不会自动挂载浏览器 DOM
brush、指针捕获或逐轴控件。这是 Rust 交互扩展，不声称 Nivo 的平行坐标包
公开了相同的 brush API。

`replace_filters` 一次准入整张筛选表；`replace_data` 校验所有新记录，并根据
完整新源重推断坐标域，保留原数据单位的筛选区间。更新失败时保留旧源、筛选、
绘制帧及拾取绑定。`options.hidden` 继续独立生效；更新宿主时应同时使用返回的
Scene 与 interaction binding。
