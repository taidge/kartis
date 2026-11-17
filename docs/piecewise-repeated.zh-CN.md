# Piecewise 域的重复节点

[English](piecewise-repeated.md)

**自 Kartis 0.2.0 提供。** 数值比例尺接受重复节点与全常量域。Line／Scatter 集成已通过下文
所列原生及四浏览器 SVG／Canvas 配置；已发布 Kartis 0.1.0 不含此扩展。

## 映射与相等节点

`PiecewiseScaleOptions` 保存等长的数值 `domain`、`range` 向量和既有
`ContinuousScaleOptions`，节点数为 2–4096。域允许非严格升序或降序，也允许全部相等；
真正转向仍明确拒绝。节点保持作者给定的顺序。

```rust
use kartis::*;

let scale = ResolvedContinuousScale::piecewise(&PiecewiseScaleOptions {
    domain: vec![0., 1., 1., 2.],
    range: vec![0., 10., 20., 30.],
    ..Default::default()
})?;
assert_eq!(scale.map(1.), 20.);
assert_eq!(scale.invert(15.)?, 1.);
assert_eq!(scale.map(scale.invert(15.)?), 20.);
# Ok::<(), ChartError>(())
```

重复节点可以产生跳跃。分段遵循原 D3 连续比例尺：先变换 domain，若末变换节点小于
首节点则同时反转 domain 和 range，再对内部节点执行 bisect right。range 为
`[0,10,20,30]` 时，Linear 域 `[0,1,1,2]` 将 `1` 映射为 `20`，降序域
`[2,1,1,0]` 则映射为 `10`。指数 `-1` 的 Pow 域 `[1,2,2,4]` 将 `2` 映射为
`10`，因为其原生有符号幂变换递减。内部裁剪坐标使用同一解析变换，包含倒数坐标归一化；
只按 raw domain 选段会丢掉这个区别。

两节点全常量域使用 D3 bimap，始终返回输出中点。三个及更多相等节点使用 polymap：
查询变换值小于节点时选首个 range 对的中点，相等或更大时选末对中点。因此 Linear 域
`[2,2,2]`、range `[0,10,20]` 将 `1` 映射为 `5`，将 `2` 或 `3` 映射为 `15`。
负指数会反转 raw 查询位于变换节点哪一侧的关系。

Clamp 在**整个 raw domain 上先执行，再变换与选段**。上述常量域启用 clamp 后，所有
有限查询均映射为 `15`；重复边缘不是单独的全局 clamp 平台。Round 在插值后执行
ECMAScript 取整，反算保持未取整语义。Nice 只调整两端节点，Reverse 反转整个 range，
刻度仍使用原始值单位。

## 校验与反算分支

相邻 raw 节点相等时，允许变换值相等。两个不同 raw 节点变换后塌缩为同一坐标仍拒绝。
有限算术、可表示跨度和既有类型约束保持受检：Log 要求同号非零，负指数要求同号非零
分支，UTC 要求有界整数毫秒。UTC 反算保留 Date 截断式 TimeClip。

`is_invertible()` 表示严格单调的 range 能选定唯一反算分支，不承诺双射、连续性或
`map(invert(pixel)) == pixel`。重复 domain 段可以产生常量代数反算。**Range**
转向或包含平台时继续沿用既有策略：全局 `invert` 拒绝，`invert_segment(index,pixel)`
显式选择分支，range 平台分支即使显式选择也拒绝。启用 clamp 时，显式分支反算夹到该
分支的域；全局反算则在逆变换之后夹到整个域。

## 数值约定

全部显式 piecewise 比例尺共用固定 `libm` log／log1p／exp／expm1 原语和缓存的变换
坐标插值，Nice 或视口重建去掉重复节点后也一样。公开 `PiecewiseScaleOptions` 不需要
携带私有来源标志，重建仍使用一致数值约定。普通两端点连续比例尺保持原有算术。

