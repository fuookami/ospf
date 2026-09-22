//! 二次函数组合验收测试 / Quadratic function composition acceptance tests
//!
//! 覆盖 P5（二次函数组合）的三项交付：
//! Covers the three P5 deliverables (quadratic function composition):
//!
//! 1. 共享线性输入提升适配与"组合不产生三次或更高次项"的不变式；
//!    the shared linear-input lifting adapter and the "composition never produces degree 3 or
//!    higher" invariant;
//! 2. 首个二次包装函数 `QuadraticAbsFunction` 的分支指示列、非对称 Big-M、辅助列注册与求值；
//!    branch indicator column, asymmetric Big-M, helper registration and evaluation of the first
//!    quadratic wrapper `QuadraticAbsFunction`;
//! 3. 二次模型生命周期：`DeferredNativeFirst` 下待展开的延迟结构必须先物化，且物化后的行与
//!    EAGER 路径逐行一致。
//!    the quadratic model lifecycle: pending deferred structures must be materialized first under
//!    `DeferredNativeFirst`, and the materialized rows must match the EAGER path row by row.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use ospf_rust_core::model::intermediate::QuadraticTetradModel;
use ospf_rust_core::model::{
    ConstraintRelation, FunctionExpansionPolicy, LinearConstraint, LinearInequality, MetaModel,
    QuadraticInequality,
};
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial, Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::{
    AbsBranchBigM, AbsFunction, QuadraticAbsFunction, guard_quadratic_composition_degree,
    has_quadratic_monomials, lift_linear_input, lift_linear_input_checked,
};
use ospf_rust_core::symbol::{
    FunctionSymbol, IntermediateSymbol, LinearIntermediateSymbol, QuadraticFunctionSymbol,
};
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId, VariableRange};

// ============================================================================
// 手工求值工具 / Manual evaluation helpers
// ============================================================================

/// 手工计算线性不等式左端值 / Manually evaluate the left-hand side of a linear inequality
fn linear_lhs(inequality: &LinearInequality<f64>, values: &HashMap<usize, f64>) -> f64 {
    let mut value = *inequality.polynomial.constant_term();
    for monomial in inequality.polynomial.monomials() {
        value +=
            *monomial.coefficient() * values.get(&monomial.var_index()).copied().unwrap_or(0.0);
    }
    value
}

/// 手工判断线性不等式是否成立 / Manually check whether a linear inequality holds
fn linear_holds(inequality: &LinearInequality<f64>, values: &HashMap<usize, f64>) -> bool {
    let lhs = linear_lhs(inequality, values);
    match inequality.relation {
        ConstraintRelation::LessEqual => lhs <= inequality.rhs + 1e-9,
        ConstraintRelation::GreaterEqual => lhs + 1e-9 >= inequality.rhs,
        ConstraintRelation::Equal => (lhs - inequality.rhs).abs() <= 1e-9,
    }
}

/// 手工计算二次不等式左端值 / Manually evaluate the left-hand side of a quadratic inequality
fn quadratic_lhs(inequality: &QuadraticInequality<f64>, values: &HashMap<usize, f64>) -> f64 {
    let mut value = *inequality.polynomial.constant();
    for monomial in inequality.polynomial.monomials() {
        let coefficient = *monomial.coefficient();
        let first = values.get(&monomial.var_index1()).copied().unwrap_or(0.0);
        value += match monomial.var_index2() {
            Some(second) => {
                coefficient * first * values.get(&second).copied().unwrap_or(0.0)
            }
            None => coefficient * first,
        };
    }
    value
}

/// 手工判断二次不等式是否成立 / Manually check whether a quadratic inequality holds
fn quadratic_holds(inequality: &QuadraticInequality<f64>, values: &HashMap<usize, f64>) -> bool {
    let lhs = quadratic_lhs(inequality, values);
    match inequality.relation {
        ConstraintRelation::LessEqual => lhs <= inequality.rhs + 1e-9,
        ConstraintRelation::GreaterEqual => lhs + 1e-9 >= inequality.rhs,
        ConstraintRelation::Equal => (lhs - inequality.rhs).abs() <= 1e-9,
    }
}

/// 取线性不等式中指定列的系数 / Coefficient of the given column in a linear inequality
fn column_coefficient(inequality: &LinearInequality<f64>, column: usize) -> f64 {
    inequality
        .polynomial
        .monomials()
        .iter()
        .find(|monomial| monomial.var_index() == column)
        .map(|monomial| *monomial.coefficient())
        .unwrap_or(0.0)
}

