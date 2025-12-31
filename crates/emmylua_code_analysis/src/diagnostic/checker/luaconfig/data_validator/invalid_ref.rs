use std::collections::{HashMap, HashSet};

use emmylua_parser::{LuaAstNode, LuaExpr, LuaIndexKey, LuaTableExpr};
use internment::ArcIntern;
use rowan::TextRange;

use super::super::attribute::vref_signature::parse_vref_signature;

use crate::{
    ConfigTablePkOccurrence, DiagnosticCode, LuaMemberKey, LuaMemberOwner, LuaSemanticDeclId,
    LuaType, LuaTypeDeclId, RenderLevel, SemanticModel,
    attributes::VRefAttribute,
    diagnostic::checker::{Checker, DiagnosticContext},
    humanize_type, infer_expr, infer_table_should_be,
    semantic::shared::luaconfig::BEAN,
};

pub struct InvalidRefChecker;

impl Checker for InvalidRefChecker {
    const CODES: &[DiagnosticCode] = &[DiagnosticCode::InvalidRef];

    fn check(context: &mut DiagnosticContext, semantic_model: &SemanticModel) {
        let file_id = semantic_model.get_file_id();
        let db = semantic_model.get_db();
        let root = semantic_model.get_root().clone();

        // 扫描所有表以收集 Bean 表 / 容器表
        // TODO: 关于 ref 的目标我们跳过了 Singleton ConfigTable, 因为在 lua 中我们完全可以通过 require 单例来解决
        let mut infer_cache = semantic_model.get_cache().borrow_mut();

        let mut beans_to_check: HashSet<LuaTypeDeclId> = HashSet::new();
        let mut bean_tables: Vec<(LuaTableExpr, LuaTypeDeclId)> = Vec::new();
        let mut container_tables: Vec<(LuaTableExpr, ContainerRefRule)> = Vec::new();

        // 仅收集本文件实际用到的 (target_table -> target_keys)，用于过滤主键值集合构建
        let mut needed: HashMap<LuaTypeDeclId, HashSet<LuaMemberKey>> = HashMap::new();

        for table_expr in root.descendants::<LuaTableExpr>() {
            let Ok(table_should_be) =
                infer_table_should_be(db, &mut infer_cache, table_expr.clone())
            else {
                continue;
            };

            if let Some(bean_id) =
                resolve_expected_bean_id(db, file_id, &table_expr, &table_should_be)
            {
                beans_to_check.insert(bean_id.clone());
                bean_tables.push((table_expr, bean_id));
                continue;
            }

            if let Some(rule) = resolve_expected_container_rule(
                db,
                file_id,
                table_expr.get_range(),
                &table_should_be,
            ) {
                if let Some(rule) = rule.key_rule.as_ref() {
                    needed
                        .entry(rule.target_table.clone())
                        .or_default()
                        .insert(rule.target_key.clone());
                }
                if let Some(rule) = rule.value_rule.as_ref() {
                    needed
                        .entry(rule.target_table.clone())
                        .or_default()
                        .insert(rule.target_key.clone());
                }
                container_tables.push((table_expr, rule));
            }
        }

        if bean_tables.is_empty() && container_tables.is_empty() {
            return;
        }

        // Bean -> v.ref rules (只生成签名合法的规则)
        let mut bean_rules_cache: HashMap<LuaTypeDeclId, Vec<ValidatedVRefRule>> = HashMap::new();
        for bean_id in beans_to_check {
            let rules = collect_vref_rules_for_bean(db, &bean_id);
            if rules.is_empty() {
                continue;
            }
            for rule in rules.iter() {
                needed
                    .entry(rule.target_table.clone())
                    .or_default()
                    .insert(rule.target_key.clone());
            }
            bean_rules_cache.insert(bean_id, rules);
        }

        if needed.is_empty() {
            return;
        }

        let pk_sets = PkValueSets::new_filtered(db, &needed);

        // 对于不存在任何主键值的 (table,key)，只报一次并跳过值校验，避免大量噪音。
        for rules in bean_rules_cache.values_mut() {
            rules.retain(|rule| {
                if pk_sets.has_any(&rule.target_table, &rule.target_key) {
                    return true;
                }

                context.add_diagnostic(
                    DiagnosticCode::InvalidRef,
                    rule.decl_range,
                    t!(
                        "Invalid v.ref: `%{table}.%{key}` has no indexed values",
                        table = rule.target_table.get_name(),
                        key = rule.target_key.to_path()
                    )
                    .to_string(),
                    None,
                );
                false
            });
        }

        let mut reported_no_values: HashSet<(LuaTypeDeclId, LuaMemberKey)> = HashSet::new();
        let mut filtered_container_tables = Vec::new();
        for (table_expr, mut rule) in container_tables {
            if let Some(key_rule) = rule.key_rule.as_ref()
                && !pk_sets.has_any(&key_rule.target_table, &key_rule.target_key)
            {
                if reported_no_values
                    .insert((key_rule.target_table.clone(), key_rule.target_key.clone()))
                {
                    context.add_diagnostic(
                        DiagnosticCode::InvalidRef,
                        key_rule.decl_range,
                        t!(
                            "Invalid v.ref: `%{table}.%{key}` has no indexed values",
                            table = key_rule.target_table.get_name(),
                            key = key_rule.target_key.to_path()
                        )
                        .to_string(),
                        None,
                    );
                }
                rule.key_rule = None;
            }

            if let Some(value_rule) = rule.value_rule.as_ref()
                && !pk_sets.has_any(&value_rule.target_table, &value_rule.target_key)
            {
                if reported_no_values.insert((
                    value_rule.target_table.clone(),
                    value_rule.target_key.clone(),
                )) {
                    context.add_diagnostic(
                        DiagnosticCode::InvalidRef,
                        value_rule.decl_range,
                        t!(
                            "Invalid v.ref: `%{table}.%{key}` has no indexed values",
                            table = value_rule.target_table.get_name(),
                            key = value_rule.target_key.to_path()
                        )
                        .to_string(),
                        None,
                    );
                }
                rule.value_rule = None;
            }

            if rule.key_rule.is_some() || rule.value_rule.is_some() {
                filtered_container_tables.push((table_expr, rule));
            }
        }

        for (table_expr, bean_id) in bean_tables {
            let Some(rules) = bean_rules_cache.get(&bean_id) else {
                continue;
            };
            if rules.is_empty() {
                continue;
            }

            validate_bean_table_data(context, db, &mut infer_cache, &pk_sets, rules, &table_expr);
        }

        for (table_expr, rule) in filtered_container_tables {
            validate_container_table_data(
                context,
                db,
                &mut infer_cache,
                &pk_sets,
                &rule,
                &table_expr,
            );
        }
    }
}

