use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;

use runner::step::AnalysisStep;
use sqparse::ast::{FunctionDefinition, FunctionDefinitionStatement, Program, VarDefinition, VarDefinitionStatement};
use sqparse::ast::StatementType::*;
pub struct FilescopeReferenceAnalysis {
}
impl AnalysisStep<Arc<IdentifiedFilescopeVariables>> for FilescopeReferenceAnalysis {
    fn analyse(&self, context: &runner::AnalysisContext) -> Result<Arc<IdentifiedFilescopeVariables>, runner::analysis_error::AnalysisError> {
        todo!()
    }
}

pub struct IdentifiedFilescopeVariables {

}

pub struct Variable{
    pub declaration: VarDefinitionStatement<'static>,
}

pub struct Function{
    pub declaration: FunctionDefinitionStatement<'static>,
}
//I just dont care anymore, im going to clone everything
pub fn parse_program(program: &Program<'static>) {
    let mut variables = HashMap::new();
    let mut functions = HashMap::new();
    for statement in &program.statements {
        match &statement.ty {
            VarDefinition(definition) => {
                for (def, a) in &definition.definitions.items{//Truly, incredible variable names right here
                    let name = def.name.value.to_string();
                    variables.insert(name, Variable {
                        declaration: definition.clone(),
                    });
                }
            }
            FunctionDefinition(definition) => {
                for (func, t) in &definition.name.items{
                    let name = func.value.to_string();
                    functions.insert(name, Function {
                        declaration: definition.clone(),
                    });
                }
            }
            _ => {}
        }
    }
}