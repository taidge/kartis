# BoxPlot ByLabel：原始 Nivo 参考

本夹具实际执行 Nivo 0.99.0，commit `0dae2c32052a573f9f1d66ec1b453b572119b265`。32 个主 profile 和 8 个资格例来自完整原始 BoxPlot hooks、分层、统计、几何生成和图例函数。原 property accessor 与真正的 Lodash 4.17.21 均实际执行；期望图例行没有用替代的去重、排序或路径读取算法生成。

```sh
node scripts/box-legend-factory-reference.mjs node_modules --check
```

参数提供已发布的 D3 包：d3-scale 4.0.2、d3-interpolate 3.0.1、d3-array 3.2.5、d3-color 3.1.0、d3-format 3.1.3，以及依赖的 d3-time、d3-time-format、internmap。`reference.json` 记录源码、运行库和生成器的哈希。`vendor/source-pins.json` 记录原始路径与出处；冻结源码附完整上游许可证。

适配器仅擦除 import 与 TypeScript 类型，再导入完整原函数体。冷执行的 `useMemo` 立即求值；state 为 null；setter 明确抛错。原 transition hook 使用记录适配器执行。UI 占位入口和不支持的 scale/scheme 分支明确抛错。格式化适配器原样返回传入的 formatter 函数。本夹具证明原始同步数据计算，不证明挂载的 React、物理绘制、动画或生命周期回调次数。

## 可机读输入与期望输出

`values.json` 包含 `profiles[]` 与 `qualifications[]`。每项有 `id`、`input`、`actual`；资格例还记录准确的范围差异。

| 字段 | 含义 |
| --- | --- |
| `input.rawData` | 按调用者顺序保留的原始观测，`v: null` 仍为 null。 |
| `input.effectiveData` | 真正传入原 hook 的数据。hidden 资格例显式在上游过滤，但仍保留全部配置域槽。 |
| `input.groups`、`input.subGroups` | 完整配置域，含 missing/reserved 槽及作者顺序。 |
| `input.label` | 实际回调输入策略：pair、constant、raw-key、empty；嵌套路径资格例直接传原 accessor 路径。 |
| `actual.ordinalDomain` | 原 hook 真正预热后的 D3 ordinal domain。 |
| `actual.boxes` | 实际生成箱体的身份、完整索引、原摘要、几何、numeric color/RGBA 和 opacity。 |
| `actual.callbackContexts` | 真正收到的回调参数与返回文字；`data` 为完整原摘要，原观测索引另作明确注释。 |
| `actual.expectedLegend` | 原 hook 实际图例行，附代表摘要和原始 typed target 注释；文字、颜色、顺序由原 hook 输出。 |
| `actual.originalLegendOperations` | 真正 Lodash 的输入、输出及被选输入对象索引；包装只委托给真实 Lodash，保留返回对象引用。 |
| `actual.transitions` | 原 transition hook 的实际记录，不是采样动画帧。 |

每条期望行保留原数值 `nativeId`、`fullColorIndex`、`label`、`color`、`rgba`、`representativeInputIndex` 及代表摘要的完整身份。Group 的 `target.kind` 为 Category，SubGroup 为 Series；`rawKey` 与显示文字分开。此注释不自行发明 Rust canonical series 编码或传输 rowID。

生成器观察原 `uniqBy(label)` 返回对象，再观察原稳定 `sortBy(id)` 返回对象，通过对象引用找到原映射输入行。同键多文字可保留重复 native 数值 ID。跨键同文字只保留首次代表的颜色和 target，不联合多个 target。空文字产生真正保留的空图例行。

`data.quantiles` 与 `quantileProbabilities` 是配置概率 `[0.1, 0.25, 0.5, 0.75, 0.9]`；`data.values` 与 `computedStatistics` 才是五个实际统计值。构造 Rust 工厂上下文时不能互换。

## Rust 测试映射与明确差异

用 `input` 构造完整源与配置域，再按输入策略构造回调。将记录的原始 target 转为 Rust typed target 后，把 ByLabel 输出与 `actual.expectedLegend` 对照；不可拿显示文字当身份。除文字、颜色和顺序外，还应比较代表源身份与完整色域索引。旧 Keys 路径的完整 SubGroup 域、`Samples` 回退单独保持兼容，不借此修改。

32 个主 profile 覆盖 Group/SubGroup、四种回调策略、完整与重排域、带真实常值零的稀疏单元、空 reserved 域。8 个资格例记录 nullable、上游 hidden 过滤、原始空 subgroup、关闭图例及真正的 `values[2]` 路径读取。

Nivo 调用 `Number(null)`，null-only 单元也会生成真实零值摘要；Rust 从统计中省略 `None`，None-only 不产生摘要，测试应显式保留差异。hidden 过滤是输入适配器，不冒称原 hook 自带 hidden API。嵌套路径的数值标签是原生输出，Rust String 工厂需显式转换。此处 alpha-zero palette CSS 保留输入 RGB；其他 inherited-color 解析路径可能丢透明 RGB，不应据此修改 Rust 原始颜色。

`reference.json` 的数据计数描述 40 份存档主冷快照，另有 32 次完整冷执行验证确定性相等；它们不承诺 React、多份 legend 配置或 Rust builder 的生命周期回调次数。