/// 按约束名查找一行 / Find a row by constraint name
fn row<'a>(rows: &'a [LinearConstraint<f64>], name: &str) -> &'a LinearInequality<f64> {
    &rows
        .iter()
        .find(|constraint| constraint.name == name)
        .unwrap_or_else(|| panic!("missing constraint row `{name}`"))
        .inequality
}

/// 用 `VecTokenList` 组装一个带取值的令牌 / Build a token with a solved result
fn solved_token(id: usize, name: &str, index: usize, value: f64) -> Token<f64> {
    let token = Token::from_generic(
        ContinuousVariableItem::create(VariableId::standalone(id), name),
        index,
    );
    token.set_result(value);
    token
}

/// 包装函数的列索引映射：结果列 → 5、分支指示列 → 6、二次输入的桥接列 → 7。
///
/// 列编号刻意避开输入多项式使用的 0/1，保证合成列空间自洽（桥接列不会与输入列撞号）。
/// Column-index map of the wrapper: result → 5, branch indicator → 6 and, for a quadratic
/// input, bridge → 7. The numbers deliberately avoid the 0/1 used by the input polynomials so the
/// synthetic column space stays coherent (the bridge column never collides with an input column).
fn symbol_indices(function: &QuadraticAbsFunction<f64>) -> HashMap<usize, usize> {
    let mut indices = HashMap::from([
        (function.result_variable().id().unique_id() as usize, 5usize),
        (function.side_variable().id().unique_id() as usize, 6usize),
    ]);
    if function.has_quadratic_input() {
        indices.insert(function.bridge_variable().id().unique_id() as usize, 7usize);
    }
    indices
}

// ============================================================================
// 1. 二次项：真正的二次输入产出二次约束 / Genuine quadratic input yields a quadratic constraint
// ============================================================================

#[test]
fn quadratic_input_produces_one_quadratic_bridge_and_linear_branch_rows() {
    // x * y + 3：真正的二次输入 / x * y + 3: a genuine quadratic input
    let input = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 1)], 3.0);
    let function = QuadraticAbsFunction::new(1, "abs_xy", input);
    assert!(function.has_quadratic_input());

    let mut tokens = Vec::new();
    function.register_tokens(&mut tokens).unwrap();
    // 桥接列 + 结果列 + 分支指示列 / bridge column + result column + branch indicator column
    assert_eq!(tokens.len(), 3);
    let names: Vec<String> = tokens.iter().map(|token| token.name().to_string()).collect();
    assert!(names.contains(&"abs_xy_bridge_lin_y".to_string()));
    assert!(names.contains(&"abs_xy_abs".to_string()));
    assert!(names.contains(&"abs_xy_side".to_string()));

    let symbol_to_index = symbol_indices(&function);
    let bridge_index = 7usize;

    // 二次项确实产出二次约束：恰好一条桥接等式 q - bridge = 0。
    // The quadratic term really produces a quadratic constraint: exactly one bridge equality
    // `q - bridge = 0`.
    let quadratic_rows = function
        .quadratic_mechanism_constraints(&symbol_to_index)
        .unwrap();
    assert_eq!(quadratic_rows.len(), 1);
    let bridge_row = &quadratic_rows[0];
    assert_eq!(bridge_row.name, "abs_xy_bridge_quad_eq");
    assert_eq!(bridge_row.inequality.relation, ConstraintRelation::Equal);
    assert_eq!(bridge_row.inequality.rhs, 0.0);
    let polynomial = &bridge_row.inequality.polynomial;
    assert!(has_quadratic_monomials(polynomial), "bridge equality keeps the x*y term");
    assert_eq!(polynomial.constant(), &3.0);
    assert!(polynomial
        .monomials()
        .iter()
        .any(|monomial| monomial.var_index2() == Some(1) && monomial.var_index1() == 0));
    // 桥接列以 -1 的一次项进入桥接等式 / the bridge column enters with the linear coefficient -1
    let bridge_linear_coefficient = polynomial
        .monomials()
        .iter()
        .find(|monomial| monomial.var_index2().is_none() && monomial.var_index1() == bridge_index)
        .map(|monomial| *monomial.coefficient())
        .unwrap_or(0.0);
    assert_eq!(bridge_linear_coefficient, -1.0);

    // 组合次数不变式：桥接列只以一次项参与组合，代入后不会出现三次或更高次项。
    // Composition degree invariant: the bridge column only takes part as a linear term, so no
    // degree-3-or-higher term appears after substitution.
    for monomial in polynomial.monomials() {
        let Some(second) = monomial.var_index2() else {
            continue;
        };
        assert_ne!(monomial.var_index1(), bridge_index);
        assert_ne!(second, bridge_index);
    }
    guard_quadratic_composition_degree(polynomial, &HashSet::from([bridge_index]), "abs_xy")
        .expect("the emitted bridge equality keeps the bridge column linear");

    // 分支行仍是四条线性行 / the branch rows remain four linear rows
    assert_eq!(
        function.mechanism_constraints(&symbol_to_index).unwrap().len(),
        4
    );

    // 求值与约束一致：q = 2 * (-3) + 3 → x * y + 3 = -3 → |q| = 3。
    // Evaluation agrees with the constraints: q = x * y + 3 = -3 → |q| = 3.
    let mut token_list = VecTokenList::new();
    token_list.add_token(solved_token(70_101, "x", 0, 2.0));
    token_list.add_token(solved_token(70_102, "y", 1, -3.0));
    assert_eq!(function.calculate_value(&token_list, false), Some(3.0));
    assert_eq!(
        function.prepare(&HashMap::from([(0usize, 2.0), (1usize, -3.0)])),
        Some(3.0)
    );
}

