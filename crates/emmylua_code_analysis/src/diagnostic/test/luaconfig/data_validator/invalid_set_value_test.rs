#[cfg(test)]
mod test {
    use crate::DiagnosticCode;

    #[test]
    fn test_vset_on_bean_field() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidSetValue,
            r#"
            ---@class TestSet: Bean
            ---@field x ([v.set([1, 2])] int)

            ---@type TestSet
            local t = { x = 3 }
            "#,
        ));
    }

    #[test]
    fn test_container_element_set() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidSetValue,
            r#"
            ---@type list<[v.set([1, 2])] int>
            local xs = { 1, 3 }
            "#,
        ));
    }

    #[test]
    fn test_map_key_set() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidSetValue,
            r#"
            ---@type map<[v.set([1, 2])] int, int>
            local m = {
                [3] = 1,
            }
            "#,
        ));
    }

    #[test]
    fn test_map_value_set() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidSetValue,
            r#"
            ---@type map<int, [v.set(["a", "b"])] string>
            local m = {
                [1] = "c",
            }
            "#,
        ));
    }

    #[test]
    fn test_enum_set_value() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidSetValue,
            r#"
            ---@enum Status
            local Status = {
                A = 1,
                B = 2,
            }

            ---@class TestEnum: Bean
            ---@field s ([v.set([1])] Status)

            ---@type TestEnum
            local t = { s = 2 }
            "#,
        ));
    }
}
