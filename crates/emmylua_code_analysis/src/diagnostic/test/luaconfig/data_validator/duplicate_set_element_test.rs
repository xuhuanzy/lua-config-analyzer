#[cfg(test)]
mod test {
    use crate::DiagnosticCode;

    #[test]
    fn test_duplicate_set_value() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();

        assert!(!ws.check_code_for(
            DiagnosticCode::DuplicateSetElement,
            r#"
            ---@type set<int>
            local s = { 1, 2, 2 }
            "#,
        ));

        assert!(!ws.check_code_for(
            DiagnosticCode::DuplicateSetElement,
            r#"
            ---@type set<string>
            local s = { "a", "a" }
            "#,
        ));

        assert!(ws.check_code_for(
            DiagnosticCode::DuplicateSetElement,
            r#"
            ---@type set<string>
            local s = { "a", "b" }
            "#,
        ));
    }

    #[test]
    fn test_list_allow_duplicate() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();

        assert!(ws.check_code_for(
            DiagnosticCode::DuplicateSetElement,
            r#"
            ---@type list<int>
            local s = { 1, 1 }
            "#,
        ));
    }
}
