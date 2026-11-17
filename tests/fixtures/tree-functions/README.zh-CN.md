# Tree 函数配置原始源码参考

运行 `node scripts/tree-function-reference.mjs --frozen` 验证冻结参考。

快照包含完整 Nivo 0.99.0 Tree hooks、类型、标签定位、默认值、SVG／Canvas 父组件和角色绘制器，以及共享 getter 源码、包元数据和三份完整 MIT 许可。全部源码／许可／producer／编译器字节均校验哈希，并重新执行原函数透传、节点／连线计算、标签定位和 SVG／Canvas 标签调度表达式。

节点先计算尺寸，再计算颜色；两者输入都是尚无最终尺寸、颜色和活动状态的中间节点。有限 `fixNodeColorAtDepth` 会继承已计算父节点颜色，跳过更深后代的原颜色回调。连线取得完整计算后端点，先厚度、后颜色，随后才处理活动修饰。标签输入包含最终尺寸的完整节点；SVG 标签创建有门禁，Canvas 即使不绘标签图层也调用标签 hook。

层次遍历、x/y、尺度和 React hooks 均为显式同步作者探针，不是 D3 树数值布局、React 调度或 Spring 动画。JSON 序列化前核对原嵌套对象引用和有符号零。Rust 的扁平 ID、聚合值、既有 inset 投影、半径约定及可选径向布局均为明确差异。五个基础回调不代表同时支持四个活动／非活动尺寸函数、颜色深度继承、任意组件或自定义图层。

[English](README.md)
