use std::collections::BTreeMap;

use ospf_renderer_contract::{group, integer, number, optional_number, string_map};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CuttingPlanProductionDTO {
    pub name: String,
    #[serde(deserialize_with = "number")]
    pub x: f64,
    #[serde(deserialize_with = "number")]
    pub width: f64,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default, alias = "unit_length", deserialize_with = "optional_number")]
    pub unit_length: Option<f64>,
    #[serde(alias = "production_type")]
    pub production_type: String,
    #[serde(default, deserialize_with = "optional_integer")]
    pub amount: Option<u64>,
    #[serde(default, deserialize_with = "string_map")]
    pub info: BTreeMap<String, String>,
    #[serde(default)]
    pub color: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CuttingPlanDTO {
    #[serde(default, deserialize_with = "group")]
    pub group: Vec<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default, alias = "material_id")]
    pub material_id: Option<String>,
    pub productions: Vec<CuttingPlanProductionDTO>,
    #[serde(deserialize_with = "number")]
    pub width: f64,
    #[serde(alias = "standard_width", deserialize_with = "number")]
    pub standard_width: f64,
    #[serde(default, alias = "rest_width", deserialize_with = "optional_number")]
    pub rest_width: Option<f64>,
    #[serde(deserialize_with = "integer")]
    pub amount: u64,
    #[serde(default, deserialize_with = "string_map")]
    pub info: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SchemaDTO {
    #[serde(default, deserialize_with = "string_map")]
    pub kpi: BTreeMap<String, String>,
    #[serde(alias = "cutting_plans")]
    pub cutting_plans: Vec<CuttingPlanDTO>,
}

fn optional_integer<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<u64>, D::Error> {
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    value
        .map(|value| integer(value).map_err(serde::de::Error::custom))
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn kotlin_and_rust_render_schemas() {
        for numeric_strings in [false, true] {
            let number = |value: u64| {
                if numeric_strings {
                    json!(value.to_string())
                } else {
                    json!(value)
                }
            };
            let input = json!({"kpi": {"cost": "3"}, "cuttingPlans": [{
                "group": ["steel"], "id": "plan", "materialId": "coil", "amount": number(2),
                "width": number(10), "standardWidth": number(12), "restWidth": "2",
                "info": {}, "productions": [{"id": "a", "name": "A", "x": number(0),
                    "width": number(5), "unitLength": "100", "amount": 2,
                    "productionType": "Product", "info": {}}]
            }]});
            let schema: SchemaDTO = serde_json::from_value(input).unwrap();
            assert_eq!(
                schema.cutting_plans[0].productions[0].unit_length,
                Some(100.0)
            );
            assert_eq!(schema.cutting_plans[0].productions[0].x, 0.0);
            let output = serde_json::to_value(schema).unwrap();
            assert_eq!(output["cuttingPlans"][0]["width"], 10.0);
            assert_eq!(output["cuttingPlans"][0]["materialId"], "coil");
        }
    }

    #[test]
    fn kotlin_optional_fields_and_legacy_colors() {
        let schema: SchemaDTO = serde_json::from_value(json!({"cuttingPlans": [{
            "group": [], "width": "10", "standardWidth": "12", "amount": "1",
            "productions": [{"name": "A", "x": "0", "width": "5", "unitLength": null,
                "productionType": "Costar", "info": {}, "color": "#123456"}]
        }]}))
        .unwrap();
        assert_eq!(
            schema.cutting_plans[0].productions[0].color.as_deref(),
            Some("#123456")
        );
        assert_eq!(schema.cutting_plans[0].productions[0].unit_length, None);
    }

    #[test]
    fn reject_invalid_dimensions_and_amounts() {
        for (width, amount) in [
            (json!("NaN"), json!(1)),
            (json!("oops"), json!(1)),
            (json!(10), json!(-1)),
            (json!(10), json!(1.5)),
        ] {
            assert!(
                serde_json::from_value::<SchemaDTO>(json!({"cuttingPlans": [{
                    "group": [], "width": width, "standardWidth": 10,
                    "amount": amount, "productions": []
                }]}))
                .is_err()
            );
        }
    }
}
