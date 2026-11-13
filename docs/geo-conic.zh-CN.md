# 圆锥投影与 Albers

[English](geo-conic.md)

等积圆锥、等距圆锥、保角圆锥和 Albers 共用地理地图、路线、经纬网、符号以及 SVG、Canvas 和 CPU Scene 管线。为地图追加图层时，保留其 `GeoProjectionContext`，确保相机和拟合坐标一致。

```rust
use kartis::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let config = ChartConfig::default();
    let features = vec![GeoFeature {
        id: "region".into(),
        label: "Observed region".into(),
        value: Some(25.),
        polygons: vec![vec![vec![
            [-110., 25.], [-110., 45.], [-80., 45.],
            [-80., 25.], [-110., 25.],
        ]]],
    }];
    let map = geographic_map_layout(
        &config,
        &features,
        &GeoMapOptions {
            projection: GeoProjection::conic_equal_area([29.5, 45.5])?,
            fit: GeoFit::Data,
            ..Default::default()
        },
    )?;
    let context = map.projection;
    let mut scene = map.scene;
    context.add_symbols(
        &mut scene,
        &[GeoSymbol::new("station", [-95., 35.], Some(25.))],
        &GeoSymbolOptions {
            clip_to_projection: true,
            clip_labels_to_projection: true,
            labels: true,
            ..Default::default()
        },
    )?;
    if let Some(pixel) = context.project([-95., 35.])? {
        let geographic = context.invert(pixel)?;
        assert!(geographic.is_some());
    }
    let svg = render::to_svg(&scene)?;
    assert!(svg.contains("<svg"));
    Ok(())
}
```

## 选择投影

在 Glory 中使用 `BrowserChart` 承载保留式 SVG/Canvas，或使用 `RasterSurface` 承载可交互 CPU 图像。投影遮罩需要这些渲染宿主；Glory 的语义 Graphic/Text 宿主没有任意路径遮罩操作。

| 投影 | 构造方式 | 标准纬线 |
| --- | --- | --- |
| 等积圆锥 | `GeoProjection::conic_equal_area([a, b])?` | 默认辅助函数采用 `[0., 60.]` |
| 等距圆锥 | `GeoProjection::conic_equidistant([a, b])?` | 默认辅助函数采用 `[0., 60.]` |
| 保角圆锥 | `GeoProjection::conic_conformal([a, b])?` | 默认辅助函数采用 `[30., 30.]` |
| Albers | `GeoProjection::Albers` | 固定 `[29.5, 45.5]` |

默认辅助函数分别是 `conic_equal_area_default()`、`conic_equidistant_default()` 和 `conic_conformal_default()`。标准纬线采用有限角度，必须严格位于 `(-90, 90)` 内。`GeoConicParallels` 保留输入顺序，并统一带符号的零。两条相同纬线及支持的圆柱极限，只要计算所得系数合法，就可以使用。

各投影分别检查自己的系数。非有限系数、多圈圆锥（`|n| > 1`）及等距投影在纬度域内部出现的径向折叠会被拒绝。解析系数还必须位于两条标准纬线正弦值之间，两个端点各保留八个相邻浮点数的余量；这可识别某些平台上差商舍入产生的错误零系数回退。该余量属于工程保护，不是经过认证的区间运算。同一对纬线可能被某种投影拒绝，但仍适用于等积投影。直接构造公开 enum 也不能绕过布局检查。Kartis 保留原始系数公式，不会静默钳制异常圆锥或换用另一种投影。

Albers 包含自身的 96° 经度旋转，以及由 `[-0.6°, 38.7°]` 投影得到的原点。应用的 `center` 和 `rotation` 仍在这个投影坐标系之前生效。最终结果使用 Kartis 的常规布局拟合，不复刻 D3 固定的默认视口和比例。`Albers` 是单个投影，不含阿拉斯加、夏威夷、波多黎各插图，也不是 `AlbersUsa` 组合投影。

