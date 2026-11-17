# Tree 组件原始源码参考

运行 `node scripts/tree-widget-reference.mjs --frozen` 验证冻结参考。

快照包含完整的 Nivo 0.99.0 Tree 原文件、包元数据与完整 MIT 许可。验证器检查源码、许可、producer 与固定 TSX 编译器的哈希，重新执行原始 UID／计算数据 hooks、节点／连线创建和 transition 配方、默认 JSX、事件处理配方、Canvas 绘制函数及图层分发表达式。

层次遍历对象、数值坐标、尺度、颜色、React hooks、JSX、Spring 标量和路径生成器均为显式同步探针。该参考不运行 React／Spring 调度、D3 树数值布局、原生 Canvas 属性赋值校验、浏览器输入或帧缓冲。序列化前检查原始嵌套 `node.data` 引用和有符号零。Rust 使用扁平且全局唯一的 ID；径向布局与聚合标量属于独立合同。

节点 leave 将尺寸归零；连线 leave 保留厚度。对应图层存在时，零尺寸节点、零厚度连线和非交互记录仍创建组件。原始点号连接的路径 UID 可能碰撞，参考保留该行为。标签源码仅说明独立的标签组件角色，本轮节点／连线功能不实现标签组件。

[English](README.md)
