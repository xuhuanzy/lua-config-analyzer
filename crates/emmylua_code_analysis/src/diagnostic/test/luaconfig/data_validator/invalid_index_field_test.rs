#[cfg(test)]
mod test {
    use crate::DiagnosticCode;

    #[test]
    fn test_valid_index_field() {
        // 所有字段都存在，不应有诊断
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(ws.check_code_for(
            DiagnosticCode::InvalidIndexField,
            r#"
            ---@class Item: Bean
            ---@field id int
            ---@field name string

            ---@[t.index("id")]
            ---@class TbItem: ConfigTable
            ---@field [int] Item
            "#,
        ));
    }

    #[test]
    fn test_invalid_single_index_field() {
        // 单个不存在的字段
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidIndexField,
            r#"
            ---@class Item: Bean
            ---@field id int
            ---@field name string

            ---@[t.index("nonexistent")]
            ---@class TbItem: ConfigTable
            ---@field [int] Item
            "#,
        ));
    }

    #[test]
    fn test_invalid_multi_index_fields() {
        // 多个字段中部分不存在
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(!ws.check_code_for(
            DiagnosticCode::InvalidIndexField,
            r#"
            ---@class Item: Bean
            ---@field id int
            ---@field name string

            ---@[t.index(["id", "bad_field"])]
            ---@class TbItem: ConfigTable
            ---@field [int] Item
            "#,
        ));
    }

    #[test]
    fn test_non_config_table_not_checked() {
        // 非 ConfigTable 类型不应被检查
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(ws.check_code_for(
            DiagnosticCode::InvalidIndexField,
            r#"
            ---@class NormalClass
            ---@field id int

            ---@[t.index("nonexistent")]
            ---@class NotConfigTable
            ---@field [int] NormalClass
            "#,
        ));
    }

    #[test]
    fn test_valid_multi_index_fields() {
        // 所有多字段都存在
        let mut ws = crate::VirtualWorkspace::new_with_init_std_lib();
        assert!(ws.check_code_for(
            DiagnosticCode::InvalidIndexField,
            r#"
            ---@class Item: Bean
            ---@field key1 int
            ---@field key2 string

            ---@[t.index(["key1", "key2"])]
            ---@class TbItem: ConfigTable
            ---@field [int] Item
            "#,
        ));
    }
}
