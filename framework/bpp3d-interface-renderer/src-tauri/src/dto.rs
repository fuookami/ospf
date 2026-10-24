use std::collections::BTreeMap;

use ospf_renderer_contract::{
    group, integer as deserialize_u64_from_number_or_string,
    number as deserialize_f64_from_number_or_string,
    optional_number as deserialize_optional_f64_from_number_or_string, string_map,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum RenderShapeTypeDTO {
    #[serde(alias = "CUBOID", alias = "cuboid")]
    Cuboid,
    #[serde(alias = "CYLINDER", alias = "cylinder")]
    Cylinder,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum RenderAxis3DTO {
    X,
    Y,
    Z,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum RenderAlgorithmShapeTypeDTO {
    #[serde(alias = "CUBOID", alias = "cuboid")]
    Cuboid,
    #[serde(alias = "VERTICAL_CYLINDER", alias = "vertical_cylinder")]
    VerticalCylinder,
    #[serde(alias = "HORIZONTAL_CYLINDER_X", alias = "horizontal_cylinder_x")]
    HorizontalCylinderX,
    #[serde(alias = "HORIZONTAL_CYLINDER_Z", alias = "horizontal_cylinder_z")]
    HorizontalCylinderZ,
    #[serde(alias = "BOUNDING_CUBOID", alias = "bounding_cuboid")]
    BoundingCuboid,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LoadingPlanItemDTO {
    name: String,
    #[serde(default)]
    #[serde(alias = "package_type")]
    package_type: Option<String>,
    #[serde(deserialize_with = "deserialize_f64_from_number_or_string")]
    width: f64,
    #[serde(deserialize_with = "deserialize_f64_from_number_or_string")]
    height: f64,
    #[serde(deserialize_with = "deserialize_f64_from_number_or_string")]
    depth: f64,
    #[serde(deserialize_with = "deserialize_f64_from_number_or_string")]
    x: f64,
    #[serde(deserialize_with = "deserialize_f64_from_number_or_string")]
    y: f64,
    #[serde(deserialize_with = "deserialize_f64_from_number_or_string")]
    z: f64,
    #[serde(deserialize_with = "deserialize_f64_from_number_or_string")]
    weight: f64,
    #[serde(deserialize_with = "deserialize_u64_from_number_or_string")]
    #[serde(alias = "loading_order")]
    loading_order: u64,
    #[serde(default)]
    #[serde(alias = "shape_type")]
    shape_type: Option<RenderShapeTypeDTO>,
    #[serde(default)]
    #[serde(alias = "render_shape_type")]
    render_shape_type: Option<RenderShapeTypeDTO>,
    #[serde(default)]
    #[serde(alias = "algorithm_shape_type")]
    algorithm_shape_type: Option<RenderAlgorithmShapeTypeDTO>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_f64_from_number_or_string"
    )]
    radius: Option<f64>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_f64_from_number_or_string"
    )]
    diameter: Option<f64>,
    #[serde(default)]
    axis: Option<RenderAxis3DTO>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_f64_from_number_or_string"
    )]
    #[serde(alias = "bounding_width")]
    bounding_width: Option<f64>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_f64_from_number_or_string"
    )]
    #[serde(alias = "bounding_height")]
    bounding_height: Option<f64>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_f64_from_number_or_string"
    )]
    #[serde(alias = "bounding_depth")]
    bounding_depth: Option<f64>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_f64_from_number_or_string"
    )]
    #[serde(alias = "actual_volume")]
    actual_volume: Option<f64>,
    #[serde(default, deserialize_with = "string_map")]
    info: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LoadingPlanDTO {
    #[serde(default, deserialize_with = "group")]
    group: Vec<String>,
    name: String,
    #[serde(alias = "type_code")]
    type_code: String,
    #[serde(deserialize_with = "deserialize_f64_from_number_or_string")]
    width: f64,
    #[serde(deserialize_with = "deserialize_f64_from_number_or_string")]
    height: f64,
    #[serde(deserialize_with = "deserialize_f64_from_number_or_string")]
    depth: f64,
    #[serde(deserialize_with = "deserialize_f64_from_number_or_string")]
    #[serde(alias = "loading_rate")]
    loading_rate: f64,
    #[serde(deserialize_with = "deserialize_f64_from_number_or_string")]
    weight: f64,
    #[serde(deserialize_with = "deserialize_f64_from_number_or_string")]
    volume: f64,
    #[serde(default)]
    items: Vec<LoadingPlanItemDTO>,
    #[serde(default, deserialize_with = "string_map")]
    info: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SchemaDTO {
    #[serde(default, deserialize_with = "string_map")]
    kpi: BTreeMap<String, String>,
    #[serde(default)]
    #[serde(alias = "loading_plans")]
    loading_plans: Vec<LoadingPlanDTO>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_snake_case_groups_and_nullable_info() {
        let schema: SchemaDTO = serde_json::from_value(serde_json::json!({
            "kpi": {"rate": 0.5, "complete": true}, "loading_plans": [{
                "group": "shipment", "name": "bin1", "type_code": "BIN",
                "width": 10, "height": 10, "depth": 10, "loading_rate": 0.5,
                "weight": 1, "volume": 1000, "info": null, "items": [{
                    "name": "cylinder", "package_type": "roll", "width": 2, "height": 3, "depth": 2,
                    "x": 0, "y": 0, "z": 0, "weight": 1, "loading_order": 1,
                    "shape_type": "Cylinder", "algorithm_shape_type": "VerticalCylinder",
                    "radius": 1, "axis": "Y", "bounding_width": 2, "bounding_height": 3,
                    "bounding_depth": 2, "actual_volume": 9.42, "info": "fragile"
                }]
            }]
        }))
        .unwrap();
        let output = serde_json::to_value(schema).unwrap();
        let plan = &output["loadingPlans"][0];
        assert_eq!(plan["group"], serde_json::json!(["shipment"]));
        assert_eq!(plan["items"][0]["algorithmShapeType"], "VerticalCylinder");
        assert_eq!(plan["items"][0]["info"]["info"], "fragile");
        assert_eq!(plan["info"], serde_json::json!({}));
        assert_eq!(output["kpi"]["complete"], "true");
        serde_json::from_value::<SchemaDTO>(output).unwrap();
    }

    #[test]
    fn reject_nonfinite_dimensions() {
        let mut input: serde_json::Value = serde_json::from_str(include_str!(
            "../../src/examples/cuboid-cylinder-loading-plan.json"
        ))
        .unwrap();
        input["loadingPlans"][0]["width"] = serde_json::json!("NaN");
        assert!(serde_json::from_value::<SchemaDTO>(input).is_err());
    }

    #[test]
    fn deserialize_horizontal_cylinder_algorithm_shape_types() {
        let json = r#"
        {
            "loadingPlans": [
                {
                    "group": ["renderer-fixture"],
                    "name": "horizontal-cylinder-axis",
                    "typeCode": "BIN-AXIS",
                    "width": 1600,
                    "height": 700,
                    "depth": 1200,
                    "loadingRate": 0.0878,
                    "weight": 50,
                    "volume": 61072622.98,
                    "items": [
                        {
                            "name": "Cylinder-X-Floor",
                            "width": 700,
                            "height": 240,
                            "depth": 240,
                            "x": 240,
                            "y": 0,
                            "z": 0,
                            "weight": 25,
                            "loadingOrder": 1,
                            "shapeType": "Cylinder",
                            "renderShapeType": "Cylinder",
                            "algorithmShapeType": "HorizontalCylinderX",
                            "radius": 120,
                            "diameter": 240,
                            "axis": "X",
                            "boundingWidth": 700,
                            "boundingHeight": 240,
                            "boundingDepth": 240,
                            "actualVolume": 31667253.89
                        },
                        {
                            "name": "Cylinder-Z-Floor",
                            "width": 240,
                            "height": 240,
                            "depth": 650,
                            "x": 0,
                            "y": 0,
                            "z": 360,
                            "weight": 25,
                            "loadingOrder": 2,
                            "shapeType": "Cylinder",
                            "renderShapeType": "Cylinder",
                            "algorithmShapeType": "HorizontalCylinderZ",
                            "radius": 120,
                            "diameter": 240,
                            "axis": "Z",
                            "boundingWidth": 240,
                            "boundingHeight": 240,
                            "boundingDepth": 650,
                            "actualVolume": 29405369.09
                        }
                    ]
                }
            ]
        }
        "#;

        let schema: SchemaDTO =
            serde_json::from_str(json).expect("horizontal cylinder axis sample should deserialize");
        let items = &schema.loading_plans[0].items;

        assert!(items.iter().any(|item| {
            matches!(
                item.algorithm_shape_type,
                Some(RenderAlgorithmShapeTypeDTO::HorizontalCylinderX)
            )
        }));
        assert!(items.iter().any(|item| {
            matches!(
                item.algorithm_shape_type,
                Some(RenderAlgorithmShapeTypeDTO::HorizontalCylinderZ)
            )
        }));
    }

    #[test]
    fn deserialize_number_strings_from_bpp3d_fixture() {
        let json = include_str!("../../src/examples/cuboid-cylinder-loading-plan.json");

        let schema: SchemaDTO =
            serde_json::from_str(json).expect("renderer fixture should deserialize");
        let items = &schema.loading_plans[0].items;

        assert!(items.iter().any(|item| {
            matches!(item.shape_type, Some(RenderShapeTypeDTO::Cylinder))
                && matches!(
                    item.algorithm_shape_type,
                    Some(RenderAlgorithmShapeTypeDTO::HorizontalCylinderX)
                )
                && item.radius == Some(100.0)
                && item.actual_volume == Some(15707963.27)
        }));
    }

    #[test]
    fn deserialize_legacy_alias_shape_names() {
        let json = r#"
        {
            "loadingPlans": [
                {
                    "group": ["renderer-fixture"],
                    "name": "alias-shape-names",
                    "typeCode": "BIN-ALIAS",
                    "width": "1000",
                    "height": "800",
                    "depth": "900",
                    "loadingRate": "0.01",
                    "weight": "10",
                    "volume": "100",
                    "items": [
                        {
                            "name": "Cylinder-Alias",
                            "width": "200",
                            "height": "500",
                            "depth": "200",
                            "x": "0",
                            "y": "0",
                            "z": "0",
                            "weight": "10",
                            "loadingOrder": "1",
                            "shapeType": "CYLINDER",
                            "renderShapeType": "CYLINDER",
                            "algorithmShapeType": "VERTICAL_CYLINDER",
                            "radius": "100",
                            "diameter": "200",
                            "axis": "Y",
                            "boundingWidth": "200",
                            "boundingHeight": "500",
                            "boundingDepth": "200",
                            "actualVolume": "15707963.27"
                        }
                    ]
                }
            ]
        }
        "#;

        let schema: SchemaDTO =
            serde_json::from_str(json).expect("legacy alias shape names should deserialize");
        let item = &schema.loading_plans[0].items[0];

        assert!(matches!(
            item.shape_type,
            Some(RenderShapeTypeDTO::Cylinder)
        ));
        assert!(matches!(
            item.algorithm_shape_type,
            Some(RenderAlgorithmShapeTypeDTO::VerticalCylinder)
        ));
        assert_eq!(item.loading_order, 1);
        assert_eq!(item.radius, Some(100.0));
    }
}
