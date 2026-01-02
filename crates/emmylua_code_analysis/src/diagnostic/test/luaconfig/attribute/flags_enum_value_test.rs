#[cfg(test)]
mod test {
    use crate::DiagnosticCode;

    #[test]
    fn test_flags_enum_values_power_of_two() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(ws.check_code_for(
            DiagnosticCode::InvalidFlagsEnumValue,
            r#"
            ---@[flags]
            ---@enum TestA
            local TestA = {
                None = 0,
                A = 1,
                B = 2,
                C = 4,
                D = 1 << 3,
            }
            "#,
        ));
    }

    #[test]
    fn test_flags_enum_invalid_value() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidFlagsEnumValue,
            r#"
            ---@[flags]
            ---@enum TestA
            local TestA = {
                None = 0,
                A = 1,
                Bad = 3,
            }
            "#,
        ));
    }
}