// ============================================================================
// 2. 纯线性退化：二次项为空且没有多余的二次 helper 列 / Pure-linear degeneracy
// ============================================================================

#[test]
fn lifting_a_linear_input_stays_degenerate_without_quadratic_helper_columns() {
    let linear = Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0);

    let lifted = lift_linear_input(&linear);
    assert!(!has_quadratic_monomials(&lifted), "lifted input keeps no quadratic monomial");
    assert_eq!(lifted.len(), 1);
    assert_eq!(lifted.constant(), &1.0);
    assert!(lifted
        .monomials()
        .iter()
        .all(|monomial| monomial.var_index2().is_none()));
    let checked = lift_linear_input_checked(&linear).unwrap();
    assert_eq!(checked.monomials().len(), lifted.monomials().len());
    assert_eq!(checked.constant(), lifted.constant());

    // 线性输入包装：没有桥接列，也没有二次约束。
    // Wrapping a linear input: no bridge column and no quadratic constraint.
    let function = QuadraticAbsFunction::from_linear(2, "abs_linear", linear);
    assert!(!function.has_quadratic_input());
    let mut tokens = Vec::new();
    function.register_tokens(&mut tokens).unwrap();
    assert_eq!(tokens.len(), 2, "only the result and the branch indicator column");
    let symbol_to_index: HashMap<usize, usize> = tokens
        .iter()
        .enumerate()
        .map(|(index, token)| (token.id().unique_id() as usize, index + 10))
        .collect();
    assert!(
        !symbol_to_index.contains_key(&(function.bridge_variable().id().unique_id() as usize)),
        "the degenerate lift must not register a quadratic helper column"
    );
    assert!(
        function
            .quadratic_mechanism_constraints(&symbol_to_index)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        function.mechanism_constraints(&symbol_to_index).unwrap().len(),
        4
    );
}

// ============================================================================
// 3. 常数项与嵌套 / Constant terms and nesting
// ============================================================================

#[test]
fn constant_terms_flow_into_the_branch_rows_with_consistent_signs() {
    // q = 2x + 3：常数项必须进入行的常数项，方向与输入符号一致。
    // q = 2x + 3: the constant must reach the row constants with the input's signs.
    let function = QuadraticAbsFunction::from_linear(
        3,
        "abs_constant",
        Linear::new(vec![LinearMonomial::new(2.0, 0)], 3.0),
    );
    let result_id = function.result_variable().id().unique_id() as usize;
    let side_id = function.side_variable().id().unique_id() as usize;
    let symbol_to_index = HashMap::from([(result_id, 1usize), (side_id, 2usize)]);
    let rows = function.mechanism_constraints(&symbol_to_index).unwrap();

    // y - q >= 0 → y - 2x - 3 >= 0；y + q >= 0 → y + 2x + 3 >= 0。
    // y - q >= 0 → y - 2x - 3 >= 0; y + q >= 0 → y + 2x + 3 >= 0.
    let ge_x = row(&rows, "abs_constant_abs_ge_x");
    assert_eq!(*ge_x.polynomial.constant_term(), -3.0);
    assert_eq!(column_coefficient(ge_x, 0), -2.0);
    let ge_neg_x = row(&rows, "abs_constant_abs_ge_neg_x");
    assert_eq!(*ge_neg_x.polynomial.constant_term(), 3.0);
    assert_eq!(column_coefficient(ge_neg_x, 0), 2.0);

    // 嵌套：内层线性 ABS 的输出（线性列）提升后交给外层二次 ABS，仍然不含二次项。
    // Nesting: the linear output column of an inner ABS is lifted and fed to an outer quadratic
    // ABS, which still carries no quadratic monomial.
    let inner = AbsFunction::new(
        4,
        "abs_inner",
        Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
    );
    let nested = lift_linear_input(&inner.to_linear_polynomial());
    assert!(!has_quadratic_monomials(&nested));
    let outer = QuadraticAbsFunction::new(5, "abs_outer", nested);
    assert!(!outer.has_quadratic_input());
    let mut outer_tokens = Vec::new();
    outer.register_tokens(&mut outer_tokens).unwrap();
    assert_eq!(outer_tokens.len(), 2);
    let outer_indices: HashMap<usize, usize> = outer_tokens
        .iter()
        .enumerate()
        .map(|(index, token)| (token.id().unique_id() as usize, index + 1))
        .collect();
    assert!(outer
        .quadratic_mechanism_constraints(&outer_indices)
        .unwrap()
        .is_empty());
    assert_eq!(
        outer.mechanism_constraints(&outer_indices).unwrap().len(),
        4
    );
}

