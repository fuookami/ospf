# BPP3D Interface Renderer

:us: [English](README.md) | :cn: 简体中文

这是一个 Tauri + Vue 渲染器，用于读取 BPP3D 装载方案 JSON，并在 Three.js 场景中展示每个货柜。

## 支持形状

- `Cuboid`：使用 `THREE.BoxGeometry` 渲染。
- `Cylinder`：当 `axis` 为 `X`、`Y` 或 `Z` 时，使用 `THREE.CylinderGeometry` 渲染。
- 缺少形状字段的旧数据会按 `Cuboid` 处理。

圆柱位置沿用后端外接盒坐标语义。mesh 中心按 `x/y/z + boundingWidth/boundingHeight/boundingDepth / 2` 计算，缺少外接尺寸时回退到 `width/height/depth`。

汇总面板会优先使用 DTO `actualVolume` 汇总物品体积，因此圆柱装载率按真实圆柱体积计算，而不是按外接长方体体积计算。

Tauri DTO parser 支持数值字段用 JSON number 或数字字符串表示。为了兼容外部输入，也接受 `CYLINDER` / `VERTICAL_CYLINDER` 这类枚举别名；BPP3D 当前输出仍是 `Cylinder` / `VerticalCylinder`。

以下圆柱数据会显示为告警色外接盒占位，在浏览器控制台输出诊断信息，并在物品详情面板展示不支持原因：

- 同时缺少 `radius` 与 `diameter`

## 形状字段

为了兼容旧 JSON，装载项的形状字段都是可选字段：

- `shapeType`：`Cuboid` 或 `Cylinder`
- `renderShapeType`：`Cuboid` 或 `Cylinder`
- `algorithmShapeType`：`Cuboid`、`VerticalCylinder`、`HorizontalCylinderX`、`HorizontalCylinderZ` 或 `BoundingCuboid`
- `radius`
- `diameter`
- `axis`：`X`、`Y` 或 `Z`
- `boundingWidth`
- `boundingHeight`
- `boundingDepth`
- `actualVolume`

物品详情面板会在字段存在时展示真实形状、渲染形状、算法形状、圆柱轴向、半径、直径、实际体积、包装尺寸和外接尺寸。

`BoundingCuboid` 只作为兼容输入处理。当旧数据同时携带 `shapeType = Cylinder` 与 `renderShapeType = Cuboid` 时，renderer 会显示外接盒，同时在详情面板保留圆柱 metadata。

## 圆柱坐标设定指南

圆柱坐标使用与长方体一致的外接盒原点语义：

- `x`、`y`、`z`：圆柱外接盒的最小角点。
- `boundingWidth`、`boundingHeight`、`boundingDepth`：完整外接盒尺寸。
- `radius`：圆柱半径。`diameter` 通常应为 `radius * 2`。

按轴向设置外接尺寸：

- `axis = X`：圆柱沿 X 轴横放。`boundingWidth` 是圆柱长度，`boundingHeight = diameter`，`boundingDepth = diameter`。
- `axis = Y`：圆柱沿 Y 轴竖放。`boundingHeight` 是圆柱长度，`boundingWidth = diameter`，`boundingDepth = diameter`。
- `axis = Z`：圆柱沿 Z 轴横放。`boundingDepth` 是圆柱长度，`boundingWidth = diameter`，`boundingHeight = diameter`。

如果要让圆柱放在货柜底面上，设置 `y = 0`。对 X/Z 轴横向圆柱，renderer 会把 mesh 中心放在 `y + boundingHeight / 2`，因此当 `boundingHeight = diameter` 时，圆柱底部正好贴地。

## 示例数据

- `src/examples/legacy-cuboid-loading-plan.json`：旧版纯长方体数据。
- `src/examples/cuboid-cylinder-loading-plan.json`：长方体、原生 X/Y/Z 轴向圆柱和 solver-selected radius 混装数据。
- `src/examples/unsupported-bounding-cuboid-loading-plan.json`：兼容 `BoundingCuboid` 与不支持圆柱数据。

## 常用命令

```bash
npm run build
npx vue-tsc --noEmit
cargo check
cargo test
```
