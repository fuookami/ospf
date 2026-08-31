//! Demo2 应用层 / Demo2 application layer
use std::error::Error;
use super::domain::{

    FullLoadApplication, LoadingOrderApplication, PredistributionApplication,
    WeightRecommendationApplication,
};
use super::infrastructure::dto::{AircraftTypeInput, Demo2Request, SolvePolicy};

/// 运行 Demo2 应用示例 / Run the Demo2 application example
pub fn run() -> Result<(), Box<dyn Error>> {
    let request = Demo2Request::sample();
    let full_load = FullLoadApplication;
    let full_load_response = full_load.execute(request.clone())?;
    let predistribution = PredistributionApplication;
    let predistribution_response = predistribution.execute(request)?;
    let weight_recommendation = WeightRecommendationApplication;
    let weight_recommendation_response = weight_recommendation.execute(Demo2Request::sample())?;
    let loading_order = LoadingOrderApplication;
    let loading_order_response = loading_order.execute(Demo2Request::sample())?;
    let mut unsupported_request = Demo2Request::sample();
    unsupported_request.aircraft_type = AircraftTypeInput::B767;
    let unsupported_response = full_load.execute(unsupported_request)?;
    let mut benders_no_fallback_request = Demo2Request::sample();
    benders_no_fallback_request.solve_policy = SolvePolicy {
        prefer_benders: true,
        benders_fallback_to_milp: false,
    };
    let benders_no_fallback_response = full_load.execute(benders_no_fallback_request)?;

    println!("=== Framework Demo2 (FullLoad Minimal) ===");
    println!("status: {}", full_load_response.status);
    if let Some(obj) = full_load_response.objective {
        println!("objective: {:.2}", obj);
    }
    for note in full_load_response.notes {
        println!("note: {}", note);
    }
    for line in full_load_response.assignments {
        println!("{}", line);
    }

    println!("=== Framework Demo2 (Predistribution Minimal) ===");
    println!("status: {}", predistribution_response.status);
    if let Some(obj) = predistribution_response.objective {
        println!("objective: {:.2}", obj);
    }
    for note in predistribution_response.notes {
        println!("note: {}", note);
    }
    for line in predistribution_response.assignments {
        println!("{}", line);
    }

    println!("=== Framework Demo2 (WeightRecommendation Minimal) ===");
    println!("status: {}", weight_recommendation_response.status);
    if let Some(obj) = weight_recommendation_response.objective {
        println!("objective: {:.2}", obj);
    }
    for note in weight_recommendation_response.notes {
        println!("note: {}", note);
    }
    for line in weight_recommendation_response.assignments {
        println!("{}", line);
    }

    println!("=== Framework Demo2 (LoadingOrder Minimal) ===");
    println!("status: {}", loading_order_response.status);
    for note in loading_order_response.notes {
        println!("note: {}", note);
    }
    for line in loading_order_response.orders {
        println!("{}", line);
    }

    println!("=== Framework Demo2 (Branch Handling Samples) ===");
    println!(
        "full-load unsupported-aircraft status: {}",
        unsupported_response.status
    );
    for note in unsupported_response.notes {
        println!("note: {}", note);
    }
    println!(
        "full-load benders-no-fallback status: {}",
        benders_no_fallback_response.status
    );
    for note in benders_no_fallback_response.notes {
        println!("note: {}", note);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framework_demo2_full_load_minimal() {
        assert!(run().is_ok());
    }
}
