use std::sync::{Arc, RwLock};

use analysis_common::variable::{Variable, VariableReference, VariableSearch};
use analysis_runner::Analyser;
use common::FileInfo;
use ASTParser::{ast::{Element, Type, AST}, RunPrimitiveInfo};
use TokenIdentifier::Globals;

use crate::{find_funcs, generate_asts, get_variable, get_variable_local, load_order::{FilePreAnalysis}, GlobalBridge, LogicError, Scope, TypeDef};

pub struct AnalysisState{
    pub active_scope: Arc<Scope>,
    pub untyped: bool,
    pub file: Arc<RunPrimitiveInfo>,
    pub globals: Arc<Globals>,
}
impl AnalysisState{
    pub fn new(file: Arc<RunPrimitiveInfo>, globals: Arc<Globals>, scope: Arc<Scope>) -> Self{
        AnalysisState{
            active_scope: scope,
            untyped: false,
            file,
            globals,
        }
    }
}

pub fn analyse(analyser: &Analyser, state: Arc<RwLock<AnalysisState>>, steps: &Vec<Element<AST>>, untyped: bool){
    for step in steps{
        //AST::visit(step, context, leaf_func, leaf_func_post)
        AST::visit(step, &(state.clone(), analyser), &analyse_step_pre, &Some(analyse_step_post));
    }
}

