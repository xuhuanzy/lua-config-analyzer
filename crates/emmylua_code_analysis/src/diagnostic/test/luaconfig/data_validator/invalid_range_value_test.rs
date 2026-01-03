#[cfg(test)]
mod test {
    use crate::DiagnosticCode;

    #[test]
    fn test_vrange_closed_interval() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidRangeValue,
            r#"
            ---@class TestRange: Bean
            ---@field x1 int
            ---@[v.range("[1,10]")]
            ---@field x2 int

            ---@type TestRange
            local t = { x1 = 1, x2 = 0 }
            "#,
        ));
    }

    #[test]
    fn test_vrange_open_close_interval() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidRangeValue,
            r#"
            ---@class TestRange: Bean
            ---@[v.range("(1,10]")]
            ---@field x int

            ---@type TestRange
            local t = { x = 1 }
            "#,
        ));

        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(ws.check_code_for(
            DiagnosticCode::InvalidRangeValue,
            r#"
            ---@class TestRange: Bean
            ---@[v.range("(1,10]")]
            ---@field x int

            ---@type TestRange
            local t = { x = 10 }
            "#,
        ));
    }

    #[test]
    fn test_vrange_infinity_interval() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidRangeValue,
            r#"
            ---@class TestRange: Bean
            ---@[v.range("[1,]")]
            ---@field x int

            ---@type TestRange
            local t = { x = 0 }
            "#,
        ));

        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(ws.check_code_for(
            DiagnosticCode::InvalidRangeValue,
            r#"
            ---@class TestRange: Bean
            ---@[v.range("[1,]")]
            ---@field x int

            ---@type TestRange
            local t = { x = 1 }
            "#,
        ));
    }

    #[test]
    fn test_vrange_exact_and_range() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidRangeValue,
            r#"
            ---@class TestRange: Bean
            ---@[v.range(10)]
            ---@field x1 int
            ---@[v.range("[1,10]")]
            ---@field x2 int

            ---@type TestRange
            local t = { x1 = 9, x2 = 11 }
            "#,
        ));

        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(ws.check_code_for(
            DiagnosticCode::InvalidRangeValue,
            r#"
            ---@class TestRange: Bean
            ---@[v.range(10)]
            ---@field x1 int
            ---@[v.range("[1,10]")]
            ---@field x2 int

            ---@type TestRange
            local t = { x1 = 10, x2 = 10 }
            "#,
        ));
    }

    #[test]
    fn test_container_element_range() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidRangeValue,
            r#"
            ---@type list<[v.range("(1,)")] int>
            local xs = { 1, 2 }
            "#,
        ));
    }

    #[test]
    fn test_map_key_range() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidRangeValue,
            r#"
            ---@type map<[v.range("[1,]")] int, int>
            local m = {
                [0] = 1,
            }
            "#,
        ));
    }
}
