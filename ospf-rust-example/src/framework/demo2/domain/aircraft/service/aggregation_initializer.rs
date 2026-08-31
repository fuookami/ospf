//! 飞机聚合初始化器 / Aircraft aggregation initializer
use crate::framework::demo2::domain::aircraft::model::*;
use crate::framework::demo2::domain::aircraft::Aggregation;
use crate::framework::demo2::domain::shared::units;
use crate::framework::demo2::infrastructure::dto::Demo2Request;
use std::collections::HashMap;

/// 飞机聚合初始化器 / Aircraft aggregation initializer
/// 对齐 Kotlin aircraft AggregationInitializer
pub struct AircraftAggregationInitializer;

impl AircraftAggregationInitializer {
    /// 从请求数据初始化飞机聚合 / Initialize aircraft aggregation from request data
    /// 对齐 Kotlin AggregationInitializer.initialize
    pub fn initialize(request: &Demo2Request) -> Option<Aggregation> {
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
                aircraft_type,
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

        Some(Aggregation {
            reg_no: "DEFAULT".to_string(),
            aircraft_model,
            formula,
            fuselage,
            fuel_tanks: Vec::new(),
            fuel,
            decks: vec![deck],
            neighbours: HashMap::new(),
        })
    }
}