// ============================================================================
// 4. 分支 gap：跨正负取值的非对称 Big-M / Branch gap: asymmetric Big-M across both signs
// ============================================================================

#[test]
fn branch_big_m_from_token_bounds_is_asymmetric_when_values_cross_zero() {
    // q = 2x + 1，x ∈ [-2, 3] → q ∈ [-3, 7]：
    // 正分支行需覆盖 max(0, -2 * lower) = 6，负分支行需覆盖 max(0, 2 * upper) = 14。
    // q = 2x + 1 with x in [-2, 3] → q in [-3, 7]: the positive branch row covers 6 and the
    // negative branch row covers 14.
    let x = ContinuousVariableItem::with_range(
        VariableId::standalone(71_001),
        "x",
        VariableRange::bounded(-2.0, 3.0),
    );
    let function = QuadraticAbsFunction::new(
        6,
        "abs_asymmetric",
        Quadratic::new(vec![QuadraticMonomial::new_linear(2.0, 0)], 1.0),
    );
    let result_id = function.result_variable().id().unique_id() as usize;
    let side_id = function.side_variable().id().unique_id() as usize;
    let symbol_to_index = HashMap::from([(result_id, 5usize), (side_id, 6usize)]);
    let tokens = vec![
        Token::from_generic(x, 0),
        Token::from_generic(function.result_variable().clone(), 5),
        Token::from_generic(function.side_variable().clone(), 6),
    ];

    let rows = function
        .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
        .unwrap();
    let pos_branch = row(&rows, "abs_asymmetric_abs_pos_branch");
    let neg_branch = row(&rows, "abs_asymmetric_abs_neg_branch");
    assert_eq!(pos_branch.rhs, 6.0);
    assert_eq!(column_coefficient(pos_branch, 6), 6.0);
    assert_eq!(neg_branch.rhs, 0.0);
    assert_eq!(column_coefficient(neg_branch, 6), -14.0);
}

// ============================================================================
// 5. 显式 Big-M 配置与策略回退 / Explicit Big-M configuration and policy fallback
// ============================================================================

#[test]
fn explicit_branch_big_m_overrides_inference_and_is_validated() {
    let input = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0);

    // 显式对称 Big-M / explicit symmetric Big-M
    let symmetric = QuadraticAbsFunction::with_big_m(8, "abs_symmetric", input.clone(), 25.0);
    let indices = symbol_indices(&symmetric);
    let rows = symmetric.mechanism_constraints(&indices).unwrap();
    assert_eq!(row(&rows, "abs_symmetric_abs_pos_branch").rhs, 25.0);
    assert_eq!(
        column_coefficient(row(&rows, "abs_symmetric_abs_pos_branch"), 6),
        25.0
    );
    assert_eq!(
        column_coefficient(row(&rows, "abs_symmetric_abs_neg_branch"), 6),
        -25.0
    );

    // 显式非对称分支 Big-M / explicit asymmetric branch Big-M
    let asymmetric = QuadraticAbsFunction::with_branch_big_m(
        9,
        "abs_asymmetric_config",
        input.clone(),
        AbsBranchBigM {
            positive_branch: 12.0,
            negative_branch: 30.0,
        },
    );
    let indices = symbol_indices(&asymmetric);
    let rows = asymmetric.mechanism_constraints(&indices).unwrap();
    assert_eq!(
        row(&rows, "abs_asymmetric_config_abs_pos_branch").rhs,
        12.0
    );
    assert_eq!(
        column_coefficient(row(&rows, "abs_asymmetric_config_abs_neg_branch"), 6),
        -30.0
    );

    // 无显式配置且无令牌上下文：回退到策略值 10^6。
    // No explicit configuration and no token context: falls back to the policy value 10^6.
    let fallback = QuadraticAbsFunction::new(10, "abs_fallback", input.clone());
    let indices = symbol_indices(&fallback);
    let rows = fallback.mechanism_constraints(&indices).unwrap();
    assert_eq!(row(&rows, "abs_fallback_abs_pos_branch").rhs, 1_000_000.0);
    assert_eq!(
        column_coefficient(row(&rows, "abs_fallback_abs_neg_branch"), 6),
        -1_000_000.0
    );

    // 非法显式 Big-M 必须报错，而不是写出退化约束。
    // An invalid explicit Big-M must error instead of writing degenerate rows.
    let zero_big_m = QuadraticAbsFunction::with_big_m(11, "abs_zero_m", input.clone(), 0.0);
    let indices = symbol_indices(&zero_big_m);
    assert!(zero_big_m.mechanism_constraints(&indices).is_err());
    let negative_branch = QuadraticAbsFunction::with_branch_big_m(
        12,
        "abs_negative_branch_m",
        input,
        AbsBranchBigM {
            positive_branch: 4.0,
            negative_branch: 0.0,
        },
    );
    let indices = symbol_indices(&negative_branch);
    assert!(negative_branch.mechanism_constraints(&indices).is_err());
}

