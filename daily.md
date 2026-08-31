# 2026-03-30 clone 优化记录（ospf-rust-math）

## 当前状态
- 已完成多轮 `clone()` 优化，`cargo test -p ospf-rust-math` 持续通过。
- `cargo clippy -p ospf-rust-math -- -W clippy::redundant_clone -W clippy::clone_on_copy` 持续无目标告警。

## 本轮确认的剩余优化空间（按优先级）
1. 高收益低风险：`MulAssign<T>` / `DivAssign<T>` 中对 `rhs.clone()` 的重复克隆
   - 重点位置：
     - `src/symbol/polynomial/canonical.rs`（值版 `MulAssign<T>` / `DivAssign<T>`）
     - `src/symbol/polynomial/quadratic.rs`（值版 `MulAssign<T>` / `DivAssign<T>`）
   - 方案：
     - 为值版实现补 `for<'a> MulAssign<&'a T>` / `for<'a> DivAssign<&'a T>` 约束，
     - 将逐项 `rhs.clone()` 改为对 `&rhs` 运算，减少每个 monomial 一次克隆。

2. 中收益：`&Polynomial` 分支常数项 `clone()` 的引用化
   - 重点位置：
     - `src/symbol/polynomial/linear.rs`
     - `src/symbol/polynomial/quadratic.rs`
   - 方案：
     - 在可行处补 `AddRef/SubRef` 约束，
     - 将 `constant.clone() +/- rhs.constant.clone()` 改为 `T::add_ref / T::sub_ref`。

3. 中收益但约束影响更大：`Evaluate` 路径常数起始拷贝
   - 重点位置：
     - `src/symbol/polynomial/canonical.rs` 的 `evaluate/partial_evaluate/evaluate_ordered`
   - 风险说明：
     - 直接引入 `AddRef` 会影响 `symbol/inequality/canonical.rs` 等泛型调用边界；
     - 需同步调整调用端 trait bound，改动面更大。

4. 低收益或基本必要：符号键与幂次容器复制
   - 典型如 `powers.insert(rhs.clone(), E::one())`、`m.powers.clone()`；
   - 多数是所有权需求，除非改数据结构（借用键/Cow），否则收益有限。

## 建议下一步
- 优先执行第 1 项（值版 `MulAssign/DivAssign` 去克隆），可安全落地且收益最直接。
