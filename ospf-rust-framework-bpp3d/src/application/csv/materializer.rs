/// CSV application 物化器 / CSV application materializer
#[derive(Debug, Clone, Default)]
pub struct CsvApplicationMaterializer;

impl CsvApplicationMaterializer {
    /// 物化 CSV 数据集 / Materialize CSV dataset
    pub fn materialize(dataset: &CsvDataset) -> Result<CsvMaterializedApplicationRequest, CsvDatasetError> {
        let draft = dataset.to_request_draft()?;
        let mut diagnostics = draft.diagnostics.clone();
        let items = dataset.items
            .iter()
            .enumerate()
            .map(|(index, record)| materialize_item(record, index + 2, &mut diagnostics))
            .collect::<Result<Vec<_>, _>>()?;
        let item_amounts = dataset.items
            .iter()
            .map(|record| (record.item_id.clone(), record.amount))
            .collect();
        let bins = dataset.bins
            .iter()
            .map(materialize_bin)
            .collect::<Vec<_>>();
        let bin_by_id = dataset.bins
            .iter()
            .cloned()
            .zip(bins.iter().cloned())
            .map(|(record, bin)| (record.bin_id, bin))
            .collect::<HashMap<_, _>>();
        let initial_layers = dataset.layers
            .iter()
            .enumerate()
            .map(|(index, record)| materialize_layer(record, index + 2, &bin_by_id))
            .collect::<Result<Vec<_>, _>>()?;
        let patterned_items = dataset.items
            .iter()
            .filter_map(|record| {
                record.pattern_code.as_ref().and_then(|pattern_code| {
                    let pattern_code = pattern_code.trim();
                    (!pattern_code.is_empty()).then(|| {
                        (
                            record.item_id.clone(),
                            PatternedItemKey {
                                pattern_code: pattern_code.to_string(),
                            },
                        )
                    })
                })
            })
            .collect::<Vec<_>>();
        let package_attributes = dataset.items
            .iter()
            .filter_map(|record| materialize_package_attribute(record))
            .collect::<Vec<_>>();
        let continuous_radius_component = materialize_continuous_radius_component(&dataset.items)?;
        let continuous_radius_component = match continuous_radius_component {
            Some(mut component) => {
                component.weight_functions =
                    materialize_radius_weight_functions(&dataset.radius_weight_functions)?;
                Some(component)
            }
            None => None,
        };

        Ok(CsvMaterializedApplicationRequest {
            items,
            item_amounts,
            bins,
            initial_layers,
            depth_boundary_policy: draft.depth_boundary_policy,
            patterned_items,
            package_attributes,
            continuous_radius_component,
            diagnostics,
        })
    }
}

