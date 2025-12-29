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
}
