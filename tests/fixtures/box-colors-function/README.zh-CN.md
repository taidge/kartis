# BoxPlot 自定义颜色原函数参考

[English](README.md)

本夹具执行固定版本 Nivo 的完整 `getOrdinalColorScale`、BoxPlot hooks、摘要与布局函数、继承色解析器、过渡 hook 和图例 helper。图例 helper 使用真实 Lodash 4.17.21。原函数体保持不变，仅适配 TypeScript 类型和导入以便执行。React hooks 使用冷调用记录适配器。组件 JSX 仅固定为源码证据，不执行渲染。

在包根目录重新生成或检查：

```text
node scripts/box-colors-function-reference.mjs <released-D3-node_modules>
node scripts/box-colors-function-reference.mjs <released-D3-node_modules> --check
```

producer 检查已发布 D3 包的版本，并固定其完整 JavaScript、JSON 和许可证资源。`reference.json` 包含原源码哈希、运行时哈希及适配器限制。`values.json` 包含 32 个主要配置和 16 个资格案例。active 状态资格案例明确调用原 hook 两次，因此共 49 个主要冷快照；另执行 32 次重复调用检查结果确定性。box、颜色调用和图例行的计数仅统计主要快照。

矩阵覆盖 Group/SubGroup 身份、两种方向、常量/group/subgroup/mean/分位统计值/cell 相关函数，以及零与半透明 alpha。每次真实回调的输入和输出均有记录。`partial-domain` 调用来自原 hook 的完整域预热，`summary` 调用接收原摘要对象。自定义函数分支直接返回真实传入函数，不创建 `.scale`。精确文本去重和完整颜色索引稳定排序均为真实 Lodash 操作；代表索引指向保留的原图例行对象。

资格案例保留以下边界：

- 未防护就读取 `values[2]` 的函数，会在第一次部分域回调中真实抛错，尚未进入任何摘要。借用式 Rust 摘要工厂无需虚构这类部分输入回调。
- 原 `Number(null)` 会生成零值和全 null 摘要；Rust `None` 不参与统计，但原观测索引仍保留其位置。原隐藏案例使用明确的上游筛选，并保留完整配置域。
- 原普通摘要会丢弃任意原始属性；真实预计算摘要展开可以保留它们，真实 Lodash datum 路径会观察到这种区别。undefined、数值及非法 CSS 返回均作为原行为资格记录，不属于有效 Rust 类型化颜色。
- 原摘要 `quantiles` 是配置概率，`values` 是计算后的统计量。投影几何和原始观测切片属于来源注记，不是 Nivo 传给颜色回调的新增字段。
- CSS alpha 为零时，D3 modifier 解析可能丢弃隐藏 RGB；恢复 opacity 的案例记录真实黑色结果。Rust 类型化 RGBA 刻意保留原 RGB。精确 CSS 半透明值和四舍五入后的 8 位表示分别记录。
- 固定 border、median、whisker 颜色不受自定义数值颜色覆盖。固定版本 Nivo BoxPlot 组件没有 outlier 标记；Rust 默认和固定 outlier 行为需要 Rust 回归测试，不能虚构原渲染器 oracle。
- active 状态案例是两次冷调用，不是 React hover、memo、commit 或生命周期测量。Rust 接受后的颜色缓存可以在 hover 时不再调用借用式工厂。

本夹具不证明浏览器、GPU、worker、spring 动画、渲染器像素或 widget 行为。它记录 `ByLabel` 首代表颜色及规范目标，但不声称 Rust 旧 `Keys` 的预留行与 Nivo 仅实际 box 的图例一致。
