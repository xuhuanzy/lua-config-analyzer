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

    #[test]
    fn test_nested_bean_ref_value() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        ws.def(
            r#"
            ---@class Item: Bean
            ---@field id int

            ---@class TbItem: ConfigTable
            ---@field [int] Item

            ---@type TbItem
            local items = {
                { id = 1 },
                { id = 3 },
            }

            ---@class TestRef: Bean
            ---@[v.ref("TbItem")]
            ---@field x1 int
        "#,
        );

        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidRef,
            r#"
            ---@type TestRef
            local testRef = { x1 = 2, }
            "#,
        ));
    }

    #[test]
    fn test_nested_bean_ref_value_in_config_table() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        ws.def(
            r#"
            ---@class Item: Bean
            ---@field id int

            ---@class TbItem: ConfigTable
            ---@field [int] Item

            ---@type TbItem
            local items = {
                { id = 1 },
                { id = 3 },
            }

            ---@class Inner: Bean
            ---@field id int
            ---@[v.ref("TbItem")]
            ---@field itemId int

            ---@class User: Bean
            ---@field id int
            ---@field inner Inner
        "#,
        );

        assert!(ws.check_code_for(
            DiagnosticCode::InvalidRef,
            r#"
            ---@type User
            local users = { id = 1, inner = { id = 1, itemId = 3 } }
            "#,
        ));
        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidRef,
            r#"
            ---@type User
            local users = { id = 1, inner = { id = 1, itemId = 2 } }
            "#,
        ));
    }

    #[test]
    fn test_array_ref_value() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        ws.def(
            r#"
            ---@class Item: Bean
            ---@field id int

            ---@class TbItem: ConfigTable
            ---@field [int] Item

            ---@type TbItem
            local items = {
                { id = 1 },
                { id = 3 },
            }
        "#,
        );

        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidRef,
            r#"
            ---@type array<[v.ref("TbItem")] int>
            local itemIds = { 1, 2 }
            "#,
        ));

        assert!(ws.check_code_for(
            DiagnosticCode::InvalidRef,
            r#"
            ---@type array<[v.ref("TbItem")] int>
            local itemIds = { 1, 3 }
            "#,
        ));
    }

    #[test]
    fn test_list_ref_value() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        ws.def(
            r#"
            ---@class Item: Bean
            ---@field id int

            ---@class TbItem: ConfigTable
            ---@field [int] Item

            ---@type TbItem
            local items = {
                { id = 1 },
                { id = 3 },
            }
        "#,
        );

        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidRef,
            r#"
            ---@type list<[v.ref("TbItem")] int>
            local itemIds = { 1, 2 }
            "#,
        ));
    }

    #[test]
    fn test_set_ref_value() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        ws.def(
            r#"
            ---@class Item: Bean
            ---@field id int

            ---@class TbItem: ConfigTable
            ---@field [int] Item

            ---@type TbItem
            local items = {
                { id = 1 },
                { id = 3 },
            }
        "#,
        );

        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidRef,
            r#"
            ---@type set<[v.ref("TbItem")] int>
            local itemSet = { 1, 2 }
            "#,
        ));
    }

    #[test]
    fn test_map_ref_key_and_value() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        ws.def(
            r#"
            ---@class Item: Bean
            ---@field id int

            ---@class TbItem: ConfigTable
            ---@field [int] Item

            ---@type TbItem
            local items = {
                { id = 1 },
                { id = 3 },
            }
        "#,
        );

        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidRef,
            r#"
            ---@type map<[v.ref("TbItem")] int, string>
            local itemNameById = { [1] = "A", [2] = "B" }
            "#,
        ));

        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidRef,
            r#"
            ---@type map<int, [v.ref("TbItem")] int>
            local itemIdByUser = { [1] = 3, [2] = 2 }
            "#,
        ));

        assert!(ws.check_code_for(
            DiagnosticCode::InvalidRef,
            r#"
            ---@type map<int, [v.ref("TbItem")] int>
            local itemIdByUser = { [1] = 1, [2] = 3 }
            "#,
        ));
    }

    #[test]
    fn test_nested_list_list_ref_value() {
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        ws.def(
            r#"
            ---@class Item: Bean
            ---@field id int

            ---@class TbItem: ConfigTable
            ---@field [int] Item

            ---@type TbItem
            local items = {
                { id = 1 },
                { id = 3 },
            }

            ---@class TestNested: Bean
            ---@field ids list<list<[v.ref("TbItem")] int>>
        "#,
        );

        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidRef,
            r#"
            ---@type TestNested
            local t = { ids = { { 1, 2 }, { 3 } } }
            "#,
        ));

        assert!(ws.check_code_for(
            DiagnosticCode::InvalidRef,
            r#"
            ---@type TestNested
            local t = { ids = { { 1, 3 }, { 3 } } }
            "#,
        ));
    }
}