#[derive(Debug, Clone)]
struct ValidatedVRefRule {
    decl_range: TextRange,
    source_key: LuaMemberKey,
    target_table: LuaTypeDeclId,
    target_key: LuaMemberKey,
}

#[derive(Debug, Clone)]
struct ValidatedVRefTarget {
    decl_range: TextRange,
    target_table: LuaTypeDeclId,
    target_key: LuaMemberKey,
}

#[derive(Debug, Clone, Copy)]
enum ContainerKind {
    Array,
    List,
    Set,
    Map,
}

#[derive(Debug, Clone)]
struct ContainerRefRule {
    kind: ContainerKind,
    key_rule: Option<ValidatedVRefTarget>,
    value_rule: Option<ValidatedVRefTarget>,
}

#[derive(Default)]
struct PkValueSets {
    // table -> (key -> set(values))
    values: HashMap<LuaTypeDeclId, HashMap<LuaMemberKey, HashSet<LuaType>>>,
}

impl PkValueSets {
    fn new_filtered(
        db: &crate::DbIndex,
        needed: &HashMap<LuaTypeDeclId, HashSet<LuaMemberKey>>,
    ) -> Self {
        let mut out = Self::default();

        for occ in db.get_config_index().iter_config_table_pk_occurrences() {
            match occ {
                ConfigTablePkOccurrence::Solo {
                    config_table,
                    key,
                    value,
                    ..
                } => {
                    if needed
                        .get(config_table)
                        .is_some_and(|keys| keys.contains(key))
                    {
                        out.insert_value(config_table, key, value.clone());
                    }
                }
                ConfigTablePkOccurrence::Union {
                    config_table,
                    keys,
                    values,
                    ..
                } => {
                    let Some(needed_keys) = needed.get(config_table) else {
                        continue;
                    };

                    for (key, value) in keys.iter().zip(values.iter()) {
                        if needed_keys.contains(key) {
                            out.insert_value(config_table, key, value.clone());
                        }
                    }
                }
            }
        }

        out
    }

    fn insert_value(&mut self, table: &LuaTypeDeclId, key: &LuaMemberKey, value: LuaType) {
        self.values
            .entry(table.clone())
            .or_default()
            .entry(key.clone())
            .or_default()
            .insert(value);
    }

    fn has_any(&self, table: &LuaTypeDeclId, key: &LuaMemberKey) -> bool {
        self.values
            .get(table)
            .and_then(|m| m.get(key))
            .is_some_and(|set| !set.is_empty())
    }

