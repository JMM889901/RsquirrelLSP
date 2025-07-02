use std::{any::TypeId, fmt::Debug};

use preprocessor::VariantID;

use crate::Analyser;

impl Analyser {
    pub fn handle_error(&self, error: AnalysisError) {
        let mut errors = self.errors.write().unwrap();
        errors.push(error);
    }
}

#[derive(Clone)]
pub enum AnalysisError {
    TreeBuildFailed {
        variant: VariantID,
        reason: String,//TODO: make these seperate errors, or remove the other things that call this
    },
    ResultsNotFound {
        step: String,
    },
    MissingCache {
        step: String,
    },
    ResultsDowncastFailed {
        step: String,
        expected: TypeId,
        found: TypeId,
    },
    VariantLacksResult{
        variant: usize,//TODO: I do have access to the full information when i create this, so i could do that
        step: String,
    },
    StepNotFound {
        step: String,
    },
    StepDowncastFailed {
        step: String,
        expected: TypeId,
        found: TypeId,
    }
    
}

impl Debug for AnalysisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalysisError::MissingCache { step } => write!(f, "Missing cache for step: {}", step),
            AnalysisError::TreeBuildFailed { variant, reason } => write!(f, "Tree build failed for variant {}: {}", variant, reason),
            AnalysisError::ResultsNotFound { step } => write!(f, "Results not found for step: {}", step),
            AnalysisError::ResultsDowncastFailed { step, expected, found } => write!(f, "Results downcast failed for step: {}, expected: {:?}, found: {:?}, a type has been entered wrong somewhere", step, expected, found),
            AnalysisError::VariantLacksResult { variant, step } => write!(f, "Variant {} lacks result for step: {}, it may be running out-of-order or failed an earlier step", variant, step),
            AnalysisError::StepNotFound { step } => write!(f, "Step not found: {}", step),
            AnalysisError::StepDowncastFailed { step, expected, found } => write!(f, "Step downcast failed for step: {}, expected: {:?}, found: {:?}, a type has been entered wrong somewhere", step, expected, found),
        }
    }
}