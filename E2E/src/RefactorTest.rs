use std::cell::RefCell;

use analysis_runner::Analyser;
use analysis_runner::AnalysisResult;
use analysis_runner::AnalysisStep;
use analysis_runner::SQDistinctVariant;
use ASTParser::ast::AST;
use ASTParser::grammar::squirrel_ast;
use ASTParser::ASTParseResult;
use ASTParser::ASTParseStep;
use ASTParser::SquirrelParse;
use ConfigAnalyser::SqFileVariant;
use super::*;





#[test]
fn bruh() {
    let mut analyser = Analyser::new();

    

    let path = "../TestSets/ValidStructure/8Global";
    let res = load_mod(PathBuf::from(path)).unwrap();
    let variants = res.scripts.iter().map(|x| {
        let file = x.clone();
        let variants = get_file_varaints(file.clone());
        let variants = variants.iter().map(|variant| {
            let sq_variant = SQDistinctVariant{
                file: file.clone(),
                state: variant.state.clone().into(),
                text: variant.text().clone(),
            };
            return sq_variant;
        }).collect::<Vec<_>>();
        return (file, variants);
    }).collect::<Vec<_>>();
    analyser.variants = variants;
    analyser.add_step::<ASTParseResult>(Box::new(ASTParseStep{}), analysis_runner::AnalysisStage::Parse);
    analyser.run_steps();

}