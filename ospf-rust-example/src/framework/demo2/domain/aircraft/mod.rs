//! 飞机领域模块 / Aircraft domain module.
pub mod model;
pub mod service;

use model::*;
use std::collections::HashMap;
use super::shared::units;

/// 飞机领域聚合 / Aircraft domain aggregation
#[derive(Debug)]
pub struct Aggregation {
    /// 注册号 / Registration number
    pub reg_no: String,
    /// 飞机型号 / Aircraft model
    pub aircraft_model: AircraftModel,
    /// 气动公式 / Aerodynamic formula
    pub formula: Formula,
    /// 机身数据 / Fuselage data
    pub fuselage: Fuselage,
    /// 燃油箱列表 / Fuel tank list
    pub fuel_tanks: Vec<FuelTank>,
    /// 各飞行阶段燃油常数 / Fuel constants per flight phase
    pub fuel: HashMap<FlightPhase, FuelConstant>,
    /// 甲板列表 / Deck list
    pub decks: Vec<Deck>,
    /// 各类型的邻接关系 / Neighbours by type
    pub neighbours: HashMap<NeighbourType, Vec<Neighbour>>,
}

impl Aggregation {
    /// 获取所有舱位 / Get all positions across decks
    pub fn positions(&self) -> Vec<&Position> {
        self.decks.iter().flat_map(|d| d.positions.iter()).collect()
    }

    /// 获取冲突舱位对 / Get conflicting position pairs
    pub fn conflict_positions(&self) -> Vec<PositionPair> {
        let positions = self.positions();
        let mut conflicts = Vec::new();
        for i in 0..positions.len() {
            for j in (i + 1)..positions.len() {
                conflicts.push((positions[i].clone(), positions[j].clone()));
            }
        }
        conflicts
    }
}

/// 飞机上下文 / Aircraft context
#[derive(Debug)]
pub struct AircraftContext {
    /// 飞机聚合 / Aircraft aggregation
    pub aggregation: Option<Aggregation>,
}

impl AircraftContext {
    /// 创建空的飞机上下文 / Create an empty aircraft context
    pub fn new() -> Self {
        Self { aggregation: None }
    }

    /// 从请求数据初始化飞机聚合 / Initialize aircraft aggregation from request data
    pub fn init(
        &mut self,
        request: &crate::framework::demo2::infrastructure::dto::Demo2Request,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let aircraft_type = match request.aircraft_type {
            crate::framework::demo2::infrastructure::dto::AircraftTypeInput::B737 => AircraftType::B737,
            crate::framework::demo2::infrastructure::dto::AircraftTypeInput::B757 => AircraftType::B757,
            crate::framework::demo2::infrastructure::dto::AircraftTypeInput::B767 => AircraftType::B767,
            crate::framework::demo2::infrastructure::dto::AircraftTypeInput::B747 => AircraftType::B747,
            _ => AircraftType::B737,
        };

        let wide_body = matches!(
            request.aircraft_type,
            crate::framework::demo2::infrastructure::dto::AircraftTypeInput::B767
                | crate::framework::demo2::infrastructure::dto::AircraftTypeInput::B747
        );

        let aircraft_model = AircraftModel {
            name: format!("{:?}", request.aircraft_type),
            aircraft_type: aircraft_type.clone(),
            minor_model: AircraftMinorModel {
                name: format!("{:?}-default", request.aircraft_type),
                aircraft_type: aircraft_type.clone(),
            },
            wide_body,
        };

        let formula = Formula {
            lip: units::length(0.0),
            chord: units::length(1.0),
            standard_datum: units::length(0.0),
            force_distance_coefficient: 1.0,
            doi_correction: 0.0,
        };

        let fuselage = Fuselage {
            liferaft: None,
            dow: units::weight(0.0),
            doi: 0.0,
            balanced_arm: units::length(0.0),
        };

        let positions: Vec<Position> = request
            .positions
            .iter()
            .map(|p| Position {
                id: p.name.clone(),
                space_name: p.name.clone(),
                alpha_space_name: p.name.clone(),
                size_code: "LD3".to_string(),
                loading_order: 0,
                coordinate: PositionCoordinate {
                    front_arm: units::length(p.longitudinal_arm),
                    back_arm: units::length(p.longitudinal_arm),
                    left_arm: units::length(p.lateral_arm),
                    right_arm: units::length(p.lateral_arm),
                    offsets: HashMap::new(),
                },
                shape: PositionShape {
                    width: units::length(1.0),
                    length: units::length(p.length),
                    height: units::length(1.0),
                },
                location: PositionLocation {
                    tags: vec![PositionLocationTag::Main],
                },
            })
            .collect();

        let deck = Deck {
            name: "Main".to_string(),
            location: DeckLocation::Main,
            positions,
        };

        let mut fuel = HashMap::new();
        fuel.insert(FlightPhase::ZeroFuel, FuelConstant { weight: units::weight(0.0), arm: units::length(0.0) });
        fuel.insert(FlightPhase::TakeOff, FuelConstant { weight: units::weight(0.0), arm: units::length(0.0) });
        fuel.insert(FlightPhase::Landing, FuelConstant { weight: units::weight(0.0), arm: units::length(0.0) });

        self.aggregation = Some(Aggregation {
            reg_no: "DEFAULT".to_string(),
            aircraft_model,
            formula,
            fuselage,
            fuel_tanks: Vec::new(),
            fuel,
            decks: vec![deck],
            neighbours: HashMap::new(),
        });

        Ok(())
    }
}
