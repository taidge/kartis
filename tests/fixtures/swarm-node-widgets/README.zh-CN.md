# Swarm 节点组件契约

此参考保留 Nivo 0.99.0 的八份完整源码、包清单和完整 MIT 许可证。
重新执行前会校验源码、生成脚本和已固定版本 TSX 编译器的哈希。

生成脚本执行原始节点准备及增强表达式、完整 `Circles` 创建流程、默认
`CircleSvg` JSX、过渡函数、七种事件处理路径和父组件的 circles 图层条件。
20 组配置覆盖动画及交互开关与五种过渡状态，共产生 80 次原始组件创建参数。
另有八组配置通过命令记录代理执行原始 Canvas 默认绘制函数；它们不代表浏览器
Canvas 像素或 CSS 颜色解析验收。

节点位置和模拟索引由参考输入指定。同步 hooks、访问器、颜色、标量 `.to`、
tooltip 和 JSX 记录均为明确的依赖探针，不执行 React 生命周期、Spring 调度、
D3 力模拟、浏览器事件投递或帧缓冲等价比较。数值布局和真实 Canvas 验收另行进行。

原始组件创建流程不按零尺寸、透明颜色、视窗外坐标或交互开关过滤节点。
图层资格单独判断；交互开关控制七个可选处理函数。`from`、`leave` 状态的半径及
透明度为零；初始、进入和更新状态采用 size/2 与透明度一。半径插值会将负值截为零。
原始 datum 采用浅拷贝，值可以是数字或 Date，并非 nullable 接口。

Kartis 采用自己的实际站点资格、隐藏及裁剪规则、稳定源索引、固定尺度装填和
已接受的类型化数据契约。Glory Widget 不等同于 React 或 Spring 组件；Rust 的
标签是源元数据，不意味着上游提供独立的 Swarm 标签组件。

安装项目固定版本的依赖后，可在不访问源码检出目录、Git、网络或写入文件的条件下验证：

```sh
node scripts/swarm-node-widget-reference.mjs --frozen
```

`--frozen` 后可指定另一份参考目录。重新生成使用独立命令：

```sh
node scripts/swarm-node-widget-reference.mjs <nivo-checkout> [output-directory]
```
