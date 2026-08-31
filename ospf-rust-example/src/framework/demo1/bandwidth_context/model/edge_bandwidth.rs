use std::error::Error;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::{SymbolCombination, LinearExpressionSymbol, flat_map1};
use ospf_rust_core::variable::{VariableCombination, UContinuous};
use ospf_rust_multiarray::{MultiArray, MultiArrayBuilder, Shape};
use crate::framework::demo1::route_context::model::{Edge, Node, NodeKind, Service};

/// 二维连续变量组合类型别名 / 2D continuous variable combination type alias
type YCombination = VariableCombination<UContinuous, Shape<2>>;
/// 一维线性表达式符号组合类型别名 / 1D linear expression symbol combination type alias
type BandwidthSymbols = SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>;

pub struct EdgeBandwidth {
    pub y: YCombination,
    pub y_idx: MultiArray<usize, Shape<2>>,
    pub bandwidth: BandwidthSymbols,
}

impl EdgeBandwidth {
    pub fn new() -> Self {
        let y = VariableCombination::new(Shape::new([0, 0]), "y");
        let y_idx = MultiArrayBuilder::from_list(Shape::new([0, 0]), vec![]);
        let bandwidth = SymbolCombination::new(Shape::new([0]), "bandwidth", |_, _| {
            LinearExpressionSymbol::new(0, "dummy", vec![], 0.0)
        });
        Self { y, y_idx, bandwidth }
    }

    pub fn register(
        &mut self,
        model: &mut MetaModel<f64>,
        edges: &[Edge],
        services: &[Service],
        _nodes: &[Node],
    ) -> Result<(), Box<dyn Error>> {
        // 1. 注册变量组合
        let shape = Shape::new([edges.len(), services.len()]);
        let y = VariableCombination::with_name_and_range_generator(
            shape,
            "y",
            |_index, vector| format!("{}_{}", vector[0], vector[1]),
            |_index, _vector| Default::default(),
        );
        let y_idx = model.register_combination(&y)?;

        // 2. 构建符号组合：对每条边，对第二维（services）求和
        let bandwidth = {
            use ospf_rust_core::symbol::next_auto_intermediate_symbol_id;
            let shape = Shape::new([edges.len()]);
            SymbolCombination::new(shape, "bandwidth", |index, _vec| {
                let poly = y.sum_along_dimension(&[index], 1, &y_idx, 1.0_f64, 0.0_f64);
                let id = next_auto_intermediate_symbol_id();
                LinearExpressionSymbol::new(
                    id,
                    &format!("bandwidth_{}", index),
                    poly.monomials().to_vec(),
                    poly.constant_term().clone(),
                )
            })
        };
        model.add_symbol_combination(&bandwidth)?;

        self.y = y;
        self.y_idx = y_idx;
        self.bandwidth = bandwidth;
        Ok(())
    }
}
