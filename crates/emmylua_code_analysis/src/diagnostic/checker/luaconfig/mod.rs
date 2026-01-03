use crate::{
    SemanticModel,
    diagnostic::checker::{DiagnosticContext, run_check},
};

mod attribute;
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
    run_check::<attribute::flags_enum_value::FlagsEnumValueChecker>(context, semantic_model);
    run_check::<attribute::vref_signature::VRefSignatureChecker>(context, semantic_model);
    run_check::<data_validator::invalid_ref::InvalidRefChecker>(context, semantic_model);
    run_check::<data_validator::duplicate_index_value::DuplicateIndexValueChecker>(
        context,
        semantic_model,
    );
    run_check::<data_validator::duplicate_set_element::DuplicateSetElementChecker>(
        context,
        semantic_model,
    );
}