    fn contains(&self, table: &LuaTypeDeclId, key: &LuaMemberKey, value: &LuaType) -> bool {
        self.values
            .get(table)
            .and_then(|m| m.get(key))
            .is_some_and(|set| set.contains(value))
    }
}

fn infer_key_type_from_index_key(
    db: &crate::DbIndex,
    infer_cache: &mut crate::LuaInferCache,
    key: &LuaIndexKey,
) -> Option<LuaType> {
    let member_key = LuaMemberKey::from_index_key(db, infer_cache, key).ok()?;
    match member_key {
        LuaMemberKey::Name(name) => Some(LuaType::StringConst(ArcIntern::new(name))),
        LuaMemberKey::Integer(i) => Some(LuaType::IntegerConst(i)),
        LuaMemberKey::ExprType(typ) => Some(typ),
        LuaMemberKey::None => None,
    }
}

fn validate_bean_table_data(
    context: &mut DiagnosticContext,
    db: &crate::DbIndex,
    infer_cache: &mut crate::LuaInferCache,
    pk_sets: &PkValueSets,
    rules: &[ValidatedVRefRule],
    table: &LuaTableExpr,
) {
    let mut field_map: HashMap<LuaMemberKey, (LuaExpr, TextRange)> = HashMap::new();
    for field in table.get_fields() {
        let Some(field_key) = field.get_field_key() else {
            continue;
        };

        let Ok(member_key) = LuaMemberKey::from_index_key(db, infer_cache, &field_key) else {
            continue;
        };

        let Some(value_expr) = field.get_value_expr() else {
            continue;
        };

        field_map.insert(member_key, (value_expr, field.get_range()));
    }

    for rule in rules {
        let Some((value_expr, range)) = field_map.get(&rule.source_key) else {
            continue;
        };

        let Ok(value_typ) = infer_expr(db, infer_cache, value_expr.clone()) else {
            continue;
        };

        if !is_checkable_literal_key(&value_typ) {
            continue;
        }

        if pk_sets.contains(&rule.target_table, &rule.target_key, &value_typ) {
            continue;
        }

        let value = humanize_type(db, &value_typ, RenderLevel::Simple);
        let key_path = rule.target_key.to_path();
        let table_name = rule.target_table.get_name();

        context.add_diagnostic(
            DiagnosticCode::InvalidRef,
            *range,
            t!(
                "Invalid reference `%{value}`: not found in `%{table}.%{key}`",
                value = value,
                table = table_name,
                key = key_path
            )
            .to_string(),
            None,
        );
    }
}

fn validate_container_table_data(
    context: &mut DiagnosticContext,
    db: &crate::DbIndex,
    infer_cache: &mut crate::LuaInferCache,
    pk_sets: &PkValueSets,
    rule: &ContainerRefRule,
    table: &LuaTableExpr,
) {
    match rule.kind {
        ContainerKind::Array | ContainerKind::List | ContainerKind::Set => {
            let Some(value_rule) = rule.value_rule.as_ref() else {
                return;
            };

            for field in table.get_fields() {
                if !field.is_value_field() {
                    continue;
                }

                let Some(value_expr) = field.get_value_expr() else {
                    continue;
                };

                let Ok(value_typ) = infer_expr(db, infer_cache, value_expr.clone()) else {
                    continue;
                };

                if !is_checkable_literal_key(&value_typ) {
                    continue;
                }

                if pk_sets.contains(&value_rule.target_table, &value_rule.target_key, &value_typ) {
                    continue;
                }

                let value = humanize_type(db, &value_typ, RenderLevel::Simple);
                let key_path = value_rule.target_key.to_path();
                let table_name = value_rule.target_table.get_name();

                context.add_diagnostic(
                    DiagnosticCode::InvalidRef,
                    field.get_range(),
                    t!(
                        "Invalid reference `%{value}`: not found in `%{table}.%{key}`",
                        value = value,
                        table = table_name,
                        key = key_path
                    )
                    .to_string(),
                    None,
                );
            }
        }
        ContainerKind::Map => {
            for field in table.get_fields() {
                if !field.is_assign_field() {
                    continue;
                }

                if let Some(key_rule) = rule.key_rule.as_ref()
                    && let Some(field_key) = field.get_field_key()
                    && let Some(key_typ) =
                        infer_key_type_from_index_key(db, infer_cache, &field_key)
                {
                    if is_checkable_literal_key(&key_typ)
                        && !pk_sets.contains(&key_rule.target_table, &key_rule.target_key, &key_typ)
                    {
                        let value = humanize_type(db, &key_typ, RenderLevel::Simple);
                        let key_path = key_rule.target_key.to_path();
                        let table_name = key_rule.target_table.get_name();
                        context.add_diagnostic(
                            DiagnosticCode::InvalidRef,
                            field.get_range(),
                            t!(
                                "Invalid reference `%{value}`: not found in `%{table}.%{key}`",
                                value = value,
                                table = table_name,
                                key = key_path
                            )
                            .to_string(),
                            None,
                        );
                    }
                }

                if let Some(value_rule) = rule.value_rule.as_ref()
                    && let Some(value_expr) = field.get_value_expr()
                    && let Ok(value_typ) = infer_expr(db, infer_cache, value_expr.clone())
                {
                    if is_checkable_literal_key(&value_typ)
                        && !pk_sets.contains(
                            &value_rule.target_table,
                            &value_rule.target_key,
                            &value_typ,
                        )
                    {
                        let value = humanize_type(db, &value_typ, RenderLevel::Simple);
                        let key_path = value_rule.target_key.to_path();
                        let table_name = value_rule.target_table.get_name();
                        context.add_diagnostic(
                            DiagnosticCode::InvalidRef,
                            field.get_range(),
                            t!(
                                "Invalid reference `%{value}`: not found in `%{table}.%{key}`",
                                value = value,
                                table = table_name,
                                key = key_path
                            )
                            .to_string(),
                            None,
                        );
                    }
                }
            }
        }
    }
}

