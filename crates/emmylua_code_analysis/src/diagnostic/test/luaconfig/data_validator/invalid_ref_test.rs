#[cfg(test)]
mod test {
    use crate::DiagnosticCode;

    #[test]
    fn test_map_ref_value() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        ws.def(
            r#"
            ---@class Item: Bean
            ---@field id int
            ---@field name string

            ---@class TbItem: ConfigTable
            ---@field [int] Item

            ---@class User: Bean
            ---@field id int
            ---@[v.ref("TbItem")]
            ---@field itemId int

            ---@class TbUser: ConfigTable
            ---@field [int] User

            ---@type TbItem
            local items = {
                { id = 1, name = "A" },
                { id = 2, name = "B" },
            }
        "#,
        );
        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidRef,
            r#"
            ---@type TbUser
            local users = {
                { id = 1, itemId = 1 },
                { id = 2, itemId = 999 },
            }
            "#,
        ));
    }

    #[test]
    fn test_map_ref_field_must_be_primary_key() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidRef,
            r#"
            ---@class Item: Bean
            ---@field id int
            ---@field name string

            ---@[t.index("id")]
            ---@class TbItem: ConfigTable
            ---@field [int] Item

            ---@type TbItem
            local items = {
                { id = 1, name = "A" },
            }

            ---@class User: Bean
            ---@field id int
            ---@[v.ref("TbItem", "name")] -- map 表只能引用主键
            ---@field itemName string

            ---@class TbUser: ConfigTable
            ---@field [int] User

            ---@type TbUser
            local users = {
                { id = 1, itemName = "A" },
            }
            "#,
        ));
    }

    #[test]
    fn test_list_ref_requires_field() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidRef,
            r#"
            ---@class Item: Bean
            ---@field id int
            ---@field name string

            ---@[t.index(["id", "name"])]
            ---@class TbItem: ConfigTable
            ---@field [int] Item

            ---@type TbItem
            local items = {
                { id = 1, name = "A" },
            }

            ---@class User: Bean
            ---@field id int
            ---@[v.ref("TbItem")] -- list 表必须显式指定索引字段
            ---@field itemId int

            ---@class TbUser: ConfigTable
            ---@field [int] User

            ---@type TbUser
            local users = {
                { id = 1, itemId = 1 },
            }
            "#,
        ));
    }

    #[test]
    fn test_list_ref_value_by_index_field() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidRef,
            r#"
            ---@class Item: Bean
            ---@field id int
            ---@field name string

            ---@[t.index(["id", "name"])]
            ---@class TbItem: ConfigTable
            ---@field [int] Item

            ---@type TbItem
            local items = {
                { id = 1, name = "A" },
                { id = 2, name = "B" },
            }

            ---@class User: Bean
            ---@field id int
            ---@[v.ref("TbItem", "name")]
            ---@field itemName string

            ---@class TbUser: ConfigTable
            ---@field [int] User

            ---@type TbUser
            local users = {
                { id = 1, itemName = "A" },
                { id = 2, itemName = "Z" },
            }
            "#,
        ));
    }

    #[test]
    fn test_list_ref_ok() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(ws.check_code_for(
            DiagnosticCode::InvalidRef,
            r#"
            ---@class Item: Bean
            ---@field id int
            ---@field name string

            ---@[t.index(["id", "name"])]
            ---@class TbItem: ConfigTable
            ---@field [int] Item

            ---@type TbItem
            local items = {
                { id = 1, name = "A" },
            }

            ---@class User: Bean
            ---@field id int
            ---@[v.ref("TbItem", "name")]
            ---@field itemName string

            ---@class TbUser: ConfigTable
            ---@field [int] User

            ---@type TbUser
            local users = {
                { id = 1, itemName = "A" },
            }
            "#,
        ));
    }
}
