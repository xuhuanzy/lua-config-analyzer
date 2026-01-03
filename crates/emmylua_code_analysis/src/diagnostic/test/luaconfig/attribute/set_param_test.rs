#[cfg(test)]
mod test {
    use crate::DiagnosticCode;

    #[test]
    fn test_vset_param_must_be_tuple_literal() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::AttributeParamTypeMismatch,
            r#"
            ---@type ([v.set((1|2|3)[])] int)
            local x = 1
            "#,
        ));
    }

    #[test]
    fn test_vset_param_reject_empty_tuple() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::AttributeParamTypeMismatch,
            r#"
            ---@type ([v.set([])] int)
            local x = 1
            "#,
        ));
    }

    #[test]
    fn test_vset_param_reject_non_literal() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::AttributeParamTypeMismatch,
            r#"
            ---@type ([v.set([int])] int)
            local x = 1
            "#,
        ));
    }

    #[test]
    fn test_vset_param_accepts_literal_values() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(ws.check_code_for(
            DiagnosticCode::AttributeParamTypeMismatch,
            r#"
            ---@type ([v.set([1, 2, "a"])] int)
            local x = 1
            "#,
        ));
    }
}
