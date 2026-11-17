# Network 连线组件契约

此参考保留 Nivo 0.99.0 的八份完整源码、包清单及完整 MIT 许可证。
验证会先检查原始源码、许可证、生成脚本与固定版本 TSX 编译器哈希，
再执行从原始源码提取的配方。

原 hooks 浅拷贝连线数据并解析端点，再加入计算节点、索引、宽度和颜色。
坐标、索引与计算节点样式是显式探针输入，不是力模拟结果。32 组配置覆盖
动画、不会生效的交互输入、两种混合模式及 initial/from/enter/update 四个阶段，
共产生 224 次原始 `createElement` 创建参数。参考执行默认 SVG JSX，另记录
28 组 Canvas 命令。依赖 hooks、JSX 和过渡标量均为明确的同步探针；不运行
React/Spring、D3 force、真实 Canvas 属性校验、浏览器事件或帧缓冲等价比较。

父层要求包含 links 图层且计算连线数组非 null；空数组不创建组件。零宽度、
透明色、自环、平行边或视窗外端点不会被过滤。创建仅传入 `key`、`link`、
`animated`、`blendMode`，没有鼠标处理器。initial/enter/update 使用完整端点及
透明度一；from 将终点收拢到起点且透明度为零。过渡配置没有显式 leave 配方。
默认 SVG 使用 round 端帽及传入的混合模式。Canvas 设置 stroke 样式和宽度后
绘制线段，不设置透明度、端帽或混合模式。命令记录器不会模拟真实 Canvas
对零线宽等赋值的规则。

原始 key 用点连接两端 ID，因此平行边及部分包含点的端点名称会碰撞；此处
如实记录，不修改上游。Kartis 使用不同的 `force-link:index` 插槽：身份跟随
边的排列，边重排时不承诺同一语义边保留局部组件状态。其数据包提供当前有限
force 节点和边状态，不冒充任意 Nivo raw-link 属性。自定义墨迹边界不替代原始
端点中点，实时组件也不会新增连线业务控件或来源 force 拖动目标。

安装固定版本项目依赖后，验证不需要源码检出目录、Git、网络或写入文件：

```sh
node scripts/network-link-widget-reference.mjs --frozen
```

`--frozen` 后可指定另一份参考目录。重新生成使用独立命令：

```sh
node scripts/network-link-widget-reference.mjs <nivo-checkout> [output-directory]
```
