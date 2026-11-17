# Stream 函数配置原始源码参考

运行 `node scripts/stream-function-reference.mjs --frozen` 验证冻结参考。

快照包含完整的 Nivo 0.99.0 Stream 与共享 getter 源码、包元数据和完整 MIT 许可。验证器校验全部源码／许可／producer／编译器哈希，并执行原函数透传分支、增强图层／切片／上下文计算、点创建、默认点与面积 JSX 及图层门禁。

八类角色是 `label`、`colors`、`valueFormat`、`borderColor`、`dotSize`、`dotColor`、`dotBorderWidth` 与 `dotBorderColor`。标签和图层色接收同一个仅含 `id`、`path` 的临时对象；源采样格式化先执行，面积汇总不额外调用格式器。边框色在完整增强图层上计算，即使边框宽度为零。点函数按尺寸、颜色、边宽、边色顺序逐点执行，包括零样式；实际图层门禁另行记录。

作者提供的堆叠值、尺度、面积字符串、React hooks、JSX 和 Spring 值都是同步探针，不是 D3／React／Spring 运行时。运行时 null 单列记录，不属于公开数值／字符串 `StreamDatum` 合同。Rust 的 nullable、隐藏系列和共享符号尺寸需明确额外规则。这些函数不等于虚构的 `layerComponent` 或完整任意自定义图层生命周期。

[English](README.md)
