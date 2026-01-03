#[cfg(test)]
mod test {
    use crate::DiagnosticCode;

    #[test]
    fn test_list_vindex_duplicate_non_primary_field() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::DuplicateIndexValue,
            r#"
            ---@class Item: Bean
            ---@field id int
            ---@field name string

            ---@type list<[v.index("name")] Item>
            local items = {
                { id = 1, name = "A" },
                { id = 2, name = "A" },
            }
            "#,
        ));
    }

    #[test]
    fn test_list_vindex_ok() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(ws.check_code_for(
            DiagnosticCode::DuplicateIndexValue,
            r#"
            ---@class Item: Bean
            ---@field id int
            ---@field name string

            ---@type list<[v.index("name")] Item>
            local items = {
                { id = 1, name = "A" },
                { id = 2, name = "B" },
            }
            "#,
        ));
    }

    #[test]
    fn test_array_vindex_duplicate() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::DuplicateIndexValue,
            r#"
            ---@class Item: Bean
            ---@field id int

            ---@type array<[v.index("id")] Item>
            local items = {
                { id = 1 },
                { id = 1 },
            }
            "#,
        ));
    }

    #[test]
    fn test_set_vindex_duplicate() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::DuplicateIndexValue,
            r#"
            ---@class Item: Bean
            ---@field id int

            ---@type set<[v.index("id")] Item>
            local items = {
                { id = 1 },
                { id = 1 },
            }
            "#,
        ));
    }

    #[test]
    fn test_vindex_not_supported_on_map() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(ws.check_code_for(
            DiagnosticCode::DuplicateIndexValue,
            r#"
            ---@class Item: Bean
            ---@field id int

            ---@type map<int, [v.index("id")] Item>
            local items = {
                [1] = { id = 1 },
                [2] = { id = 1 },
            }
            "#,
        ));
    }

    #[test]
    fn test_vindex_requires_bean_element() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(ws.check_code_for(
            DiagnosticCode::DuplicateIndexValue,
            r#"
            ---@type list<[v.index("id")] int>
            local xs = { 1, 1 }
            "#,
        ));
    }
}