## 拟合、路径与裁剪

`GeoFit::World` 根据有限圆锥扇区的极值拟合整个世界。`GeoFit::Data` 使用球面曲边保守包络，保留多边形孔洞和接缝片段。普通地图和 choropleth 入口使用约两度的固定粗采样；带 path options 的入口可显式启用像素自适应采样及细叶包络。包络需要覆盖弯曲边界，因此不与 D3 的紧致拟合逐位一致。路径精度、工作量和点数预算仍然生效。可通过 `geographic_map_layout_with_path_options` 配置自适应路径精度。

保角投影在应用相机变换后的纬度采用 ±85.0511287798066° 有限上限。点、路径、流式几何、裁剪和逆投影共用此范围，包括退化到 Mercator 的情形。范围之外的点不会产生可见坐标。等积、等距与 Albers 保留完整的有限纬度域。

`context.projection_clip_path(precision, max_points)?` 返回经过地图相同拟合变换的闭合有限世界轮廓。它是扇区或环形扇区；圆柱极限采用真实投影世界矩形。精度以逻辑像素计，范围是 `0.0001..=64`，点数上限必须在 `4..=180_000` 内；返回结果还必须符合共享 ClipPath 的 4096 条命令上限及 Scene 预算。Scene 坐标不可表示或预算不足会返回错误。

`clip_to_projection` 为符号笔迹应用投影轮廓；符号的绘图区裁剪仍由 `GeoSymbolOptions::clip` 独立控制。`clip_labels_to_projection` 为文字应用独立的投影轮廓蒙版，不意味着文字也被绘图区矩形裁剪。复制大量符号蒙版之前会检查累计命令数、线段数和工作量，避免耗尽 Scene 预算。配置为跳过的缺失符号不消耗蒙版复制预算。

## 逆坐标与发布

`project` 和 `invert` 使用 Scene 逻辑像素。浏览器宿主应先利用元素实际边界换算客户端坐标，并且仅撤销一次 Scene 视图变换。DPR 决定后备图像分辨率，不应再次乘入地理逆投影。原生宿主同样应通过已接受图像的边界换算真实事件坐标。

`invert` 在投影固有轮廓或有限纬度范围之外返回 `Ok(None)`。它不判断多边形覆盖，也不判断绘图区矩形；后者请使用 `invert_in_plot`。输入非法、运算不可表示或逆解精度不足会返回 `Err`。条件数检查估计数值不确定性并拒绝病态逆解，它是工程保护，不是覆盖所有平台与相机的严格区间证明。极点和反经线上经度可能不唯一，应比较球面位置。

应用负责保留原始 `GeoFeature`、`GeoSymbol` 及其 `Option<f64>`。旧式 `Mark::value` 占位值不能证明缺失测量等于零。使用带类型的 `InteractionBinding` 站点保留缺失值与来源身份。

将候选 Scene、交互绑定、原始数据表和投影上下文一起准备。先发布 Scene，仅在发布成功后替换应用已接受的上下文。非法标准纬线、来源值、工作预算或渲染发布失败时，指针逆投影和带类型选择都应继续使用之前已接受的帧。拟合数据、可见性、相机或尺寸改变时，需要重建上下文。

参考夹具与受保护逆投影测试位于 `tests/fixtures/geo-conic`、`tests/fixtures/geo-conic-unsafe` 和 `tests/geo_conic*.rs`。异常参数补充夹具保留 D3 原函数的多圈、径向折叠及错误逆解分支，不把重新投影相同坐标作为逆解有效的充分证明。固定参考版本及许可证见[第三方声明](third-party-notices.md)。

## 中点进度修复与验收范围