fn collect_vref_rules_for_bean(
    db: &crate::DbIndex,
    bean_id: &LuaTypeDeclId,
) -> Vec<ValidatedVRefRule> {
    let mut out = Vec::new();

    let Some(bean_members) = db
        .get_member_index()
        .get_members(&LuaMemberOwner::Type(bean_id.clone()))
    else {
        return out;
    };

    for member in bean_members {
        let LuaMemberKey::Name(name) = member.get_key() else {
            continue;
        };

        let owner_id = LuaSemanticDeclId::Member(member.get_id());
        let Some(property) = db.get_property_index().get_property(&owner_id) else {
            continue;
        };

        let Some(vref_attr) = VRefAttribute::find_in(property) else {
            continue;
        };

        let Some(table_name) = vref_attr.get_table_name() else {
            continue;
        };
        let field_name = vref_attr.get_field_name();

        let Some((target_table, target_key)) =
            validate_vref_signature(db, member.get_file_id(), table_name, field_name)
        else {
            continue;
        };

        out.push(ValidatedVRefRule {
            decl_range: member.get_range(),
            source_key: LuaMemberKey::Name(name.clone()),
            target_table,
            target_key,
        });
    }

    out
}

fn resolve_expected_bean_id(
    db: &crate::DbIndex,
    file_id: crate::FileId,
    table_expr: &LuaTableExpr,
    ty: &LuaType,
) -> Option<LuaTypeDeclId> {
    match ty {
        LuaType::Ref(type_decl_id) => {
            if BEAN.is_bean(db, type_decl_id) {
                Some(type_decl_id.clone())
            } else {
                None
            }
        }
        LuaType::Def(type_decl_id) => {
            if BEAN.is_bean(db, type_decl_id) {
                Some(type_decl_id.clone())
            } else {
                None
            }
        }
        LuaType::Generic(generic) => {
            let base_type_id = generic.get_base_type_id();
            if BEAN.is_bean(db, &base_type_id) {
                Some(base_type_id)
            } else {
                None
            }
        }
        LuaType::Union(union) => {
            let mut bean_ids: HashSet<LuaTypeDeclId> = HashSet::new();
            for inner in union.into_vec().iter() {
                if let Some(bean_id) = resolve_expected_bean_id(db, file_id, table_expr, inner) {
                    bean_ids.insert(bean_id);
                }
            }

            if bean_ids.len() == 1 {
                bean_ids.into_iter().next()
            } else {
                None
            }
        }
        LuaType::MultiLineUnion(multi) => {
            let union = multi.to_union();
            resolve_expected_bean_id(db, file_id, table_expr, &union)
        }
        LuaType::TableConst(in_file_range) => {
            // Unresolved table: expected type is itself, skip.
            match in_file_range.file_id == file_id && in_file_range.value == table_expr.get_range()
            {
                true => None,
                false => None,
            }
        }
        _ => None,
    }
}

