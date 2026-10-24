# ospf-rust-framework-csp2d

:us: [English](README.md) | :cn: 简体中文

## 简介

`ospf-rust-framework-csp2d` 是 Kotlin `ospf-kotlin-framework-csp2d` 的 Rust 迁移。它沿用
[`ospf-rust-framework-csp1d`](../ospf-rust-framework-csp1d/README_ch.md) 的 CSP 语义（产品、需求、图案、
方案、良率、余料、板长分配、列生成），同时采用
[`ospf-rust-framework-bpp2d`](../ospf-rust-framework-bpp2d/README_ch.md) 的泛型分层纪律：领域层泛型于 `V`，
求解器数值类型只出现在建模边界与应用层。

二维几何与 BPP2D 共用 [`ospf-rust-framework-geometry2d`](../ospf-rust-framework-geometry2d/README_ch.md)。

## 作用范围

两种供给模式，一套模型：

| 模式 | 问题类别 | 物料模型 | 决策变量 |
| --- | --- | --- | --- |
| 卷材套裁 | 条带排样（幅宽固定，长度待定） | `width` 固定、`length = None` | 消耗长度与排样 |
| 定尺板材 | 二维装箱 | `width × height` 固定 | 板材张数与排样 |

裁切工艺可配置，且与模式正交：

| 工艺 | 含义 | 设备 |
| --- | --- | --- |
| `Freeform` | 任意排样，只需放置坐标 | 激光、水刀、CNC |
| `GuillotineTwoStage` | 先切条再切件，输出切分树 | 剪板机、开料锯 |
| `GuillotineThreeStage` | 切条 → 切块 → 切件，输出切分树 | 剪板机、开料锯 |

异形件（非矩形件）明确不在范围内。

## 模块结构

| Rust 模块 | Kotlin 边界 | 职责 |
| --- | --- | --- |
| [`src/infrastructure`](src/infrastructure) | `csp2d-infrastructure` | 裁切区域、刀缝、断头台刀线与切分树、裁切工艺与错误。 |
| [`src/domain/material`](src/domain/material) | `csp2d-domain-material-context` | 双模式物料、产品与需求、裁切图案与裁切方案。 |
| [`src/domain/pattern_generation`](src/domain/pattern_generation) | `csp2d-domain-pattern-generation-context` | 图案生成请求与货架式生成器。 |
| [`src/domain/produce`](src/domain/produce) | `csp2d-domain-produce-context` | 主问题：图案使用量、需求覆盖、物料可用量、目标项。 |
| [`src/domain/yield`](src/domain/yield) | `csp2d-domain-yield-context` | 欠产与超产分析、超产上界。 |
| [`src/domain/wasting_minimization`](src/domain/wasting_minimization) | `csp2d-domain-wasting-minimization-context` | 余料面积与余料率分析、余料惩罚项。 |
| [`src/application`](src/application) | `csp2d-application` | 问题定义、MILP 路径与列生成路径。 |

## 切分树语义

断头台方案不是一组坐标，而是一棵**切分树**：内部结点是贯穿刀，叶子是最终料块。刀线的级数是它的
切割层号：根刀线为第 1 级，其下的刀线必须恰为第 2 级，依此类推。因此 2 级工艺恰好等于「先切条、
再切件」，3 级工艺等于「切条 → 切块 → 切件」。

`CutTree2::validate` 会拒绝：区域切分不一致（含刀缝带）、超出工艺级数上限、子刀线层号不正确。

货架式生成器构造的是「级数等于层级」的平衡树。所需深度超过工艺上限时不构造切分树，图案随之无法
通过校验——这是显式的能力边界，而不是静默产出非法图案。

## 列生成与定价

列追加由「剩余需求生成候选 + 检验数筛选」两步驱动：

1. 按剩余需求生成候选图案。
2. 用检验数 `rc = 消耗量 × 单价 − Σ 覆盖系数 × π` 筛选，只把 `rc < -tolerance` 的候选按升序加入。

需要明确的是：影子价格 `π` 目前由**覆盖缺口比例估计**，不是 LP 对偶。因此这是 reduced-cost pricing
的工程近似，不是严格意义上的定价证书；接入真实 LP 对偶后即可替换，接口已经就位。

## 泛型数值边界

1. 领域层泛型于 `V` 并保留编译时单位 `U`。
2. 坐标、尺寸与面积使用 `Quantity<V, U>`；无量纲值使用裸 `V`。
3. 求解器数值类型只出现在 `produce`（建模边界）与应用层。
4. 泛型代码不得调用 `to_f64()`。

## 公开 API

| API | 职责 | 稳定性 |
| --- | --- | --- |
| `Region2` / `CutTree2` / `CuttingProcess2` | 裁切几何、断头台切分树与工艺能力。 | migration |
| `Material2` / `Product2` / `Pattern2` / `CuttingPlan2` | 双模式领域模型。 | migration |
| `ShelfPatternGenerator2` | 带刀缝节距的构造式图案生成。 | migration |
| `Csp2dMasterProblem` | 主问题建模。 | migration |
| `Csp2dSolvers` | MILP 路径与列生成路径。 | migration |
| `Csp2dPricingConfig` / `improving_columns` | 检验数定价与改进列筛选。 | migration |
| `SheetAssignment` | 定尺张数 / 卷材段高的分配口径。 | migration |
| `CostarFiller` | 副产物填充余料。 | migration |
| `Csp2dMasterSolver` | 可注入的主问题求解接口与内置启发式。 | migration |

## 相关文档

- [ospf-rust README](../README_ch.md) 列出全部 crate 及其 Kotlin 边界。
- [csp1d](../ospf-rust-framework-csp1d/README_ch.md) 是成熟的一维参考。
- [geometry2d](../ospf-rust-framework-geometry2d/README_ch.md) 提供共享二维几何。
- [csp2d.md](csp2d.md) 记录迁移交接与已知差距。

## 本地验证

```powershell
$env:RUSTC_BOOTSTRAP = "1"
cargo test -p ospf-rust-framework-csp2d
```
