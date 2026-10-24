# ospf-rust-framework-geometry2d

:us: [English](README.md) | :cn: 简体中文

## 简介

`ospf-rust-framework-geometry2d` 是 `ospf-rust` 的共享二维量纲几何内核。它被抽取出来，使装箱框架
`ospf-rust-framework-bpp2d` 与套裁框架 `ospf-rust-framework-csp2d` 复用同一套量纲化二维几何，
而不是各自维护一份。

本 crate 与求解器无关：只依赖 `ospf-rust-base`、`ospf-rust-math` 与 `ospf-rust-quantities`。

`Box2Geometry::new` 与 `of_rectangle` 是纯构造方法，支持运行时单位及原域高精度值，
不要求 `Float`。独立的 `rectangle_overlapped` 路径只依赖原域有序加法，要求量纲及
比例尺相同，贴边不算重叠；单位／形状不兼容或数值不可比较时返回 `None`。
调用方须先校验有限正尺寸；其他形状／包含 API 保留原有 trait 边界。

## 作用范围

| 领域 | 类型 |
| --- | --- |
| 量值算术 | `quantity_plus_safe`、`quantity_minus_safe`、`quantity_multiply`、`quantity_ord_safe`/`quantity_ord_or_null`、`quantity_greater`/`quantity_less`/`quantity_greater_or_equal`/`quantity_less_or_equal`、`quantity_zero_of`、`quantity_of`、`quantity_scaled_by`、`quantity_sum`、`sum_of_quantity`、`sorted_by_quantity`、`sorted_by_descending_quantity`、`count_fitting`、`scaled_quantity`、`quantity_max_safe`/`quantity_min_safe`、`quantity_max_of`、`is_same_dimension` |
| 点与向量 | `QuantityPoint2`、`QuantityVector2`、`point2_of`、`vector2_of` |
| 形状 | `PackingShape2`（`Rectangle`/`Circle`）、`RectanglePackingShape2`、`CirclePackingShape2`、`PackingShape2Type`、`Shape2Candidate`、`candidate_shapes`、`rectangle_shape2`、`circle_shape2` |
| 放置与包含 | `Placement2`、`Box2Geometry` |
| 板材几何 | `Sheet2Geometry`、`Sheet2Shape`、`sheet2_shape`、`origin_box` |
| 刀缝 | `inflated_by_shape`、`inflated_by_placement`、`pitch_of`、`zero_spacing_of` |
| 面积与边界 | `area_of`、`bounding_width`、`bounding_height`、`bounding_area`、`min_x`、`min_y`、`max_x`、`max_y`、`inside_sheet` |

明确非目标：

1. 业务 DTO、请求协议、租户上下文或公式语言。
2. solver backend 及其安装与许可。
3. 外部数据集格式与读取；CSV schema 属于使用方 framework crate。

## 面积口径

`area` 是实际占用面积，`bounding_area` 是外接矩形面积：

1. 矩形件：`area = width × height`，`bounding_area = width × height`。
2. 圆形件：`area = πr²`，`bounding_area = 4r²`。

目标函数与 KPI 使用 `area`；网格生成使用包围尺寸。

## 重叠语义

重叠统一为**内部重叠**：相切**不算**重叠。矩形贴边、圆与圆相切、圆与矩形边界相切都视为不重叠。

这是网格排样的前提——若把相切判为重叠，任何规则网格都无法产出。量值库底层把相切判为重叠，
因此 `Box2Geometry::overlapped` 用严格不等式重新实现了分派。

## 刀缝

刀缝通过**膨胀参与重叠判定的形状**来表达：

1. `inflated_by_shape`：矩形宽高各加 `spacing`；圆形直径加 `spacing`。
2. `pitch_of` 返回包围尺寸加间距，使相邻件的真实间隙恰为间距。
3. 几何校验必须**双向膨胀**：只有把任一侧膨胀后仍相交，才说明真实间隙小于刀缝。单向膨胀会漏掉
   另一侧的间隙不足。

## 泛型数值边界

1. 所有类型泛型于 `V`，并保留编译时单位 `U`。
2. 坐标、尺寸与面积使用 `Quantity<V, U>`；无量纲值使用裸 `V`。
3. 面积由 `quantity_multiply` 产生，量纲按基础幂次相加合成，因此长度平方不会被算成立方。
4. 泛型代码不得调用 `to_f64()`；计数与缩放通过 `count_fitting` 与 `scaled_quantity` 表达。

## 使用方

| 框架 | 用途 |
| --- | --- |
| `ospf-rust-framework-bpp2d` | 矩形与圆形件的二维装箱 |
| `ospf-rust-framework-csp2d` | 矩形件的二维套裁 |

## 本地验证

```powershell
$env:RUSTC_BOOTSTRAP = "1"
cargo test -p ospf-rust-framework-geometry2d
```

## 相关文档

- [ospf-rust README](../README_ch.md) 列出全部 crate 及其 Kotlin 边界。
- [`tests/geometry2d_contract.rs`](tests/geometry2d_contract.rs) 固化面积、重叠、刀缝与包含契约。