pub fn analyse_step_pre(context: &(Arc<RwLock<AnalysisState>>, &Analyser), _: &Vec<()>, step: &Element<AST>) -> Vec<()>{
    let state = &context.0;
    let analyser = context.1;
    let scope = state.read().unwrap().active_scope.clone();
    let untyped = state.read().unwrap().untyped.clone();
    //This is passed through AST::Visitor, so we dont visit outselves anymore
    match step.value.as_ref(){
        AST::Global(name) => {
            //does function exist (func names are searched first so this should be ok to check)
            
            //Functions are already located locally (find_funcs()) and globally (Token Identifier)
            let variable = get_variable_local(scope.clone(), name);
            
            if let Some(var) = variable.as_ref(){
                if let AST::Function { name, args, returns, actions } = var.ast().value.as_ref(){
                    let mut write = state.try_write().unwrap();
                    let reference = VariableReference::new(var.clone(), step.clone(), write.file.clone() );
                    let reference = Arc::new(reference);
                    write.active_scope.references.add_arc(reference.clone());

                    let glob_var = write.globals.globals.index(name.value.as_ref()).unwrap().clone();

                    let global_map = GlobalBridge{
                        func: var.clone(),//TODO: May leak
                        global: glob_var,
                    };
                    write.active_scope.global.add(name.value.to_string(), global_map);
                } else{
                    //We have called "global function *" on a variable that is not a function
                    let mut errors = scope.errors.try_write().unwrap();
                    errors.push(Element::new(LogicError::UndefinedVariableError(name.clone()), step.range));//I should make a special error for this
                }
            } else {
                //We have called "global function *" on a function that does not exist
                let mut errors = scope.errors.try_write().unwrap();
                errors.push(Element::new(LogicError::UndefinedVariableError(name.clone()), step.range));//I should make a special error for this
            }
        }
        AST::Typedef {global,  name, sqtype } => {
            scope.types.add(name.value.to_string(), TypeDef{ast: step.clone(), name: name.value.to_string() });
        }
        AST::Declaration { name, vartype, value } => {
            //let mut vars = scope.vars.try_write().unwrap();
            //vars.push(Arc::new(Variable::new(step.clone())));
            scope.vars.add( name.value.to_string(), Variable::new(step.clone()) );

        }
        AST::ConstDeclaration { global, name, vartype, value } => {
            //let mut vars = scope.vars.try_write().unwrap();
            //Todo: raise error if given a none-constant value
            //TODO: This should be handled earlier than here
            //vars.push(Arc::new(Variable::new(step.clone())));
            if *global {
                let global = state.read().unwrap().globals.get(name.value.as_ref()).unwrap().clone();
                scope.vars.add_arc(name.value.to_string(), global.clone());
            } else {
                scope.vars.add(name.value.to_string(), Variable::new(step.clone()));
            }
        }
        AST::EnumDeclaration { global,  name } => {
            let name = name.value.to_string();
            scope.types.add(name.clone(), TypeDef{ast: step.clone(), name: name.clone() });
            //Bit sketch
            if *global {
                let global = state.read().unwrap().globals.get(&name);
                if let Some(global) = global{
                    scope.vars.add_arc(name.clone(), global.clone());
                } else {
                    panic!("did not run token Identifier")
                }
            } else{
                scope.vars.add(name, Variable::new(step.clone()));
            }

        }
        AST::StructDeclaration { global, name, attributes } => {
            let name = name.value.to_string();
            scope.types.add(name.clone(), TypeDef{ast: step.clone(), name: name.clone() });
        }
        AST::Assignment { var, value } => {
            //let vars = scope.vars.read().unwrap();
            //let var = get_variable(scope, state.clone(), )
            //need to make some kind of nice "is this a variable" function (Not get variable, i want to resolve the expression not just test if a given name is a variable)
            //Todo: check if var is mutable
        }
        AST::If { condition, actions } => {
            let scope = Scope::add_child(scope, step.range);
            state.try_write().unwrap().active_scope = scope.clone();
        }
        AST::While { condition, actions } => {
            let scope = Scope::add_child(scope, step.range);
            state.try_write().unwrap().active_scope = scope.clone();
        }
        AST::ForEach { iterators, iterable, actions } => {
            let scope = Scope::add_child(scope, step.range);
            state.try_write().unwrap().active_scope = scope.clone();
        }
        AST::For { init, condition, increment, actions} => {
            let scope = Scope::add_child(scope, step.range);
            drop(untyped);
            state.try_write().unwrap().active_scope = scope.clone();
        }
        AST::Switch { condition, cases, default } => {
            let scope = Scope::add_child(scope, step.range);
            drop(untyped);
            state.try_write().unwrap().active_scope = scope.clone();
        }
        AST::Case { condition, actions } => {
            let scope = Scope::add_child(scope, step.range);
            drop(untyped);
            state.try_write().unwrap().active_scope = scope.clone();
        }
        AST::Return(value) => {
            let mut has_return = scope.has_return.try_write().unwrap();
            *has_return = true;
        }
        AST::Unreachable => {
            let mut has_return = scope.has_return.try_write().unwrap();
            *has_return = true;
        }
        AST::Function { name, args, returns, actions } => {
            //let mut vars = scope.vars.try_write().unwrap();
            //vars.push(Arc::new(Variable::new(step.clone())));//Should functions be in the vars list? they basically are
            //drop(vars);
            let scope = Scope::add_child(scope.clone(), step.range);
            drop(untyped);
            state.try_write().unwrap().active_scope = scope.clone();
        }
        AST::AnonymousFunction(args, included_vars , actions) => {
            
            for var in included_vars{
                let name = var.value.to_string();
                let variable = get_variable(analyser, scope.clone(), state.clone(), &name);
                variable.for_each(|context, var| {
                    let reference = VariableReference::new(var.clone(), step.clone(), state.read().unwrap().file.clone() );
                    let reference = Arc::new(reference);
                    state.try_write().unwrap().active_scope.references.add_arc(reference.clone());
                    var.try_add_reference(reference.clone());
                });
                variable.for_missing(|context| {
                    let mut errors = scope.errors.try_write().unwrap();
                    errors.push(Element::new(LogicError::UndefinedVariableError(name.to_string()), var.range));
                });
            }
            let scope = Scope::add_child(scope, step.range);
            drop(untyped);
            state.try_write().unwrap().active_scope = scope.clone();
        }
        AST::AnonymousScope(actions) => {
            let scope = Scope::add_child(scope, step.range);
            drop(untyped);
            state.try_write().unwrap().active_scope = scope.clone();
        }
        AST::Try { actions, catch } => {
            //Dont do anything here yet
        }
        AST::Catch { actions, exception } => {
            let scope = Scope::add_child(scope, step.range);
            drop(untyped);
            state.try_write().unwrap().active_scope = scope.clone();
        }
        AST::Wait(_) => {
            //Todo, either shout if this is not a thread, or mark this as needing to be threaded
            //And complain when the function is called
        }
        AST::Member(left, right ) => {
            //Todo: Actually test if this is even a struct
        }
        AST::Expect(_, _) | AST::Cast(_, _) => {
            //We dont care about types yet
        }
        AST::Add(left, right) | AST::Sub(left, right) | AST::Mul(left, right) | AST::Div(left, right) | AST::Mod(left, right) | AST::Pow(left, right) |
        AST::Gt(left, right) | AST::Gte(left, right) | AST::Eq(left, right) | AST::Neq(left, right) | AST::Gt(left, right) | AST::Lt(left, right) | AST::Lte(left, right) |
        AST::And(left, right) | AST::Or(left, right) | AST::Xor(left, right)=> {
            //Todo: this doesnt test if these operations are valid
        }
        AST::Index(left, right) => {
            //todo: this doesnt test if these operations are valid
        }
        AST::FunctionCall { function, args } => {
            //Todo: test for thread stuff
        }
        AST::Variable(name) => {
            let var = get_variable(analyser, scope.clone(), state.clone(), name.value.as_ref());
            //println!("Checking variable: {:?}", var);
            var.for_each(|context, var| {
                let reference = VariableReference::new(var.clone(), step.clone(), state.read().unwrap().file.clone() );
                let reference = Arc::new(reference);
                state.try_write().unwrap().active_scope.references.add_arc(reference.clone());
                var.try_add_reference(reference.clone());
            });
            var.for_missing(|context| {
                let mut errors = scope.errors.try_write().unwrap();
                errors.push(Element::new(LogicError::UndefinedVariableError(*name.value.clone()), step.range));
            });
        }
        AST::Error(err) => {
            let mut errors = scope.errors.try_write().unwrap();
            let log_err;
            if err.get_level() == 1{
                log_err = LogicError::SyntaxWarning(err.clone())
            }
            else if err.get_level() == 2{
                log_err = LogicError::SyntaxError(err.clone())
            } else {
                log_err = LogicError::SyntaxError(err.clone())
            }
            errors.push(Element::new(log_err, step.range));
        }
        _ => {}
    }
    return Vec::new();
}

