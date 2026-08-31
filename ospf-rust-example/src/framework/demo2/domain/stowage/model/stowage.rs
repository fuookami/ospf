//! 装载模型 / Stowage model
use super::item::{Item, ItemStatus};
use super::position::Position;
use std::error::Error;
use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::LinearExpressionSymbol;
use ospf_rust_core::symbol::flatten::LinearMonomial;
use ospf_rust_core::variable::{BinaryVariableItem, UContinuousVariableItem};

/// 装载模式 / Stowage mode (对齐 Kotlin StowageMode)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StowageMode {
    /// 满载模式 / Full load mode
    FullLoad,
    /// 预分配模式 / Predistribution mode
    Predistribution,
    /// 重量推荐模式 / Weight recommendation mode
    WeightRecommendation,
}

impl StowageMode {
    /// 是否启用 MAC 优化 / Whether MAC optimization is enabled
    pub fn with_mac_optimization(&self) -> bool {
        matches!(self, StowageMode::FullLoad | StowageMode::WeightRecommendation)
    }
}

/// 装载变量索引 / Stowage variable indices
#[derive(Debug, Clone)]
pub struct StowageVariables {
    /// 物品装载决策变量索引 / Item-to-position assignment variable indices
    pub x: Vec<Vec<usize>>,
    /// 物品调整变量索引 / Item adjustment variable indices
    pub u: Vec<Vec<usize>>,
    /// 装载状态中间符号索引 / Stowage status intermediate symbol indices
    pub stowage: Vec<Vec<usize>>,
    /// 物品是否被装载索引 / Whether item is loaded to any position
    pub loaded: Vec<usize>,
}

/// 装载 / Stowage (对齐 Kotlin Stowage)
#[derive(Debug)]
pub struct Stowage {
    /// 物品列表 / Item list
    pub items: Vec<Item>,
    /// 舱位列表 / Position list
    pub positions: Vec<Position>,
}

impl Stowage {
    /// 判断物品是否需要装载到指定舱位 / Whether the item needs stowage at the specified position
    pub fn stowage_needed(item: &Item, position: &Position) -> bool {
        item.status.stowage_needed() && position.status.stowage_needed && position.enabled(item)
    }

    /// 判断物品是否需要调整到指定舱位 / Whether the item needs adjustment at the specified position
    pub fn adjustment_needed(item: &Item, position: &Position) -> bool {
        if !item.status.adjustment_needed() {
            return false;
        }
        if position.status.stowage_needed && position.enabled(item) {
            return true;
        }
        if position.status.adjustment_needed
            && (position.enabled(item) || position.loaded_items.contains(&item.id))
        {
            return true;
        }
        false
    }

    /// 注册装载变量和中间符号到模型
    /// 对齐 Kotlin Stowage.register:
    /// 1. 创建 x[i][j] 二元变量 (物品 i 是否装载到舱位 j)
    /// 2. 创建 u[i][j] 二元变量 (物品 i 是否需要调整)
    /// 3. 创建 stowage[i][j] 中间符号 (装载状态)
    /// 4. 创建 loaded[i] 中间符号 (物品是否被装载)
    pub fn register(&self, model: &mut MetaModel<f64>) -> Result<StowageVariables, Box<dyn Error>> {
        let item_count = self.items.len();
        let position_count = self.positions.len();
        let mut next_id = 20000u64;

        // 1. 注册 x[i][j] 二元变量
        let mut x_idx = vec![vec![0usize; position_count]; item_count];
        for (i, item) in self.items.iter().enumerate() {
            for (j, position) in self.positions.iter().enumerate() {
                let name = format!("x_{}_{}", item.id, position.id);
                if Self::stowage_needed(item, position) {
                    let var = BinaryVariableItem::auto(&name);
                    x_idx[i][j] = model.register_variable(var)?;
                } else if position.loaded_items.contains(&item.id) {
                    // 已装载的物品，变量固定为 1
                    let var = BinaryVariableItem::auto(&name);
                    x_idx[i][j] = model.register_variable(var)?;
                } else {
                    // 不需要装载的物品，变量固定为 0
                    let var = BinaryVariableItem::auto(&name);
                    x_idx[i][j] = model.register_variable(var)?;
                }
            }
        }

        // 2. 注册 u[i][j] 二元变量
        let mut u_idx = vec![vec![0usize; position_count]; item_count];
        for (i, item) in self.items.iter().enumerate() {
            for (j, position) in self.positions.iter().enumerate() {
                let name = format!("u_{}_{}", item.id, position.id);
                if Self::adjustment_needed(item, position) {
                    let var = BinaryVariableItem::auto(&name);
                    u_idx[i][j] = model.register_variable(var)?;
                } else {
                    let var = BinaryVariableItem::auto(&name);
                    u_idx[i][j] = model.register_variable(var)?;
                }
            }
        }

        // 3. 创建 stowage[i][j] 中间符号
        // stowage[i][j] = x[i][j] + u[i][j] (如果需要调整) + 已装载常量
        let mut stowage_idx = vec![vec![0usize; position_count]; item_count];
        for (i, item) in self.items.iter().enumerate() {
            for (j, position) in self.positions.iter().enumerate() {
                let mut monomials = Vec::new();
                if Self::stowage_needed(item, position) {
                    monomials.push(LinearMonomial::new(1.0, x_idx[i][j]));
                }
                if Self::adjustment_needed(item, position) {
                    monomials.push(LinearMonomial::new(1.0, u_idx[i][j]));
                }
                let constant = if position.loaded_items.contains(&item.id) {
                    1.0
                } else {
                    0.0
                };
                let symbol = LinearExpressionSymbol::new(
                    next_id,
                    &format!("stowage_{}_{}", item.id, position.id),
                    monomials,
                    constant,
                );
                model.add_symbol(Arc::new(symbol))?;
                stowage_idx[i][j] = next_id as usize;
                next_id += 1;
            }
        }

        // 4. 创建 loaded[i] 中间符号
        // loaded[i] = sum(stowage[i][j] for all j)
        let mut loaded_idx = vec![0usize; item_count];
        for (i, item) in self.items.iter().enumerate() {
            let monomials: Vec<LinearMonomial<f64>> = (0..position_count)
                .map(|j| LinearMonomial::new(1.0, stowage_idx[i][j]))
                .collect();
            let symbol = LinearExpressionSymbol::new(
                next_id,
                &format!("loaded_{}", item.id),
                monomials,
                0.0,
            );
            model.add_symbol(Arc::new(symbol))?;
            loaded_idx[i] = next_id as usize;
            next_id += 1;
        }

        Ok(StowageVariables {
            x: x_idx,
            u: u_idx,
            stowage: stowage_idx,
            loaded: loaded_idx,
        })
    }
}
