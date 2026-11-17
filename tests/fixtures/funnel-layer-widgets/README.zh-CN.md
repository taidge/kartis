# Funnel 自定义图层与样式回调

此参考保留 Nivo 0.99.0 Funnel、colors、core 的完整源码文件、包清单及三份
完整 MIT 许可证。离线验证会校验全部源码、许可证、生成脚本和固定编译器哈希。

原始 Funnel 没有 `partComponent`、`labelComponent`、`separatorComponent`
属性。其函数型 custom layers 按声明的图层顺序接收完整 parts 与处理器、面积和
边框生成器、两侧分隔线、宽高及 `setCurrentPartId`。八组父图层配置执行真实的
`createElement` 参数构造。

16 组配置执行原始计算阶段回调和轮廓构造。原 colors、继承色、formatter 的
函数分支返回传入的函数。每个 datum 的调用顺序为尺寸、原始数据颜色、数值格式、
计算阶段边框色、标签色；样式回调执行时，轮廓数组仍为空。零宽、零值和零透明度
阶段仍参与这些回调。`part.data` 保留原输入对象和泛型额外字段。默认标签渲染
`formattedValue`，而不是 `data.label`。四组处理器配置执行原始 current、tooltip
与使用者回调的调用顺序；两组渲染配置执行默认阶段、标签和分隔线 JSX。

带宽、传入的尺寸函数、路径生成器、JSX、hook 调度和 Spring 均是明确的依赖
探针。此参考不运行 D3 布局或路径生成器、React 生命周期、Spring 时间调度、
原生事件、真实 tooltip、任意 Glory Widget 或帧缓冲等价比较。

参考也执行原始分隔线函数：其横向分支忽略两侧开关，末端分隔线使用字符串
`none`，空 parts 即使关闭开关仍会报错。这些上游事实如实保留，不要求 Rust
照搬。原 Parts/PartLabels 遍历所有阶段；Kartis 的 body、border、separator
组件角色要求存在真实来源 Mark。Rust 末端分隔线不会伪造来源 datum。测量
Widget 是 custom-layer 适配及 Glory 扩展，不是原版逐阶段组件属性，也不宣称
等价于任意图层位置 API。

安装固定版本项目依赖后，验证不需要源码检出目录、Git、网络或写入文件：

```sh
node scripts/funnel-layer-widget-reference.mjs --frozen
```

`--frozen` 后可指定另一份参考目录。重新生成使用独立命令：

```sh
node scripts/funnel-layer-widget-reference.mjs <nivo-checkout> [output-directory]
```
