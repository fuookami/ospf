# 三维装箱 BPP3D

货物具有长宽高和重量，需要选择箱型、朝向和放置位置，最后得到可检查、可展示的装载方案。第一课只处理长方体，不从圆柱与复杂支撑开始。

## 最少需要理解的对象

| 业务概念 | API | 输入或输出 |
| --- | --- | --- |
| 货物与包装 | `Item` / `Package` | 尺寸、重量、物料和允许朝向 |
| 箱型 | `BinType` | 几何、承重和类型身份 |
| 箱层候选 | `BinLayer` | 箱型、需求覆盖与放置追踪 |
| 应用服务 | `ColumnGenerationApplicationService` | 主问题、生成、最终选择与装载分析 |
| 终态验证 | `PackingGeometryGuard` | 越界、重叠和对应形状路径的合法性 |

候选列描述可选的装载结构，不是已经完成的最终装箱。优化选择、实际放置和报告都要接通。

## 从哪个入口调用？

Kotlin 的应用请求、标准执行器和 packing analyzer 组成入口。Rust 提供 materialized / CSV 路径，以 RMP 与 final executor 注入后端。不能只把 Kotlin 类型改成蛇形命名就当作 Rust 实现。

先用固定箱型和少量 seed 验证装配，再引入自动候选、多箱型与目标配置。

## 能力边界

- 圆柱支持依赖轴向以及生成、支撑、终态验证路径，不代表任意三维旋转都支持。
- 装载率应使用对应形状的真实体积，不能把外接体积当实占体积。
- 外部 renderer 的实现不在框架内；输出 DTO 不等于已经启动可视化页面。
- `finalSolved`、packing snapshot 和定价诊断应一起检查，不能只看非空候选列表。

## 学习路线

1. 完成[环境准备](../setup)。
2. 跟随[第一次三维装箱](./first-loading)，学习从候选选择到最终放置。
3. 增加箱型、重量和费用，明确目标权重的业务含义。
4. 综合航空装载可参考[复杂示例 2](/zh-cn/examples/framework-example2)，但行业安全规则不因使用 BPP3D 而自动具备。

## 接口参考

- [Kotlin 入口与形状路径矩阵](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-bpp3d/README_ch.md)
- [Rust 入口与领域契约](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-bpp3d/README_ch.md)

[返回场景选型](../)

## 继续实践

下一课：[多箱型与目标配置](./business-scenario)。将场景迁入自己的应用前，完成[业务接入检查](../integration)。
