#[cfg(test)]
mod test {
    use crate::DiagnosticCode;

    #[test]
    fn test_vset_must_be_type_attribute() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidSetSignature,
            r#"
            ---@class TestSet: Bean
            ---@[v.set([1, 2])]
            ---@field x int
            "#,
        ));
    }

    #[test]
    fn test_vset_supported_scalar_type() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(ws.check_code_for(
            DiagnosticCode::InvalidSetSignature,
            r#"
            ---@type ([v.set([1, 2])] int)
            local x = 1
            "#,
        ));
    }

    #[test]
    fn test_vset_does_not_check_target_type() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(ws.check_code_for(
            DiagnosticCode::InvalidSetSignature,
            r#"
            ---@type ([v.set([1, 2])] boolean)
            local x = true
            "#,
        ));
    }

    #[test]
    fn test_vset_supported_in_container_element_type() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(ws.check_code_for(
            DiagnosticCode::InvalidSetSignature,
            r#"
            ---@type list<[v.set([1, 2])] int>
            local xs = { 1, 2 }
            "#,
        ));
    }
}
