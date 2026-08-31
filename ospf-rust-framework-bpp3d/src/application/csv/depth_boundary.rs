/// CSV depth boundary policy / CSV depth boundary policy
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvDepthBoundaryPolicy {
    /// 首层允许圆柱轴 / First layer allowed cylinder axes
    pub first_layer_allowed_cylinder_axes: Option<HashSet<Axis3>>,
    /// 末层允许圆柱轴 / Last layer allowed cylinder axes
    pub last_layer_allowed_cylinder_axes: Option<HashSet<Axis3>>,
    /// 首层允许长方体朝向 / First layer allowed cuboid orientations
    pub first_layer_allowed_cuboid_orientations: Option<HashSet<Orientation>>,
    /// 末层允许长方体朝向 / Last layer allowed cuboid orientations
    pub last_layer_allowed_cuboid_orientations: Option<HashSet<Orientation>>,
}

impl CsvDepthBoundaryPolicy {
    fn from_records(records: &[CsvDepthBoundaryPolicyRecord]) -> Result<Option<Self>, CsvDatasetError> {
        if records.is_empty() {
            return Ok(None);
        }
        let mut policy = Self {
            first_layer_allowed_cylinder_axes: None,
            last_layer_allowed_cylinder_axes: None,
            first_layer_allowed_cuboid_orientations: None,
            last_layer_allowed_cuboid_orientations: None,
        };
        for (index, record) in records.iter().enumerate() {
            let row = index + 2;
            match normalize_token(&record.field).as_str() {
                "firstlayerallowedcylinderaxes" => {
                    policy.first_layer_allowed_cylinder_axes = Some(parse_axis_set(
                        DEPTH_BOUNDARY_POLICY_TABLE,
                        row,
                        "values",
                        &record.values,
                    )?);
                }
                "lastlayerallowedcylinderaxes" => {
                    policy.last_layer_allowed_cylinder_axes = Some(parse_axis_set(
                        DEPTH_BOUNDARY_POLICY_TABLE,
                        row,
                        "values",
                        &record.values,
                    )?);
                }
                "firstlayerallowedcuboidorientations" => {
                    policy.first_layer_allowed_cuboid_orientations = Some(parse_orientation_set(
                        DEPTH_BOUNDARY_POLICY_TABLE,
                        row,
                        "values",
                        Some(&record.values),
                        true,
                    )?.into_iter().collect());
                }
                "lastlayerallowedcuboidorientations" => {
                    policy.last_layer_allowed_cuboid_orientations = Some(parse_orientation_set(
                        DEPTH_BOUNDARY_POLICY_TABLE,
                        row,
                        "values",
                        Some(&record.values),
                        true,
                    )?.into_iter().collect());
                }
                _ => {
                    return Err(invalid_value(
                        DEPTH_BOUNDARY_POLICY_TABLE,
                        row,
                        "field",
                        &record.field,
                        "unknown depth boundary policy field",
                    ));
                }
            }
        }
        Ok(Some(policy))
    }
}