fn resolve_expected_container_rule(
    db: &crate::DbIndex,
    file_id: crate::FileId,
    decl_range: TextRange,
    ty: &LuaType,
) -> Option<ContainerRefRule> {
    let ty = ty.strip_attributed();
    match ty {
        LuaType::Generic(generic) => {
            let base_name = generic.get_base_type_id_ref().get_name();
            let params = generic.get_params();

            match base_name {
                "array" => {
                    let element_ty = params.first()?;
                    let value_rule =
                        resolve_vref_target_from_type(db, file_id, decl_range, element_ty);
                    value_rule.map(|value_rule| ContainerRefRule {
                        kind: ContainerKind::Array,
                        key_rule: None,
                        value_rule: Some(value_rule),
                    })
                }
                "list" => {
                    let element_ty = params.first()?;
                    let value_rule =
                        resolve_vref_target_from_type(db, file_id, decl_range, element_ty);
                    value_rule.map(|value_rule| ContainerRefRule {
                        kind: ContainerKind::List,
                        key_rule: None,
                        value_rule: Some(value_rule),
                    })
                }
                "set" => {
                    let element_ty = params.first()?;
                    let value_rule =
                        resolve_vref_target_from_type(db, file_id, decl_range, element_ty);
                    value_rule.map(|value_rule| ContainerRefRule {
                        kind: ContainerKind::Set,
                        key_rule: None,
                        value_rule: Some(value_rule),
                    })
                }
                "map" => {
                    let key_ty = params.first()?;
                    let value_ty = params.get(1)?;
                    let key_rule = resolve_vref_target_from_type(db, file_id, decl_range, key_ty);
                    let value_rule =
                        resolve_vref_target_from_type(db, file_id, decl_range, value_ty);
                    if key_rule.is_none() && value_rule.is_none() {
                        None
                    } else {
                        Some(ContainerRefRule {
                            kind: ContainerKind::Map,
                            key_rule,
                            value_rule,
                        })
                    }
                }
                _ => None,
            }
        }
        LuaType::Array(array) => {
            let value_rule =
                resolve_vref_target_from_type(db, file_id, decl_range, array.get_base());
            value_rule.map(|value_rule| ContainerRefRule {
                kind: ContainerKind::Array,
                key_rule: None,
                value_rule: Some(value_rule),
            })
        }
        LuaType::TableGeneric(params) => {
            let key_ty = params.first()?;
            let value_ty = params.get(1)?;
            let key_rule = resolve_vref_target_from_type(db, file_id, decl_range, key_ty);
            let value_rule = resolve_vref_target_from_type(db, file_id, decl_range, value_ty);
            if key_rule.is_none() && value_rule.is_none() {
                None
            } else {
                Some(ContainerRefRule {
                    kind: ContainerKind::Map,
                    key_rule,
                    value_rule,
                })
            }
        }
        LuaType::Union(union) => {
            let mut found: Option<ContainerRefRule> = None;
            for inner in union.into_vec().iter() {
                let Some(rule) = resolve_expected_container_rule(db, file_id, decl_range, inner)
                else {
                    continue;
                };

                if found.is_some() {
                    return None;
                }
                found = Some(rule);
            }
            found
        }
        LuaType::MultiLineUnion(multi) => {
            let union = multi.to_union();
            resolve_expected_container_rule(db, file_id, decl_range, &union)
        }
        _ => None,
    }
}

fn resolve_vref_target_from_type(
    db: &crate::DbIndex,
    file_id: crate::FileId,
    decl_range: TextRange,
    ty: &LuaType,
) -> Option<ValidatedVRefTarget> {
    let LuaType::Attributed(attributed) = ty else {
        return None;
    };

    let vref_attr = VRefAttribute::find_in_uses(attributed.get_attributes())?;

    let table_name = vref_attr.get_table_name()?;
    let field_name = vref_attr.get_field_name();

    let (target_table, target_key) = validate_vref_signature(db, file_id, table_name, field_name)?;

    Some(ValidatedVRefTarget {
        decl_range,
        target_table,
        target_key,
    })
}

fn validate_vref_signature(
    db: &crate::DbIndex,
    file_id: crate::FileId,
    target_table_name: &str,
    target_field_name: Option<&str>,
) -> Option<(LuaTypeDeclId, LuaMemberKey)> {
    parse_vref_signature(db, file_id, target_table_name, target_field_name).ok()
}

fn is_checkable_literal_key(ty: &LuaType) -> bool {
    matches!(
        ty,
        LuaType::IntegerConst(_)
            | LuaType::DocIntegerConst(_)
            | LuaType::StringConst(_)
            | LuaType::DocStringConst(_)
            | LuaType::BooleanConst(_)
            | LuaType::DocBooleanConst(_)
            | LuaType::FloatConst(_)
    )
}