这对不连续点非常重要。SymLog 常量为 `2` 时，原 D3 变换区分 `-9.3` 与相邻更小的
浮点数，而相邻更大的浮点数确实与节点共享变换值。某些宿主 `ln_1p` 实现此前还把更小
输入与节点合并，选中了另一输出分段。改成 raw domain 二分也不正确，因为原版确实存在
的变换后相等仍须保留。

与此前严格单调域行为比较，2,400 个配置的 31,904 次查询中，76 个 raw 位模式变化；
最大绝对差为 `5.684341886080802e-14`，最大相对差为 `8.574643181216107e-16`。
两处曾记录的 SymLog 取整差异由 `208` 修正为 `207`，与原 D3 一致。原参考值与历史
差异名单均保留。

原生构建以及未启用 `canvas` 的独立 `wasm32` 核心，对一般有符号幂保留平台 `powf`。
`wasm32` 配合 `canvas`（含 `web`）特性时，显式 piecewise 比例尺通过安全的 wasm-bindgen 导入调用
实际宿主的 `Math.pow`，与同一运行时的原 D3 选择一致。因此，该特性会选择浏览器数学
原语，而非独立核心的平台数学路径；普通两端点比例尺仍保留原算术。固定 `libm`
log／log1p／exp／expm1 的约定不变。

固定 V8 13.6 原生参考默认调用 `std::pow`，其 legacy fdlibm pow 属于另一数值配置。
引入浏览器数学桥之前，完整 9,496 个配置的首轮 WASM 探针在 72 个指数 `0.25` 配置中
记录了 108 处差异，全部失败值及批量输出均保留。重建后的浏览器数学版本已在单个 Chromium
上下文完成同一完整矩阵，没有差异或一个 ULP 取整例外；这是独立实测的 WASM 结果，
不是用原生通过代替。选择同运行时原语也不承诺不同操作系统或浏览器引擎
的超越函数逐位相同。重复节点附近，一个 ULP 就可能选到另一分支并产生显著跳跃，
不能用通用像素容差掩盖。

## 图型与视口验收

Line 使用 `PiecewiseLineScaleOptions` 搭配
`line_layout_with_piecewise_scale_options`，Scatter 使用
`ScatterAxisScale::Piecewise`。图型 range 是首项 `0`、末项 `1` 的绘图区比例；底层
数值比例尺接受真实输出坐标。静态常量域可用，数据视口仍要求可用的外侧跨度和严格单调
的 range。

重复节点视口的目标是原 D3 `rescaleX`／`rescaleY`：保留完整作者映射与原始局部像素
range，再通过真实像素变换逐个反算节点。设基准尺寸为 `L`、窗口起点 `a`、跨度 `b`，
则 `k = 1/b`、`tx = -a*L*k`，每个新节点来自原比例尺的
`invert((rangeStop - tx)/k)`。range 方向已经包含源顺序和输出反转。另做一次 `[0,1]`
归一化／反算可能改变常量 Log 的结果，或使 UTC 相差一毫秒。Resize 必须保留已接受域
与原映射来源。

原生矩阵让 Line、Scatter 分别核对全部 4,128 条原 D3 视口记录。每种消费端拒绝
1,216 个常量显示域基线和八个原生真正转向的提案；Line 接受 2,904 次重排及重置，
Scatter 接受 2,848 次，另拒绝 56 个保留点中心超过 Scene 坐标预算的提案。两者均保留
872 个全常量窗口，并通过原 controller 重置。另四个无界请求核对作者指定的逆算 clamp。
有界导航约束像素窗口，不在反算结果上再加一次原始域夹紧。

Chrome、Firefox、WebKit、Edge 在 DPR 1/2 下另通过 288 组 SVG／Canvas 帧、256 组
真实指针／键盘拾取、96 组双渲染器拒绝事务和 16 组全常量窗口／重置。序列化 SVG
样本保留页面实际继承语言，确保字体选择与原图相同；包含 UTC 日期文字的整帧像素均差
保持在未更改的 0.5 门槛之内。这不代表 compositor 字节一致或所有引擎数学相同。
布局及回调 API 见[进阶比例尺指南](advanced-scales.zh-CN.md)，
受检证据见[验收](verification.md)。
