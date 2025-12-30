use std::collections::{HashMap, HashSet};

use crate::{
    ConfigTablePkOccurrence, DiagnosticCode, LuaMemberKey, LuaType, LuaTypeDeclId, RenderLevel,
    SemanticModel,
    diagnostic::checker::{Checker, DiagnosticContext},
    humanize_type,
};

pub struct DuplicatePrimaryKeyChecker;

impl Checker for DuplicatePrimaryKeyChecker {
    const CODES: &[DiagnosticCode] = &[DiagnosticCode::DuplicatePrimaryKey];

    fn check(context: &mut DiagnosticContext, semantic_model: &SemanticModel) {
        let file_id = semantic_model.get_file_id();
        let db = semantic_model.get_db();
        let Some(occurrences) = db
            .get_config_index()
            .get_config_table_pk_occurrences(&file_id)
        else {
            return;
        };

        // 当前文件中所有的 ConfigTable
        let mut relevant_tables: HashSet<LuaTypeDeclId> = HashSet::new();
        for occ in occurrences.iter() {
            relevant_tables.insert(occ.get_config_table().clone());
        }

        let mut solo_counts: HashMap<(LuaTypeDeclId, LuaMemberKey, LuaType), u32> = HashMap::new();
        let mut union_counts: HashMap<(LuaTypeDeclId, Vec<LuaType>), u32> = HashMap::new();

        // 遍历所有 ConfigTable 的索引键
        for occ in db.get_config_index().iter_config_table_pk_occurrences() {
            let config_table = occ.get_config_table();
            if !relevant_tables.contains(config_table) {
                continue;
            }

            match occ {
                ConfigTablePkOccurrence::Solo {
                    config_table,
                    key,
                    value,
                    ..
                } => {
                    *solo_counts
                        .entry((config_table.clone(), key.clone(), value.clone()))
                        .or_default() += 1;
                }
                ConfigTablePkOccurrence::Union {
                    config_table,
                    values,
                    ..
                } => {
                    *union_counts
                        .entry((config_table.clone(), values.clone()))
                        .or_default() += 1;
                }
            }
        }

        for occ in occurrences {
            match occ {
                ConfigTablePkOccurrence::Solo {
                    config_table,
                    key,
                    value,
                    range,
                } => {
                    let count = solo_counts
                        .get(&(config_table.clone(), key.clone(), value.clone()))
                        .copied()
                        .unwrap_or(0);
                    if count <= 1 {
                        continue;
                    }

                    let keys_len = db
                        .get_config_index()
                        .get_config_table_keys(config_table)
                        .map(|k| k.keys().len())
                        .unwrap_or(1);

                    let name = if keys_len > 1 {
                        format!(
                            "{}={}",
                            key.to_path(),
                            humanize_type(db, value, RenderLevel::Simple)
                        )
                    } else {
                        humanize_type(db, value, RenderLevel::Simple)
                    };

                    context.add_diagnostic(
                        DiagnosticCode::DuplicatePrimaryKey,
                        *range,
                        t!("Duplicate primary key `%{name}`", name = name).to_string(),
                        None,
                    );
                }
                ConfigTablePkOccurrence::Union {
                    config_table,
                    values,
                    ranges,
                } => {
                    let count = union_counts
                        .get(&(config_table.clone(), values.clone()))
                        .copied()
                        .unwrap_or(0);
                    if count <= 1 {
                        continue;
                    }

                    let Some(index_keys) =
                        db.get_config_index().get_config_table_keys(config_table)
                    else {
                        continue;
                    };

                    let mut name = String::new();
                    name.push('[');
                    for (idx, (key, value)) in
                        index_keys.keys().iter().zip(values.iter()).enumerate()
                    {
                        if idx > 0 {
                            name.push_str(", ");
                        }
                        name.push_str(&key.to_path());
                        name.push('=');
                        name.push_str(&humanize_type(db, value, RenderLevel::Simple));
                    }
                    name.push(']');

                    for range in ranges {
                        context.add_diagnostic(
                            DiagnosticCode::DuplicatePrimaryKey,
                            *range,
                            t!("Duplicate primary key `%{name}`", name = name).to_string(),
                            None,
                        );
                    }
                }
            }
        }
    }
}
