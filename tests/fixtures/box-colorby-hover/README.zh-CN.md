# 原 Nivo BoxPlot 取色、图例和悬停参考

本夹具执行 Nivo 0.99.0 完整的 `useLevels`、`useBoxPlot`、`getLegendData`、`useBoxPlotTransition` 函数体，以及原始序数色、继承色、统计和比例尺函数。`vendor/` 保留原文件、MIT 许可证、提交号及字节哈希。

运行 `node scripts/box-colorby-hover-reference.mjs <released-D3-node_modules>` 生成结果，追加 `--check` 验证结果。依赖 Node TypeScript 类型擦除及发布版 D3 scale 4.0.2、interpolate 3.0.1、array 3.2.5、color 3.1.0，不安装新依赖。本参考没有执行 Babel/TSX、挂载后的 React 生命周期、Spring 动画、原图例组件或像素渲染。

`values.json.profiles` 有 32 组输入：group/subGroup 取色、正向/反向完整分类顺序、四种源数据、默认/显式 `group` 属性图例标签。每组保留完整观测、色盘、基态、由真实先前计算项激活的状态和已核对的恢复结果。完整色盘预热由原 hook 自己执行，包括预留空键；记录实际 D3 序数域。两个非空图例配置真实调用原 helper，保留数字索引 ID、文字、颜色和顺序。原图例按文字去重，且只包括实际盒项。`sourceKey` 是把原数字索引对应到输入域的适配注释，不是原生字段。

`baseline` 与 `active` 包含原计算盒项和过渡属性。`activeKeys` 通过原依赖对唯一识别并捕获实际 `useMemo` 返回值，不由替代悬停算法重建。关联条件始终为同 group 或同 subgroup，与 colorBy 无关。十个资格快照记录空源、上游过滤、外来/陈旧活动身份、畸形 JavaScript 活动对象、显式标签回调、null 转换和组/子组同名。

明确保留 Rust 比较边界：Rust 忽略 `None`，原 `Number(null)` 却贡献零，甚至形成全 null 盒项。原计算 key 为数字索引对；Rust 使用带类型的 group/subgroup 身份。仅改变原数字 key 而保留身份不会影响原关联，Rust key 也没有可任意覆盖的对应数字字段。外来身份在原生代码中可能部分关联，Rust 有界查找则拒绝。新 Group 默认图例在把数字 ID 对应原字符串后可以比较；旧 Rust SubGroup 图例保留完整配置域，包括空槽，本参考不暗改默认语义。自定义图例标签/属性路径仅记录原生行为，不表示新增 Rust 访问器 API。零 alpha 数字色盘保留 RGB，不为强迫一致而修改解析器或声明原生 Contrast 等价。

适配范围限于立即执行 memo、显式冷态、原始文字去重与稳定数字排序、普通点分属性/函数访问器、线性和 band 比例尺。保留的其他原比例尺/色带分支若被调用会明确拒绝。全部颜色、图例、key 和透明度期待值来自实际原函数输出，不是像素或按源码另写的替代算法。
