# Network 节点组件契约

此参考保留 Nivo 0.99.0 的八份完整源码、包清单和完整 MIT 许可证。
验证会先校验源码、生成脚本和固定版本 TSX 编译器的哈希，再执行原始配方。

原始节点准备及计算样式 hook 通过明确的依赖探针执行，记录四组普通、激活、
非激活样式配置及默认样式值。20 组组件配置覆盖动画、交互和五种过渡状态，
共产生 80 次创建参数。参考执行原始默认节点 JSX、四种鼠标处理路径和
tooltip、激活节点的调用顺序；另有八组默认 Canvas 绘制命令记录。

原始准备函数浅拷贝原始数据，将临时位置和速度置零，随后交由 D3 模拟修改。
此参考显式指定最终坐标、速度和索引，不执行力模拟、React 生命周期、Spring
调度、浏览器事件投递、真实 Canvas 颜色解析或帧缓冲等价比较。

父节点图层要求节点数组非 null；空数组不产生组件。创建过程不按零尺寸、
透明颜色或视窗外坐标过滤节点，交互开关仅控制四个处理函数。初始、进入和
更新状态的缩放及透明度为一；from、leave 为零，但保留原尺寸。默认半径直接
取 size/2，不使用 Swarm 的非负截断。鼠标进入依次显示 `{node}` tooltip、
设置激活 ID、调用使用者；离开依次隐藏 tooltip、清空激活 ID、调用使用者。

Kartis 提供当前 `ForceNodeState` 快照，而非任意原始 Nivo `InputNode` 对象。
物理半径、固定坐标、tick 和热度属于 Rust API，与视觉图形尺寸及边界分别表达。
空标签或无墨迹 Widget 角色不会因此增加标准业务控件；Scene 自有标签和图关系
行为仍由各自的接口管理。

安装固定版本的项目依赖后，验证不需要源码检出目录、Git、网络或写入文件：

```sh
node scripts/network-node-widget-reference.mjs --frozen
```

`--frozen` 后可指定另一份参考目录。重新生成使用独立命令：

```sh
node scripts/network-node-widget-reference.mjs <nivo-checkout> [output-directory]
```