// ============================================================================
// 6. 求值与实际约束一致 / Evaluation agrees with the actual constraints
// ============================================================================

#[test]
fn branch_rows_are_exactly_feasible_only_at_the_absolute_value() {
    // q = 2x + 3，x ∈ [-3, 7] → q ∈ [-3, 17]，M_pos = 6、M_neg = 34。
    // q = 2x + 3 with x in [-3, 7] → q in [-3, 17], M_pos = 6 and M_neg = 34.
    let x = ContinuousVariableItem::with_range(
        VariableId::standalone(72_001),
        "x",
        VariableRange::bounded(-3.0, 7.0),
    );
    let function = QuadraticAbsFunction::from_linear(
        13,
        "abs_grid",
        Linear::new(vec![LinearMonomial::new(2.0, 0)], 3.0),
    );
    let result_id = function.result_variable().id().unique_id() as usize;
    let side_id = function.side_variable().id().unique_id() as usize;
    let symbol_to_index = HashMap::from([(result_id, 1usize), (side_id, 2usize)]);
    let tokens = vec![
        Token::from_generic(x, 0),
        Token::from_generic(function.result_variable().clone(), 1),
        Token::from_generic(function.side_variable().clone(), 2),
    ];
    let rows = function
        .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
        .unwrap();
    assert_eq!(rows.len(), 4);

    for x_value in [-3.0_f64, -1.0, 0.0, 1.0, 7.0] {
        let q = 2.0 * x_value + 3.0;
        let expected = q.abs();
        for candidate in [expected, (expected - 1.0).max(0.0), expected + 1.0] {
            // 分支指示列任取 0/1，只有当结果列等于 |q| 时四条行才可能同时成立。
            // The branch indicator may be 0 or 1; all four rows can hold only when the result
            // column equals |q|.
            let feasible = [0.0_f64, 1.0].iter().any(|side| {
                let values = HashMap::from([
                    (0usize, x_value),
                    (1usize, candidate),
                    (2usize, *side),
                ]);
                rows.iter().all(|constraint| linear_holds(&constraint.inequality, &values))
            });
            assert_eq!(
                feasible,
                (candidate - expected).abs() <= 1e-9,
                "x={x_value}, q={q}, candidate={candidate}"
            );
        }
    }
}

