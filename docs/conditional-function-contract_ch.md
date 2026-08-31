# 条件函数契约

[English](conditional-function-contract.md) | 简体中文

## 稳定命名

- `IfFunction` 保留为旧三元表达式：`condition != 0 ? then : else`。
- `IfElseFunction` 是首选三元函数，条件必须是显式二值变量。
- `ConditionalIndicatorFunction` 是可注册的关系指示器，必须提供 `ConditionRelation`、正的 `strict_boundary` 和有限的 `ConditionBounds`。
- `ConditionalIfFunction` 是不注册模型的关系描述器，供纯分类 API 使用。
- `semantic::if_` 和 `semantic::if_named` 构造显式范围驱动指示器；`if_legacy` 保留原有阈值二值化行为。
- `IfInFunction` 保留离散集合成员语义；`IfInRangeFunction` 是独立的闭区间描述器，`RegisterableIfInRangeFunction` 会注册两侧指示器和 AND 结果。
- `IfThenConstraintFunction` 保留旧不等式蕴含别名；`ConditionalThenFunction` 会注册条件值结果，假分支为零、真分支等于 `then_poly`；非恒定 `then_poly` 必须提供显式有限范围。
- `ConditionalImplyFunction` 是范围驱动、可注册的蕴含函数，前件为假时会门控后件行；`imply_constraint` 保留为明确的旧 Big-M 入口。
- `SigmoidStepFunction` 是可注册的关系阶跃形式；`SigmoidFunction` 仍是连续 PWL 形式。

## 关系语义

令 `d = lhs - rhs`。只有下列真、假分支区域可判定，间隔内的值为 `Undefined`。

| 关系 | 真 | 假 |
| --- | --- | --- |
| `Greater` | `d >= g` | `d <= 0` |
| `GreaterEqual` | `d >= 0` | `d <= -g` |
| `Less` | `d <= -g` | `d >= 0` |
| `LessEqual` | `d <= 0` | `d >= g` |

`g` 是正的业务 `strict_boundary`，不是求解器可行性容差。通用一元指示器刻意拒绝等式关系。

## 注册规则

可注册关系指示器绝不推断默认 Big-M。调用方必须提供覆盖条件多项式的有限有序范围；完全落在 `Undefined` 间隔内的范围会被拒绝。范围完全落在单一分支时，两个辅助变量都会被固定；范围跨越分支时，生成两条范围驱动约束，并把稳定结果变量连接到内部指示变量。

`evaluate` 返回 `None` 可能表示 `Undefined`，也可能表示输入不可用；需要区分时使用 `classify`。蕴含的前件为假时短路为真，范围驱动蕴含会在该情况下松弛后件行。

## 离散条件与原子性

非单位 `delta` 不能由调用方声明一个步长直接通过。`ConditionalIndicatorFunction::from_discrete_condition` 会先从线性多项式和已注册 token 元数据推导 `DiscreteConditionLatticeProof`，再创建可注册指示器：所有参与变量都是整数类型，所有系数都是有限整数，步长为系数绝对值的 gcd，常数项按该步长归一化为余数。关系相关校验会同时检查 `delta`、`strict_boundary` 和零两侧最近格点距离。

`MetaModel::add_symbols` 与 `register_combination` 在同一事务内提交；任一项失败时，事务会恢复 token、符号和约束；`BasicModel` 会使旧缓存上下文失效，并根据恢复后的 token 表重建并重新绑定缓存，因此不会保留过期的缓存引用。`MutableTokenList::try_add_tokens` 与 `MutableTokenTable::register_batch` 不再提供逐项注册的默认实现。本仓库的 `VecTokenList`/`VecTokenTable` 及 `ConcurrentTokenList`/`ConcurrentTokenTable` 写锁路径已提供原子实现；第三方 trait 实现者必须新增显式的原子事务实现，这属于源码迁移影响。`try_add_tokens` 会预检变量 ID、名称和已分配 solver index，失败时不写入任何 token，且保留已有 token 求解结果缓存。旧的无返回值 `add_tokens` 入口保留源兼容；新代码必须使用 `try_add_tokens` 观察注册失败。

`ConcurrentTokenList` 与 `ConcurrentTokenTable` 通过 `read()` guard 暴露真实 trait 视图。对于 `ConcurrentTokenTable`，创建视图时应声明并持有 guard：`let guard = concurrent.read(); let view: &dyn TokenTable<_> = &*guard;` 对应的 `ConcurrentTokenList` 视图同样使用 `let guard = concurrent.read(); let view: &dyn TokenList<_> = &*guard;`。它们不会直接实现返回借用 token 引用的 trait，因为那会在引用使用前释放锁。

旧 Big-M 工具会拒绝非有限、零和负值；小的正数会被提升到最小稳定值。`IfInRangeFunction::new` 只接受同一单变量的 `x - lower >= 0` 和 `upper - x >= 0`，并校验两侧有限范围与端点顺序。