上一版画廊审计发现了真实的 Conic 采样盲点：四分之一、中点和四分之三点的弦误差虽低于请求精度，靠近端点的短急弯仍可能偏离约九像素。修复仅为新增 Conic 投影增加沿弦的中点进度细分条件，不改变工作量、深度或点数预算。Chrome 中两个独立 old/new WASM 二进制的实测误差由 9.01848 降至 0.4967817 像素，低于原 0.5 目标；这项成对诊断不能替代新版本完整功能验收。原 D3 自身约 0.000485 的近极点细分偏差单独保留为参考诊断，不能用来解释或豁免已修复的 Rust 盲点。

修复前 1126 路径 cohort、183 项已选回归通过及原功能证据已完整归档且不可变；该基线明确 featureAccepted=false。不将修复前通过项或图片拼入修复后版本成绩。

新源码 Rust 与消费者检查：最小特性 1082 项、全部特性 1285 项、15 个 doctest、native/WASM 严格 Clippy、Rust 1.95 和四个新构建的浏览器消费者全部通过。新四引擎 SVG/Canvas 与 DPR1/2 功能检查：Chrome、Firefox、Playwright WebKit、Edge 各自完成 DPR1/2 的 SVG/Canvas：208 项普通/typed 用例，另有 96 次接缝、32 次绘制对照、32 次非法候选、16 次发布故障及 32 次 resize 检查（16 次合法、16 次实际发布失败）。独立核验 288 PNG、96 对原始回滚记录、144 marker 像素、480 可信输入包和 11 个篡改反例。最大绘制平均差为 0.207068，原门槛仍为 0.5；源码与 HTTP/WASM 指纹一致。新原生 focused/full：正常 target 的格式及严格 Clippy 通过，headless Apace focused/full 各通过 16 case、各保留 32 PNG；独立解码全部 64 PNG，核验 3476 个精确来源颜色像素与 175 个原生源码指纹，前后源码指纹一致。新画廊几何、拟合、蒙版及导出：实际 246 幅场景保留旧 244 幅独立 SVG 字节不变。两张新卡通过可移植原 D3 几何/绘制检查，包括每图 761 个经纬网作者节点的 1e-9 对照、来源色与缺失值、plot/projection 蒙版及低于 0.5 的 SVG-image/Canvas 平均差。修复后旋转 Albers 可见经纬网的双向采样距离为 0.056184 和 0.495643 像素；不称原生细分逐位相等或全局 Hausdorff 保证，另以 25001 个原 direct-function 点验证已修复急弯。新版本正常回归与 Chrome CLI 收口：全新完整 cohort 2863059d-9ad6-4490-a2be-a0f1caf966bf 通过 183/183，Firefox、WebKit、Edge 各完成明确选取的 61 套，整个 piecewiseScales 仍排除。四组系统 Chrome CLI 独立通过，host 严格 compositor CLI 未计入。新版首轮 182/183 因 WebKit scaleProfiles 等待滚动稳定超时而保留为失败；原脚本在新旧宿主的四次独立对照未复现，原因仍未确定。最终仅采用三份完整新子报告，不复用部分通过项，也未更改断言、超时或像素门槛。

普通/typed 用例计数与额外的接缝、parity、invalid、publication、resize 检查口径分开；合法 resize 与真实 resize 发布失败分别计数。原生源码指纹与浏览器源码、HTTP/WASM 指纹也分开记录。只有完整同版本终结报告支持验收，部分通过项及早期运行均不拼接。

默认地图与 choropleth 入口保留固定粗采样及球面曲边保守包络；带 path options 的入口可显式启用像素自适应采样及细叶包络。不承诺与 D3 逐点细分或紧致拟合逐位一致。AlbersUsa 组合、球面面积/长度/质心/包含/圆生成 API、任意球面拓扑及更广投影配置仍待推进。投影蒙版应使用 BrowserChart、RasterSurface 或 SVG 导出，语义 Graphic/Text 不支持任意蒙版。GPU/worker 暂缓，较早严格 compositor 仍未通过，整体 D3/Nivo 等价目标保持进行中。详见[验收记录](verification.md)和[差距清单](feature-gaps.md)。