#[test]
fn quadratic_bridge_and_branch_rows_are_jointly_exact_for_a_quadratic_input() {
    // q = x * y + 1，x ∈ [-3, 7]、y ∈ [2, 4] → q ∈ [-11, 29]。
    // q = x * y + 1 with x in [-3, 7] and y in [2, 4] → q in [-11, 29].
    let x = ContinuousVariableItem::with_range(
        VariableId::standalone(73_001),
        "x",
        VariableRange::bounded(-3.0, 7.0),
    );
    let y = ContinuousVariableItem::with_range(
        VariableId::standalone(73_002),
        "y",
        VariableRange::bounded(2.0, 4.0),
    );
    let function = QuadraticAbsFunction::new(
        14,
        "abs_quadratic_grid",
        Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 1)], 1.0),
    );
    let symbol_to_index = symbol_indices(&function);
    let tokens = vec![
        Token::from_generic(x, 0),
        Token::from_generic(y, 1),
        Token::from_generic(function.result_variable().clone(), 5),
        Token::from_generic(function.side_variable().clone(), 6),
        Token::from_generic(function.bridge_variable().clone(), 7),
    ];
    let rows = function
        .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
        .unwrap();
    let bridge_row = function
        .quadratic_mechanism_constraints(&symbol_to_index)
        .unwrap()
        .remove(0);

    for (x_value, y_value) in [(-3.0_f64, 2.0_f64), (-1.0, 4.0), (0.0, 3.0), (2.0, 2.0), (7.0, 4.0)] {
        let q = x_value * y_value + 1.0;
        let expected = q.abs();
        for candidate in [expected, (expected - 2.0).max(0.0), expected + 1.0] {
            let feasible = [0.0_f64, 1.0].iter().any(|side| {
                let values = HashMap::from([
                    (0usize, x_value),
                    (1usize, y_value),
                    (5usize, candidate),
                    (6usize, *side),
                    (7usize, q),
                ]);
                quadratic_holds(&bridge_row.inequality, &values)
                    && rows.iter().all(|constraint| linear_holds(&constraint.inequality, &values))
            });
            assert_eq!(
                feasible,
                (candidate - expected).abs() <= 1e-9,
                "x={x_value}, y={y_value}, q={q}, candidate={candidate}"
            );
        }
    }
}

// ============================================================================
// 7. helper/token 注册与结果范围收紧 / Helper registration and result range tightening
// ============================================================================

#[test]
fn helper_columns_are_registered_with_expected_names_and_ranges() {
    let mut model = MetaModel::<f64>::new("quadratic_abs_helpers");
    let x = ContinuousVariableItem::with_range(
        VariableId::standalone(74_001),
        "x",
        VariableRange::bounded(-3.0, 7.0),
    );
    let y = ContinuousVariableItem::with_range(
        VariableId::standalone(74_002),
        "y",
        VariableRange::bounded(2.0, 4.0),
    );
    let x_index = model.register_variable(x).unwrap();
    let y_index = model.register_variable(y).unwrap();

    let function = QuadraticAbsFunction::new(
        15,
        "abs_helpers",
        Quadratic::new(
            vec![QuadraticMonomial::new_quadratic(1.0, x_index, y_index)],
            0.0,
        ),
    );
    let result_id = function.result_variable().id();
    let side_id = function.side_variable().id();
    let bridge_id = function.bridge_variable().id();
    model.add_symbol(Arc::new(function)).unwrap();

    let mechanism = model.try_into_mechanism_model().unwrap();
    // 2 个输入列 + 桥接列 + 结果列 + 分支指示列 / 2 inputs + bridge + result + branch indicator
    assert_eq!(mechanism.as_basic().tokens().len(), 5);
    assert_eq!(mechanism.as_basic().constraints().len(), 4);
    assert_eq!(mechanism.as_basic().quadratic_constraints().len(), 1);

    let names: Vec<&str> = mechanism
        .as_basic()
        .tokens()
        .iter()
        .map(|token| token.name())
        .collect();
    assert!(names.contains(&"abs_helpers_bridge_lin_y"));
    assert!(names.contains(&"abs_helpers_abs"));
    assert!(names.contains(&"abs_helpers_side"));
    assert!(mechanism.find_token(result_id).is_some());
    assert!(mechanism.find_token(side_id).is_some());
    assert!(mechanism.find_token(bridge_id).is_some());

    // 结果列范围被二次输入域收紧：q = x * y ∈ [-12, 28] → |q| ∈ [0, 28]。
    // The result range is tightened by the quadratic input domain.
    let result_token = mechanism.find_token(result_id).unwrap();
    assert_eq!(result_token.variable.lower_bound(), Some(0.0));
    assert_eq!(result_token.variable.upper_bound(), Some(28.0));

    // 非对称分支 Big-M：M_pos = max(0, -2 * (-12)) = 24，M_neg = 2 * 28 = 56。
    // Asymmetric branch Big-M: M_pos = 24 and M_neg = 56.
    let side_index = mechanism.find_token(side_id).unwrap().solver_index;
    let rows = mechanism.as_basic().constraints();
    let pos_branch = row(rows, "abs_helpers_abs_pos_branch");
    assert_eq!(pos_branch.rhs, 24.0);
    assert_eq!(column_coefficient(pos_branch, side_index), 24.0);
    let neg_branch = row(rows, "abs_helpers_abs_neg_branch");
    assert_eq!(neg_branch.rhs, 0.0);
    assert_eq!(column_coefficient(neg_branch, side_index), -56.0);
}

// ============================================================================
// 8. 二次模型生命周期 / Quadratic model lifecycle
// ============================================================================

