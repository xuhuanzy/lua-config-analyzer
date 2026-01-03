#[cfg(test)]
mod test {
    use crate::DiagnosticCode;

    #[test]
    fn test_vsize_array_exact() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidSizeValue,
            r#"
            ---@type ([v.size(1)] array<int>)
            local xs = { 1, 2 }
            "#,
        ));
    }

    #[test]
    fn test_vsize_array_ok() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(ws.check_code_for(
            DiagnosticCode::InvalidSizeValue,
            r#"
            ---@type ([v.size(2)] array<int>)
            local xs = { 1, 2 }
            "#,
        ));
    }

    #[test]
    fn test_vsize_array_range() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidSizeValue,
            r#"
            ---@type ([v.size("[1,2]")] array<int>)
            local xs = { }
            "#,
        ));
    }

    #[test]
    fn test_vsize_map_exact() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidSizeValue,
            r#"
            ---@type ([v.size(1)] map<int, int>)
            local m = {
                [1] = 1,
                [2] = 2,
            }
            "#,
        ));
    }
}
