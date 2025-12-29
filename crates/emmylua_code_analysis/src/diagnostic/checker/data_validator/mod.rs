use crate::{
    SemanticModel,
    diagnostic::checker::{DiagnosticContext, run_check},
};

#[allow(unused)]
mod duplicate_primary_key;

pub fn check_data_validator(context: &mut DiagnosticContext, semantic_model: &SemanticModel) {
    run_check::<duplicate_primary_key::DuplicatePrimaryKeyChecker>(context, semantic_model);
}