/// 构造同时含线性 ABS（延迟结构）与二次 ABS 的二次模型 / Build a quadratic model that mixes a
/// deferred linear ABS with an eagerly expanded quadratic ABS
fn lifecycle_quadratic_model(policy: FunctionExpansionPolicy) -> QuadraticTetradModel {
    let mut model = MetaModel::<f64>::new("quadratic_abs_lifecycle");
    model.set_function_expansion_policy(policy);

    let x = ContinuousVariableItem::with_range(
        VariableId::standalone(75_001),
        "x",
        VariableRange::bounded(-3.0, 7.0),
    );
    let y = ContinuousVariableItem::with_range(
        VariableId::standalone(75_002),
        "y",
        VariableRange::bounded(-2.0, 4.0),
    );
    let x_index = model.register_variable(x).unwrap();
    let y_index = model.register_variable(y).unwrap();

    // 线性 ABS 有求解器无关结构：非 EAGER 策略下它保持延迟，必须在转换前物化。
    // The linear ABS owns a solver-neutral structure: under a non-EAGER policy it stays deferred
    // and has to be materialized before conversion.
    let linear_abs = AbsFunction::new(
        16,
        "abs_lifecycle_linear",
        Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
    );
    model.add_symbol(Arc::new(linear_abs)).unwrap();

    // 二次 ABS 没有延迟结构，因此在两种策略下都走即时通用展开。
    // The quadratic ABS has no deferred structure, so it is eagerly expanded under both policies.
    let quadratic_abs = QuadraticAbsFunction::new(
        17,
        "abs_lifecycle_quadratic",
        Quadratic::new(
            vec![QuadraticMonomial::new_quadratic(1.0, x_index, y_index)],
            1.0,
        ),
    );
    model.add_symbol(Arc::new(quadratic_abs)).unwrap();

    let mechanism = model.try_into_mechanism_model().unwrap();
    if policy.is_deferred() {
        assert!(
            !mechanism.as_basic().deferred_functions().is_empty(),
            "a deferred policy must keep pending structures before conversion"
        );
    } else {
        assert!(mechanism.as_basic().deferred_functions().is_empty());
    }
    mechanism.into_quadratic_tetrad_model()
}

/// 归一化线性行：名称、右端项与排序后的稀疏系数 / Normalize linear rows: name, right-hand side
/// and sorted sparse coefficients
fn normalized_linear_rows(model: &QuadraticTetradModel) -> Vec<String> {
    let basic = &model.basic.linear;
    let mut rows = basic
        .constraint_names
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let mut entries = basic.A.rows[index].entries.clone();
            entries.sort_by_key(|(column, _)| *column);
            let coefficients = entries
                .iter()
                .map(|(column, coefficient)| format!("{column}:{coefficient:.12}"))
                .collect::<Vec<_>>()
                .join(",");
            format!("{name}|{:.12}|{coefficients}", basic.b[index])
        })
        .collect::<Vec<_>>();
    rows.sort();
    rows
}

/// 归一化二次行：名称、关系、右端项、常数项与排序后的单项式
/// Normalize quadratic rows: name, relation, right-hand side, constant and sorted monomials
fn normalized_quadratic_rows(model: &QuadraticTetradModel) -> Vec<String> {
    let mut rows = model
        .quadratic_constraints
        .iter()
        .enumerate()
        .map(|(index, constraint)| {
            let polynomial = &constraint.polynomial;
            let mut terms = polynomial
                .monomials()
                .iter()
                .map(|monomial| match monomial.var_index2() {
                    Some(second) => format!(
                        "{:.12}*x{}*x{}",
                        monomial.coefficient(),
                        monomial.var_index1(),
                        second
                    ),
                    None => format!("{:.12}*x{}", monomial.coefficient(), monomial.var_index1()),
                })
                .collect::<Vec<_>>();
            terms.sort();
            format!(
                "{}|{:?}|{:.12}|{:.12}|{}",
                model.quadratic_constraint_names[index],
                constraint.relation,
                constraint.rhs,
                polynomial.constant(),
                terms.join("+")
            )
        })
        .collect::<Vec<_>>();
    rows.sort();
    rows
}

