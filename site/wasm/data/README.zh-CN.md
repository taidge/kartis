# 世界国家多边形数据

[English](README.md) | [简体中文](README.zh-CN.md)

`world_countries.json` 是 Nivo 地理示例使用的原始 GeoJSON，包含 176 个 Polygon / MultiPolygon 地区，保留其稳定 ID、名称和全部原始字节。

来源：https://github.com/plouc/nivo/blob/0dae2c32052a573f9f1d66ec1b453b572119b265/website/src/data/components/geo/world_countries.json

`source-index.json` 记录来源提交、文件大小及 SHA-256；`NIVO-LICENSE.md` 保存完整上游 MIT 许可。

## 显式闭环适配

原件中 KOR 的第 0 个多边形、第 0 个环未闭合。派生文件 `world_countries.closed.geojson` 保留该环的全部 62 个原坐标，并追加首坐标；其余环不变，全部 176 地区仍然保留。原件文件不修改。清单同时记录原件、派生文件哈希及适配详情。

在仓库根目录执行以下相对命令可复现检查，不需要任何外部源码目录：

```sh
python site/wasm/data/normalize_world.py --check
```

## 示例绘制范围

Kartis 使用正式 `geodata::parse_geojson` API 严格解析完整的 176 地区派生数据。两张展示地图明确绘制 **175 个非南极地区**：当前平面投影不支持 ATA（南极洲）的包极球面帽拓扑，故显式排除。其余 175 地区均通过正常几何和预算校验，不跳过其他地区或降低校验要求。

实际作者数据面板记录 `sourceFeatureCount`、`renderedFeatureCount`、`excluded`、闭环说明与来源信息。地区数值按稳定 ID 生成，仅为演示，不是国家实测统计。边界及名称沿用上游数据，不表达对领土状态的立场。
