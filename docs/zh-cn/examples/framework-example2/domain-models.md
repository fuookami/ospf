# 复杂示例 2：上下文模型索引

[English](/examples/framework-example2/domain-models)

本索引是 `framework_demo/demo2` 下 11 个有界上下文的文档边界。每个链接页都按领域模型模板的顺序描述一个上下文契约；索引只承担导航和模式说明，变量、中间值、断言、约束及目标函数的细节以各上下文页面为准。

## 1. 上下文图与导航

上下文依赖方向为：

```text
aircraft → stowage → {mac, airworthiness_security, soft_security,
                      mac_optimization, express_effectiveness,
                      loading_effectiveness, redundancy,
                      recommended_weight_equalization, payload_maximization}
```

`aircraft` 提供飞机、甲板、燃油、舱位和邻接数据；`stowage` 消费这些配置并负责货物到舱位的分配及装载表达式。其余上下文消费共享的装载值，分别负责平衡、安全、效能或载荷职责。

## 2. 上下文模型页面

| 上下文 | 职责 | 本地模型 |
| --- | --- | --- |
| `aircraft` | 飞机、甲板、舱位、燃油、ULD 和邻接数据 | [飞机](domain-aircraft/domain-model) |
| `stowage` | 货物-舱位分配、调整、装载数量、重量及核心装载限制 | [装载分配](domain-stowage/domain-model) |
| `mac` | 扭矩、CLIM、指数和 MAC 中间值 | [平均气动弦（MAC）](domain-mac/domain-model) |
| `airworthiness_security` | 密度、累积/区域载荷、载荷、总重量、包络线、配平和 CLIM 限制 | [适航安全](domain-airworthiness_security/domain-model) |
| `soft_security` | 空舱位、舱门、空载分离和压舱物等软偏好 | [软安全](domain-soft_security/domain-model) |
| `mac_optimization` | 纵向/横向平衡及水平安定面限制与目标 | [MAC 优化](domain-mac_optimization/domain-model) |
| `express_effectiveness` | 必须发运和货物优先级顺序 | [快件效能](domain-express_effectiveness/domain-model) |
| `loading_effectiveness` | 同源/同目的地邻接、装载顺序、复称、拖车和序列策略 | [装载效能](domain-loading_effectiveness/domain-model) |
| `redundancy` | 冗余和实验纵向平衡限制 | [冗余](domain-redundancy/domain-model) |
| `recommended_weight_equalization` | 货物顺序、优先级预约及建议载重量偏差 | [建议载重量均衡](domain-recommended_weight_equalization/domain-model) |
| `payload_maximization` | 最大载荷限制和载荷目标 | [载荷最大化](domain-payload_maximization/domain-model) |

每个页面都提供中文或英文镜像链接；父页面负责说明各模式的精确注册边界。

## 3. 模式注册说明

| 模式 | 普通模型中的上下文 | 边界 |
| --- | --- | --- |
| `LoadingOrder` | `aircraft` | 仅导出配置，不注册元模型或求解 |
| `FullLoad` | `stowage`、`mac`、`airworthiness_security`、`soft_security`、`mac_optimization`、`express_effectiveness`、`loading_effectiveness` | 适航上下文可能移到 Benders 子问题 |
| `Predistribution` | FullLoad 上下文加 `redundancy` | 装载顺序管线受模式控制 |
| `WeightRecommendation` | `stowage`、`mac`、`airworthiness_security`、`express_effectiveness`、`recommended_weight_equalization`、`payload_maximization` | 推荐偏差和载荷目标是模式专属 |

上下文中存在类或模型文件，并不等同于约束已经生效；只有所选模式的管线生成器返回的管线才会进入该模式的模型。
