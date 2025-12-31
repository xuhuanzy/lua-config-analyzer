use crate::{
    SemanticModel,
    diagnostic::checker::{DiagnosticContext, run_check},
};

mod data_validator;

pub fn check_luaconfig(context: &mut DiagnosticContext, semantic_model: &SemanticModel) {
    run_check::<data_validator::duplicate_primary_key::DuplicatePrimaryKeyChecker>(
        context,
        semantic_model,
    );
    run_check::<data_validator::invalid_index_field::InvalidIndexFieldChecker>(
        context,
        semantic_model,
    );
    run_check::<data_validator::invalid_ref::InvalidRefChecker>(context, semantic_model);
}
