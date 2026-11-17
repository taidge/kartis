# 原版 Tree 活动状态参考

`reference.json` 记录 Nivo 0.99.0 原始代码的实际执行结果。`original/` 包含完整上游源码文件、包清单及四份完整 MIT 许可。参考文件使用紧凑 JSON，未省略字段。

使用项目固定版本的依赖验证冻结参考：

```sh
npm ci
node scripts/tree-activity-reference.mjs --frozen
```

验证只读取冻结文件和已有编译器包，不需要上游源码检出、Git、网络请求，也不改写参考文件。可选的参考目录参数用于隔离完整性测试。重新生成的命令为 `node scripts/tree-activity-reference.mjs <nivo-checkout> <output-directory>`。

参考执行原始当前节点 setter、节点及连线 hooks、四项 modifier、固定深度颜色继承、标签计算、节点鼠标 handlers、mesh 事件 handlers，以及 SVG/Canvas 的 mesh 参数传递表达式。案例覆盖四个 highlight 标志的全部组合、中性/聚焦/清除状态、函数/常量/未指定 modifier、零结果及原始数值深度阈值边界。

React hooks 使用明确的同步依赖探针。层次遍历、坐标、光标位置及 Delaunay 匹配属于作者提供的输入。JSX 编译器记录组件创建参数；不宣称执行真实 React、Spring、浏览器事件循环、Voronoi 匹配或等价的 D3 数值布局。

中性状态的 `isActive` 为 `null`，不调用任何 modifier 分支。存在聚焦节点时，连线依据独立的活动边集合计算；该集合为空意味着所有连线均为非活动。固定深度继承会跳过深层节点的颜色回调。默认节点 handler 在进入和离开时改变当前节点，移动时不会；mesh 移动 handler 则会更新当前节点。键盘焦点和 Rust 平面标识符属于独立的宿主与受检输入合同。
