mod index_data;
mod resolve_keys;

use crate::{
    compilation::analyzer::AnalysisPipeline, db_index::DbIndex, is_sub_type_of, profile::Profile,
};

use super::{AnalyzeContext, infer_cache_manager::InferCacheManager};

pub struct LuaConfigPipeline;

impl AnalysisPipeline for LuaConfigPipeline {
    fn analyze(db: &mut DbIndex, context: &mut AnalyzeContext) {
        let _p = Profile::cond_new("luaconfig analyze", context.tree_list.len() > 1);

        let config_table_type_id = crate::LuaTypeDeclId::new(crate::CONFIG_TABLE_TYPE_NAME);

        // 收集 ConfigTable 的主键字段
        for in_filed_tree in context.tree_list.iter() {
            let file_id = in_filed_tree.file_id;
            // 获取当前文件定义的所有类型
            let Some(type_decl_ids) = db.get_type_index().get_file_types(&file_id).cloned() else {
                continue;
            };
            for type_decl_id in type_decl_ids {
                // 检查是否是 ConfigTable 的子类型
                if is_sub_type_of(db, &type_decl_id, &config_table_type_id) {
                    resolve_keys::resolve_config_table_index(db, file_id, &type_decl_id);
                }
            }
        }

        // 收集 ConfigTable 数据索引
        let mut infer_manager = InferCacheManager::new();
        let tree_list = context.tree_list.clone();
        for in_filed_tree in tree_list.iter() {
            let file_id = in_filed_tree.file_id;
            let root = in_filed_tree.value.clone();
            index_data::index_file(db, &mut infer_manager, file_id, root);
        }
    }
}
