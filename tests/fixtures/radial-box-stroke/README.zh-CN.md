# 原始 RadialBar 与 BoxPlot 描边参考

生成器实际执行 Nivo 0.99.0 提交 `0dae2c32052a573f9f1d66ec1b453b572119b265` 中完整的 hooks、过渡、继承与序数颜色生成器、defs 绑定及 JSX 渲染组件函数体。原始源码和完整 Nivo 许可证固定在 `vendor`；既有 BoxPlot 计算、比例尺、颜色参考及 D3 许可证通过仓库相对路径复用。

```sh
node scripts/radial-box-stroke-reference.mjs <scale-node_modules> <shape-node_modules> --check
```

传入 `reference.json` 所列发行版依赖。项目固定的 Playwright 1.56.1 提供已有 Babel TypeScript/JSX 转译器；生成器不启动 React 应用、动画引擎、浏览器或像素渲染，也不安装依赖。

React memo/callback 同步求值，state 是显式冷快照。Spring 适配器记录原始各阶段函数，并用 update 结果交给 JSX/createElement 属性记录器。属性访问仅覆盖普通点路径；基本 `uniqBy`/`sortBy`、band ticks、tooltip 与 scheme 的适配范围在生成器中明确。不改写原始角色算法或删除错误分支，未支持的适配路径会拒绝。

`values.json.radial[].bars[]` 保存真实类别与组身份、可空源值、数值调色板、绑定填充、边框颜色、各过渡阶段和原路径属性。`values.json.box[].boxes[]` 保存真实统计摘要、group/subgroup 颜色、三个角色输出及原 group/rect/line 属性。主数据包含32组 Radial 配置、128个计算段，28组 BoxPlot 配置、140个箱体、420个角色输出；另有11组输入覆盖空源、显式上游过滤、原生 null 转换、零范围、禁用宽度、来自实际计算项的 active 快照和预计算摘要嵌套属性，并保留6个原生错误和3个属性访问边界。

Rust 对照须遵循资格标记。原生 Radial 的 null 仍形成计算段，Rust `None` 不凭空创建段；原生 BoxPlot 对原始 null 执行 `Number(null)`，计入零值。上游过滤是适配步骤，并非原生隐藏 API。Radial 调色板按类别首次出现顺序；BoxPlot group 配色单列为已确认功能差距，当前 Rust 只按 subgroup 配色。paint URL 替换显示填充时，角色仍读取原 `datum.color`。

直接返回未修饰属性时保留 CSS 零 alpha 的 RGB；执行 D3 修饰器会把其 RGB 解析为 NaN，后续 opacity 修饰甚至可能让黑色可见。Rust 有意保留有类型的原始 RGB，因此相关行带 `cssAlpha0ParserBoundary`，不伪造完全相等。原 Nivo 的 Theme 路径和任意属性访问能力独立记录；Rust 有界角色的 Theme 指各自配置的固定描边色，Contrast 属于 Rust 扩展。宽度、透明度及渲染属性只是数据，不代表像素一致或 React 生命周期通过。
