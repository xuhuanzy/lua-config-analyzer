#[cfg(test)]
mod test {
    use crate::DiagnosticCode;

    #[test]
    fn test_invalid_range_syntax() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidRangeSignature,
            r#"
            ---@class TestRange: Bean
            ---@[v.range("[1,10")] -- missing closing bracket
            ---@field x int
            "#,
        ));
    }

    #[test]
    fn test_invalid_range_min_max_order() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidRangeSignature,
            r#"
            ---@class TestRange: Bean
            ---@[v.range("[10,1]")] -- min > max
            ---@field x int
            "#,
        ));
    }

    #[test]
    fn test_valid_range_signature() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(ws.check_code_for(
            DiagnosticCode::InvalidRangeSignature,
            r#"
            ---@class TestRange: Bean
            ---@[v.range("[1,10)")]
            ---@field x int
            "#,
        ));
    }
}
