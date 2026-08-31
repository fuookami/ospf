// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variable_array1_new_and_len() {
        let array: VariableArray1<String, ContinuousVariableItem> = VariableArray1::new("x");
        assert_eq!(array.prefix, "x");
        assert!(array.is_empty());
    }

    #[test]
    fn variable_array2_new_and_len() {
        let array: VariableArray2<String, String, BinaryVariableItem> = VariableArray2::new("u");
        assert_eq!(array.prefix, "u");
        assert!(array.is_empty());
    }

    #[test]
    fn expression_array1_insert_and_get() {
        let mut exprs: ExpressionArray1<String> = ExpressionArray1::new("load");
        exprs.insert("demand_1".to_string(), vec![(0, 1.0), (1, 2.0)]);
        assert_eq!(exprs.len(), 1);

        let terms = exprs.get(&"demand_1".to_string()).unwrap();
        assert_eq!(terms.len(), 2);
        assert_eq!(terms[0], (0, 1.0));
    }

    #[test]
    fn solution_extractor_extract_value() {
        let solution = vec![0.0, 1.5, 0.8, 1.0];
        assert_eq!(SolutionExtractor::extract_value(&solution, 0), Some(0.0));
        assert_eq!(SolutionExtractor::extract_value(&solution, 1), Some(1.5));
        assert_eq!(SolutionExtractor::extract_value(&solution, 10), None);
    }

    #[test]
    fn solution_extractor_extract_binary() {
        let solution = vec![0.0, 1.0, 0.3, 0.8];
        assert_eq!(SolutionExtractor::extract_binary(&solution, 0), Some(false));
        assert_eq!(SolutionExtractor::extract_binary(&solution, 1), Some(true));
        assert_eq!(SolutionExtractor::extract_binary(&solution, 2), Some(false));
        assert_eq!(SolutionExtractor::extract_binary(&solution, 3), Some(true));
    }

    #[test]
    fn variable_array1_register_and_extract() {
        let mut model = MetaModel::<f64>::new("test_model");

        // 测试连续变量注册
        let mut array: VariableArray1<String, ContinuousVariableItem> = VariableArray1::new("x");
        let keys = vec!["layer_0".to_string(), "layer_1".to_string()];
        array.register_continuous(&keys, &mut model).unwrap();

        assert!(array.index(&"layer_0".to_string()).is_some());
        assert!(array.index(&"layer_1".to_string()).is_some());
        assert!(array.index(&"layer_2".to_string()).is_none());
        assert_eq!(array.len(), 2);

        // 测试二值变量注册
        let mut bin_array: VariableArray1<String, BinaryVariableItem> = VariableArray1::new("u");
        bin_array.register_binary(&keys, &mut model).unwrap();
        assert_eq!(bin_array.len(), 2);
    }
}