#[test]
fn deferred_native_first_materializes_pending_structures_before_the_quadratic_model() {
    let eager = lifecycle_quadratic_model(FunctionExpansionPolicy::Eager);
    let deferred = lifecycle_quadratic_model(FunctionExpansionPolicy::DeferredNativeFirst);

    let eager_rows = normalized_linear_rows(&eager);
    let deferred_rows = normalized_linear_rows(&deferred);

    // 延迟结构必须已物化为通用 fallback：线性 ABS 的四条分支行出现在二次模型里。
    // The deferred structure must be materialized into the generic fallback: the four linear ABS
    // branch rows appear in the quadratic model.
    assert_eq!(eager_rows.len(), 8, "two ABS wrappers with four rows each");
    assert!(eager_rows
        .iter()
        .any(|row| row.starts_with("abs_lifecycle_linear_abs_pos_branch|")));
    assert!(eager_rows
        .iter()
        .any(|row| row.starts_with("abs_lifecycle_quadratic_abs_pos_branch|")));

    // 物化后的行与 EAGER 路径逐行一致（名称、右端项与稀疏系数全部相同）。
    // The materialized rows match the EAGER path row by row (name, right-hand side and sparse
    // coefficients all identical).
    assert_eq!(eager_rows, deferred_rows);
    assert_eq!(
        normalized_quadratic_rows(&eager),
        normalized_quadratic_rows(&deferred)
    );
    assert_eq!(eager.num_quadratic_constraints(), 1);
}

// ============================================================================
// 9. 组合次数守卫 / Composition degree guard
// ============================================================================

#[test]
fn composition_degree_guard_rejects_cubic_and_higher_terms() {
    // 桥接列与其它列配成二次项：代入桥接等式后次数 ≥ 3 / bridge paired with another column:
    // degree ≥ 3 after substituting the bridge equality.
    let bridge_columns = HashSet::from([5usize]);
    let paired = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 5, 0)], 0.0);
    let error = guard_quadratic_composition_degree(&paired, &bridge_columns, "paired").unwrap_err();
    assert!(error.to_string().contains("degree 3 or higher"), "{error}");

    // 桥接列自乘：次数 ≥ 4 / squaring the bridge column: degree ≥ 4
    let squared = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 5, 5)], 0.0);
    assert!(guard_quadratic_composition_degree(&squared, &bridge_columns, "squared").is_err());

    // 桥接列只以一次项参与组合：不变式成立 / the bridge column stays linear: invariant holds
    let linear_bridge = Quadratic::new(
        vec![
            QuadraticMonomial::new_quadratic(1.0, 0, 1),
            QuadraticMonomial::new_linear(-1.0, 5),
        ],
        0.0,
    );
    assert!(guard_quadratic_composition_degree(&linear_bridge, &bridge_columns, "linear").is_ok());

    // 组合入口的错误路径：调用方给出的列空间显示桥接列参与了二次项时，二次行生成必须报错。
    // Error path of the composition entry point: when the caller's column space says the bridge
    // column takes part in a quadratic term, quadratic row generation must fail.
    let wrapper = QuadraticAbsFunction::new(
        19,
        "abs_degree_guard",
        Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 1)], 0.0),
    );
    let mut tokens = Vec::new();
    wrapper.register_tokens(&mut tokens).unwrap();
    let colliding = HashMap::from([
        // 故意把桥接列映射到输入二次项 (0, 1) 中的列 1，以触发次数守卫。
        // The bridge column is deliberately mapped onto column 1 of the input monomial (0, 1) to
        // trigger the degree guard.
        (tokens[0].id().unique_id() as usize, 1usize),
        (tokens[1].id().unique_id() as usize, 11usize),
        (tokens[2].id().unique_id() as usize, 12usize),
    ]);
    let error = wrapper
        .quadratic_mechanism_constraints(&colliding)
        .unwrap_err();
    assert!(error.to_string().contains("degree 3 or higher"), "{error}");
}

// ============================================================================
// 10. trait 契约 / Trait contract
// ============================================================================

#[test]
fn quadratic_abs_function_satisfies_the_quadratic_contract() {
    fn assert_quadratic_function_symbol<T: QuadraticFunctionSymbol<f64>>() {}

    assert_quadratic_function_symbol::<QuadraticAbsFunction<f64>>();

    let function = QuadraticAbsFunction::new(
        18,
        "abs_contract",
        Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 1)], 2.0),
    );
    assert_eq!(
        <QuadraticAbsFunction<f64> as IntermediateSymbol<f64>>::category(&function),
        ospf_rust_core::symbol::Category::Linear
    );
    assert_eq!(
        <QuadraticAbsFunction<f64> as IntermediateSymbol<f64>>::operation_category(&function),
        ospf_rust_core::symbol::Category::Quadratic
    );

    // 结果列的一阶表达：1 * y / the first-order expression of the result column: 1 * y
    let linear = function.to_linear_polynomial();
    assert_eq!(linear.monomials().len(), 1);
    assert_eq!(linear.monomials()[0].var_index(), function.result_variable().index());
    assert!(!has_quadratic_monomials(&function.to_quadratic_polynomial()));
}