pub fn analyse_step_post(context: &(Arc<RwLock<AnalysisState>>, &Analyser), _: &Vec<()>, elem: &Element<AST>) -> Vec<()>{
    let state = context.0.clone();
    let analyser = context.1;
    //Mostly reserved for exiting out of entered scopes
    let scope = state.read().unwrap().active_scope.clone();
    if scope.parent.is_none(){
        return Vec::new();
    }
    let parent = scope.parent.clone().unwrap().upgrade();
    let parent = parent.unwrap();//VERY DANGEROUS
    //But also like, if that happens we literally should crash
    match elem.value.as_ref(){
        AST::If { condition, actions } => {
                state.try_write().unwrap().active_scope = parent.clone();
        }
        AST::While { condition, actions } => {
                state.try_write().unwrap().active_scope = parent.clone();
        }
        AST::ForEach { iterators, iterable, actions } => {
                state.try_write().unwrap().active_scope = parent.clone();
        }
        AST::For { init, condition, increment, actions} => {
                state.try_write().unwrap().active_scope = parent.clone();
        }
        AST::Switch { condition, cases, default } => {
                state.try_write().unwrap().active_scope = parent.clone();
        }
        AST::Case { condition, actions } => {
            if condition.iter().any(|x| matches!(x.value.as_ref(), AST::Empty)) {
                //Default case
                if scope.has_return.read().unwrap().to_owned() == true{
                    //Default returns, so we assume the entire case calls return, thus the function returns
                    *parent.has_return.try_write().unwrap() = true;
                }
            }
            state.try_write().unwrap().active_scope = parent.clone();
        }
        AST::Function { name, args, returns, actions } => {
            
            if let Some(returns) = returns{
                //analyse_step(scope.clone(), step, untyped);//TODO: type checking
                if returns.value.as_ref() != &Type::Void &&  returns.value.as_ref() != &Type::Var{
                    let has_return = scope.has_return.read().unwrap();
                    if !*has_return{
                        let mut errors = scope.errors.try_write().unwrap();
                        errors.push(Element::new(LogicError::DoesNotReturnError, returns.range));
                    }
                }
            }
            state.try_write().unwrap().active_scope = parent.clone();
        }
        AST::AnonymousFunction(args, included_vars , actions) => {
            state.try_write().unwrap().active_scope = parent.clone();
        }
        AST::Catch { exception, actions } => {
            state.try_write().unwrap().active_scope = parent.clone();
        }
        AST::AnonymousScope(actions) => {
            state.try_write().unwrap().active_scope = parent.clone();
        }
        _ => {}
    }
    return Vec::new();
}

pub fn is_expression_mutable(scope: Arc<Scope>, expression: &AST) -> bool{
    todo!()
    //Pain
}
#[cfg(test)]
pub fn parse_file(file: FileInfo, id: usize){
    let asts = generate_asts(file.clone());

    for run in asts{
        let steps = run.ast.clone();
        let state = run.context;
        println!("\n\n\n{:?}\n\n\n\n", state);
        let scope = Scope::new((0, file.len()));
        find_funcs(scope.clone(), &steps);
        //let state = Arc::new(RwLock::new(AnalysisState::new(FilePreAnalysis::debugblank(), scope.clone())));
        //let untyped = state.read().unwrap().untyped.clone();
        //analyse(state.clone(), &steps, untyped);

        for err in collect_errs(scope.clone()){
            if let LogicError::UndefinedVariableError(_) = err.value.as_ref(){
                continue
            }
            println!("{:?} at \n {} \n", err, file.text()[err.range.0 .. err.range.1].to_string());
        }
        println!("ast {:?}", steps);
    }
}

pub fn collect_errs(scope: Arc<Scope>) -> Vec<Element<LogicError>>{
    let mut errors = scope.errors.read().unwrap().clone();
    for child in scope.children.get(){
        errors.append(&mut collect_errs(child.clone()));
    }
    return errors;
}