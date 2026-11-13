# 原 Nivo BoxPlot caption map 参考

[English](README.md)

本夹具执行 Nivo 0.99.0 完整的 `useBoxPlot`、`getLegendData`、`useBoxPlotTransition`、原统计／比例尺／颜色辅助函数，以及原 core `getPropertyAccessor` / `usePropertyAccessor`。JSON caption 描述会转换成真实函数，作为 `legendLabel` 传入。原访问器返回同一个函数，原图例 helper 在真实摘要上调用它。期待的图例行全部来自原 helper，包括按文字去重的结果。

```text
node scripts/box-legend-labels-reference.mjs <released-D3-node_modules>
node scripts/box-legend-labels-reference.mjs <released-D3-node_modules> --check
```

可选的 `--shared-vendor <directory>` 参数选择未修改的[共享原源码包](../box-colorby-hover/README.zh-CN.md)。producer 不安装依赖，使用 Node TypeScript 类型擦除及受版本检查的 D3 发布模块，并记录完整依赖源码哈希。复用 Nivo 源文件及其 [MIT 许可证](../box-colorby-hover/vendor/nivo-LICENSE)；新增 core 属性源码保存在 `vendor/`。Lodash 4.17.21 发布源码以原始字节保存在 `vendor/lodash.cjs`（仅将包内文件名 `lodash.js` 改名），附带[许可证](vendor/lodash-LICENSE)。运行其真实 `uniqBy`、`sortBy`、`isFunction`、`get`、`isPlainObject`，没有替代算法。来源与哈希见 `vendor/source-pins.json`。

30 组主输入为 Group／SubGroup × 唯一／重复／空 caption × 完整／稀疏／删除 group／删除 subgroup／空源预留域。每组保存完整原始域、色盘、观测值及 caption map。基态、使用真实先前计算摘要的激活态、恢复态分别执行原 hook。两个非空图例配置调用原 helper，并用一次直接调用交叉核对。记录实际 callback 参数与返回文字。`activeKeys` 来自原 memo 的实际返回值，不重写关联谓词。删除输入后仍保留显式完整色盘域；这是上游来源删除，不是虚构原生 hidden 参数。

七个资格快照覆盖部分映射、同一选中身份经非 map 回调产生多个标签、显式空子组文字与原始回退、未使用的额外 map 键，以及超过 Rust 字节限制的 caption。主矩阵使用有限数值观测；nullable／强制转换的独立边界继续由共享参考保留。

Rust 比较边界明确保留：

- 拟议 Rust map 在 caption 相同或为空时仍保留 ID、角色和行数；Nivo 按文字去重，保留首个来源项及其数字 ID／颜色。
- 原数字图例 ID 原样记录。`sourceKey` 只是把数字索引对应完整原始域的适配注释，不是原生字段。
- Rust Group 图例保留实际有资格的组；旧 SubGroup 图例保留完整域，包括预留／隐藏身份。原生图例只包含实际摘要。
- 缺失映射键时，原回调返回原始选中键；Rust 保留既有回退，包括空子组的 `Samples`。显式空 caption 可以请求空文字。
- Rust 检查每个键是否属于选中完整域，并限制每条 caption 为 1024 个 UTF-8 字节。原回调可以忽略描述中的额外键或返回更长文字；本参考不宣称执行原 UI 排版。
- key-stable map 无法为同一选中身份产生多个不同标签。记录的非 map 回调及任意属性访问器不属于拟议 API，也不推导新的系列别名或图例动作协议。

producer 不执行 React 挂载、事件派发、Spring 积分、原图例组件、字体排版、SVG／Canvas／CPU 像素或 Rust 实现。生成与 `--check` 只核对原函数参考输出；当前产品验收仍需独立的源码及功能检查。
