#[cfg(test)]
mod test {
    use crate::DiagnosticCode;

    #[test]
    fn test_vsize_must_apply_to_container_type() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidSizeSignature,
            r#"
            ---@type ([v.size(1)] int)
            local x = 1
            "#,
        ));
    }

    #[test]
    fn test_vsize_must_be_type_attribute() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidSizeSignature,
            r#"
            ---@[v.size(1)]
            ---@class TestSize: Bean
            ---@field x int
            "#,
        ));
    }

    #[test]
    fn test_vsize_invalid_range_string() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidSizeSignature,
            r#"
            ---@type ([v.size("[1,10")] array<int>)
            local xs = { 1 }
            "#,
        ));
    }

    #[test]
    fn test_vsize_ok() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(ws.check_code_for(
            DiagnosticCode::InvalidSizeSignature,
            r#"
            ---@type ([v.size(1)] array<int>)
            local xs = { 1 }
            "#,
        ));
    }

    #[test]
    fn test_vsize_not_supported_on_element_type() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidSizeSignature,
            r#"
            ---@type array<[v.size(1)] int>
            local xs = { 1, 2 }
            "#,
        ));
    }
}
