use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::{SymbolCombination, LinearExpressionSymbol};
use ospf_rust_core::variable::{VariableCombination, Binary};
use ospf_rust_multiarray::{MultiArray, Shape};

/// 二维二值变量组合类型别名 / 2D binary variable combination type alias
type XCombination = VariableCombination<Binary, Shape<2>>;
/// 一维线性表达式符号组合类型别名 / 1D linear expression symbol combination type alias
type AssignmentSymbols = SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>;

/// 分配 / Assignment (对齐 Kotlin Assignment)
pub struct Assignment {
    pub normal_node_indices: Vec<usize>,
    pub x: XCombination,
    pub x_idx: MultiArray<usize, Shape<2>>,
    /// 每个 normal node 的分配符号：sum(x[node, *]) across services
    pub node_assignment: AssignmentSymbols,
    /// 每个 service 的分配符号：sum(x[*, service]) across nodes
    pub service_assignment: AssignmentSymbols,
}

impl Assignment {
    pub fn new(normal_node_indices: Vec<usize>) -> Self {
        let x = VariableCombination::new(Shape::new([0, 0]), "x");
        let x_idx = MultiArray::from_list(Shape::new([0, 0]), vec![]);
        let dummy = SymbolCombination::new(Shape::new([0]), "dummy", |_, _| {
            LinearExpressionSymbol::new(0, "dummy", vec![], 0.0)
        });
        Self {
            normal_node_indices,
            x,
            x_idx,
            node_assignment: dummy.clone(),
            service_assignment: dummy,
        }
    }

    pub fn register(
        &mut self,
        model: &mut MetaModel<f64>,
        service_count: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let normal_count = self.normal_node_indices.len();
        let normal_indices = &self.normal_node_indices;

        // 1. 注册变量组合
        let shape = Shape::new([normal_count, service_count]);
        let x = VariableCombination::with_name_and_range_generator(
            shape,
            "x",
            |_index, vector| format!("{}_{}", normal_indices[vector[0]], vector[1]),
            |_index, _vector| Default::default(),
        );
        let x_idx = model.register_combination(&x)?;

        // 2. node_assignment: 对每个 node，sum across services (dim 1)
        use ospf_rust_core::symbol::intermediate_symbol::next_auto_intermediate_symbol_id;
        let node_assignment = SymbolCombination::new(
            Shape::new([normal_count]),
            "node_assignment",
            |index, _vec| {
                let poly = x.sum_along_dimension(&[index], 1, &x_idx, 1.0_f64, 0.0_f64);
                let id = next_auto_intermediate_symbol_id();
                LinearExpressionSymbol::new(
                    id,
                    &format!("node_assignment_{}", normal_indices[index]),
                    poly.monomials().to_vec(),
                    poly.constant_term().clone(),
                )
            },
        );
        model.add_symbol_combination(&node_assignment)?;

        // 3. service_assignment: 对每个 service，sum across nodes (dim 0)
        let service_assignment = SymbolCombination::new(
            Shape::new([service_count]),
            "service_assignment",
            |index, _vec| {
                let poly = x.sum_along_dimension(&[index], 0, &x_idx, 1.0_f64, 0.0_f64);
                let id = next_auto_intermediate_symbol_id();
                LinearExpressionSymbol::new(
                    id,
                    &format!("service_assignment_{}", index),
                    poly.monomials().to_vec(),
                    poly.constant_term().clone(),
                )
            },
        );
        model.add_symbol_combination(&service_assignment)?;

        self.x = x;
        self.x_idx = x_idx;
        self.node_assignment = node_assignment;
        self.service_assignment = service_assignment;
        Ok(())
    }
}
