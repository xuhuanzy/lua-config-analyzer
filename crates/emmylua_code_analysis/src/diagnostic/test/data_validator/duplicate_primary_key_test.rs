#[cfg(test)]
mod test {
    use crate::DiagnosticCode;

    #[test]
    fn test_1() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::DuplicatePrimaryKey,
            r#"
            ---@class Item: Bean
            ---@field id int
            ---@field name string

            ---@[]
            ---@class TbItem: ConfigTable
            ---@field [int] Item

            ---@type TbItem
            local items = {
                { id = 1, name = "A" },
                { id = 2, name = "B" },
                { id = 2, name = "C" },
            }
            "#,
        ));
    }

    #[test]
    fn test_multi_key_default() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        ws.def(
            r#"
        ---@class Item: Bean
        ---@field key1 int
        ---@field key2 string

        ---@[t.index(["key1", "key2"])]
        ---@class TbItem: ConfigTable
        ---@field [int] Item
        "#,
        );
        assert!(ws.check_code_for(
            DiagnosticCode::DuplicatePrimaryKey,
            r#"
            ---@type TbItem
            local items = {
                { key1 = 1, key2 = "A" },
                { key1 = 1, key2 = "B" },
            }
            "#,
        ));
        assert!(!ws.check_code_for(
            DiagnosticCode::DuplicatePrimaryKey,
            r#"
            ---@type TbItem
            local items = {
                { key1 = 11, key2 = "A" },
                { key1 = 11, key2 = "B" },
                { key1 = 11, key2 = "A" },
            }
            "#,
        ));
    }

    #[test]
    fn test_multi_key_solo_no_cross_key_collision() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(ws.check_code_for(
            DiagnosticCode::DuplicatePrimaryKey,
            r#"
            ---@class Item: Bean
            ---@field key1 int
            ---@field key2 int

            ---@[t.index(["key1", "key2"])]
            ---@class TbItem: ConfigTable
            ---@field [int] Item

            ---@type TbItem
            local items = {
                { key1 = 1, key2 = 2 },
                { key1 = 2, key2 = 1 },
            }
            "#,
        ));
    }

    #[test]
    fn test_multi_key_solo() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        ws.def(
            r#"
        ---@class Item: Bean
        ---@field key1 int
        ---@field key2 string

        ---@[t.index(["key1", "key2"], "solo")]
        ---@class TbItem: ConfigTable
        ---@field [int] Item
        "#,
        );
        assert!(!ws.check_code_for(
            DiagnosticCode::DuplicatePrimaryKey,
            r#"
            ---@type TbItem
            local items = {
                { key1 = 1, key2 = "A" },
                { key1 = 1, key2 = "B" },
            }
            "#,
        ));
        assert!(!ws.check_code_for(
            DiagnosticCode::DuplicatePrimaryKey,
            r#"
            ---@type TbItem
            local items = {
                { key1 = 11, key2 = "A1" },
                { key1 = 11, key2 = "B1" },
            }
            "#,
        ));
        assert!(ws.check_code_for(
            DiagnosticCode::DuplicatePrimaryKey,
            r#"
            ---@type TbItem
            local items = {
                { key1 = 21, key2 = "A2" },
                { key1 = 22, key2 = "B2" },
            }
            "#,
        ));
    }

    #[test]
    fn test_multiple_definition() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        ws.def(
            r#"
        ---@class Item: Bean
        ---@field id int

        ---@class TbItem: ConfigTable
        ---@field [int] Item
        "#,
        );
        ws.def(
            r#"
        ---@type TbItem
        local items = {
            { id = 1 },
            { id = 2 },
        }
        "#,
        );
        assert!(!ws.check_code_for(
            DiagnosticCode::DuplicatePrimaryKey,
            r#"
            ---@type TbItem
            local items = {
                { id = 1 },
            }
            "#,
        ));
    }
}
