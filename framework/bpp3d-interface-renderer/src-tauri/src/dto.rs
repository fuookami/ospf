use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};

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

fn deserialize_f64_from_number_or_string<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NumberOrString {
        Number(f64),
        String(String),
    }

    match NumberOrString::deserialize(deserializer)? {
        NumberOrString::Number(value) => Ok(value),
        NumberOrString::String(value) => value.parse().map_err(serde::de::Error::custom),
    }
}

fn deserialize_optional_f64_from_number_or_string<'de, D>(
    deserializer: D,
) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OptionalNumberOrString {
        Number(f64),
        String(String),
        None,
    }

    match OptionalNumberOrString::deserialize(deserializer)? {
        OptionalNumberOrString::Number(value) => Ok(Some(value)),
        OptionalNumberOrString::String(value) => value.parse().map(Some).map_err(serde::de::Error::custom),
        OptionalNumberOrString::None => Ok(None),
    }
}

fn deserialize_usize_from_number_or_string<'de, D>(deserializer: D) -> Result<usize, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum IntegerOrString {
        Integer(usize),
        String(String),
    }

    match IntegerOrString::deserialize(deserializer)? {
        IntegerOrString::Integer(value) => Ok(value),
        IntegerOrString::String(value) => value.parse().map_err(serde::de::Error::custom),
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LoadingPlanItemDTO {
    name: String,
    #[serde(default)]
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
    #[serde(deserialize_with = "deserialize_usize_from_number_or_string")]
    loading_order: usize,
    #[serde(default)]
    shape_type: Option<RenderShapeTypeDTO>,
    #[serde(default)]
    render_shape_type: Option<RenderShapeTypeDTO>,
    #[serde(default)]
    algorithm_shape_type: Option<RenderAlgorithmShapeTypeDTO>,
    #[serde(default, deserialize_with = "deserialize_optional_f64_from_number_or_string")]
    radius: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64_from_number_or_string")]
    diameter: Option<f64>,
    #[serde(default)]
    axis: Option<RenderAxis3DTO>,
    #[serde(default, deserialize_with = "deserialize_optional_f64_from_number_or_string")]
    bounding_width: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64_from_number_or_string")]
    bounding_height: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64_from_number_or_string")]
    bounding_depth: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64_from_number_or_string")]
    actual_volume: Option<f64>,
    #[serde(default)]
    info: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LoadingPlanDTO {
    #[serde(default)]
    group: Vec<String>,
    name: String,
    type_code: String,
    #[serde(deserialize_with = "deserialize_f64_from_number_or_string")]
    width: f64,
    #[serde(deserialize_with = "deserialize_f64_from_number_or_string")]
    height: f64,
    #[serde(deserialize_with = "deserialize_f64_from_number_or_string")]
    depth: f64,
    #[serde(deserialize_with = "deserialize_f64_from_number_or_string")]
    loading_rate: f64,
    #[serde(deserialize_with = "deserialize_f64_from_number_or_string")]
    weight: f64,
    #[serde(deserialize_with = "deserialize_f64_from_number_or_string")]
    volume: f64,
    #[serde(default)]
    items: Vec<LoadingPlanItemDTO>,
    #[serde(default)]
    info: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SchemaDTO {
    #[serde(default)]
    kpi: BTreeMap<String, String>,
    #[serde(default)]
    loading_plans: Vec<LoadingPlanDTO>,
}

#[cfg(test)]
mod tests {
    use super::*;

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

        assert!(matches!(item.shape_type, Some(RenderShapeTypeDTO::Cylinder)));
        assert!(matches!(
            item.algorithm_shape_type,
            Some(RenderAlgorithmShapeTypeDTO::VerticalCylinder)
        ));
        assert_eq!(item.loading_order, 1);
        assert_eq!(item.radius, Some(100.0));
    }
}
